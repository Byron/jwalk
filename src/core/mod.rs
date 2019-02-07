//! A flexible walk function suitable for arbitrary sorting/filtering.

mod index_path;
mod iterators;
mod ordered;
mod ordered_queue;

use lazycell::LazyCell;
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::ffi::{OsStr, OsString};
use std::fs::{self, FileType, Metadata};
use std::io::{Error, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::vec;

use index_path::*;
use iterators::*;
use ordered::*;
use ordered_queue::*;

pub use iterators::DirEntryIter;
pub use ordered_queue::OrderedQueueIter;

/// Orchestrates a parallel (optionaly) and recursive directory walk.
///
/// - `path` The path to walk
/// - `num_threads` The number of threads to use:
///     - `0` Use rayon global pool.
///     - `1` Perform walk on calling thread.
///     - `n > 1` Construct a new rayon ThreadPool to perform the walk.
/// - `client_function` The callback function used to read a single directory.
///   It is passed a ReadDirSpec and returns a ReadDirResult.
pub fn walk<P, F>(path: P, num_threads: usize, client_function: F) -> Result<DirEntryIter<F>>
where
  P: Into<PathBuf>,
  F: Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Clone + 'static,
{
  let mut path = path.into();
  let path_metadata = fs::metadata(&path)?;
  if path_metadata.file_type().is_symlink() {
    path = fs::read_link(path)?;
  }

  let read_dir_spec = Arc::new(ReadDirSpec { path });
  let ordered_read_dir_spec = Ordered::new(read_dir_spec, IndexPath::new(vec![0]), 0);

  // Single threaded walk returns here. Is run on calling thread.
  if num_threads == 1 {
    return Ok(DirEntryIter::new(ReadDirIter::Walk {
      read_dir_spec_stack: vec![ordered_read_dir_spec],
      client_function,
    }));
  }

  // Mutli-threaded walk is scheduled here. Runs on rayon
  let stop = Arc::new(AtomicBool::new(false));
  let read_dir_result_queue = new_ordered_queue(stop.clone(), Ordering::Strict);
  let (read_dir_result_queue, read_dir_result_iter) = read_dir_result_queue;

  let walk_closure = move || {
    let read_dir_spec_queue = new_ordered_queue(stop.clone(), Ordering::Relaxed);
    let (read_dir_spec_queue, read_dir_spec_iter) = read_dir_spec_queue;

    read_dir_spec_queue.push(ordered_read_dir_spec).unwrap();

    let run_context = RunContext {
      stop,
      read_dir_spec_queue,
      read_dir_result_queue,
    };

    read_dir_spec_iter.into_iter().par_bridge().for_each_with(
      (client_function, run_context),
      |(client_function, run_context), ordered_read_dir_spec| {
        par_walk_dir(client_function, ordered_read_dir_spec, run_context);
      },
    );
  };

  if num_threads > 0 {
    if let Ok(thread_pool) = ThreadPoolBuilder::new().num_threads(num_threads).build() {
      thread_pool.spawn(walk_closure);
    } else {
      rayon::spawn(walk_closure);
    }
  } else {
    rayon::spawn(walk_closure);
  }

  Ok(DirEntryIter::new(ReadDirIter::ParWalk {
    read_dir_result_iter,
  }))
}

// Walk the single directory specified by the given `read_dir_spec`. Uses the
// `client_function` to read the directory contents. Sends that ReadDirResult to
// results queue. And then schedules any generated `children_specs` to be walked
// in the future.
fn par_walk_dir<F>(
  client_function: &F,
  read_dir_spec: Ordered<Arc<ReadDirSpec>>,
  run_context: &mut RunContext,
) where
  F: Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Clone + 'static,
{
  let (read_dir_result, children_specs) = run_client_function(client_function, read_dir_spec);

  if !run_context.push_read_dir_result(read_dir_result) {
    run_context.stop();
    return;
  }

  if let Some(children_specs) = children_specs {
    for each in children_specs {
      if !run_context.schedule_read_dir_spec(each) {
        run_context.stop();
        return;
      }
    }
  }

  run_context.complete_item();
}

// Given `ordered_read_dir_spec` runs `client_function` on the `read_dir_spec`.
// Orders the resulting ReadDirResult. Gathers and orders any generated
// `child_specs`. Returns both the ReadDirResult and generated `child_specs`.
pub(crate) fn run_client_function<F>(
  client_function: &F,
  ordered_read_dir_spec: Ordered<Arc<ReadDirSpec>>,
) -> (
  Ordered<Result<ReadDir>>,
  Option<Vec<Ordered<Arc<ReadDirSpec>>>>,
)
where
  F: Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Clone + 'static,
{
  let Ordered {
    value: read_dir_spec,
    index_path,
    child_count: _,
  } = ordered_read_dir_spec;

  let read_dir_result = client_function(read_dir_spec);

  let ordered_children_specs = read_dir_result
    .as_ref()
    .ok()
    .map(|read_dir| read_dir.ordered_children_specs(&index_path));

  let ordered_read_dir_result = Ordered::new(
    read_dir_result,
    index_path,
    ordered_children_specs
      .as_ref()
      .map_or(0, |specs| specs.len()),
  );

  (ordered_read_dir_result, ordered_children_specs)
}

/// Specification use to read a directory.
///
/// - The `path` field is the directory to read.
/// - The `state` field is used to track related state by the client function.
///   For example it might be used to track `.gitignore` state and use that to
///   filter the entries when reading a directory.
pub struct ReadDirSpec {
  pub path: PathBuf,
  //pub state: Box<Any>,
}

/// Results of reading a directory returned by `client_function`.
pub struct ReadDir {
  dir_entry_results: Vec<Result<DirEntry>>,
}

/// Entries returned by when reading a directory. Each DirEntry represents an
/// entry inside of a directory on the filesystem.
pub struct DirEntry {
  depth: usize,
  file_name: OsString,
  file_type: Result<FileType>,
  metadata: LazyCell<Result<Metadata>>,
  parent_spec: Arc<ReadDirSpec>,
  children_spec: Option<Arc<ReadDirSpec>>,
  children_error: Option<Error>,
}

struct RunContext {
  stop: Arc<AtomicBool>,
  read_dir_spec_queue: OrderedQueue<Arc<ReadDirSpec>>,
  read_dir_result_queue: OrderedQueue<Result<ReadDir>>,
}

impl ReadDirSpec {
  pub fn new(path: PathBuf) -> ReadDirSpec {
    ReadDirSpec { path }
  }
}

impl ReadDir {
  pub fn new(dir_entry_results: Vec<Result<DirEntry>>) -> ReadDir {
    ReadDir { dir_entry_results }
  }

  pub fn dir_entry_results(&self) -> &Vec<Result<DirEntry>> {
    &self.dir_entry_results
  }

  fn ordered_children_specs(&self, index_path: &IndexPath) -> Vec<Ordered<Arc<ReadDirSpec>>> {
    self
      .dir_entry_results()
      .iter()
      .filter_map(|dir_entry_result| {
        if let Ok(dir_entry) = dir_entry_result {
          dir_entry.children_spec().clone()
        } else {
          None
        }
      })
      .enumerate()
      .map(|(i, spec)| Ordered::new(spec, index_path.adding(i), 0))
      .collect()
  }
}

impl IntoIterator for ReadDir {
  type Item = Result<DirEntry>;
  type IntoIter = vec::IntoIter<Result<DirEntry>>;

  fn into_iter(self) -> Self::IntoIter {
    self.dir_entry_results.into_iter()
  }
}

impl DirEntry {
  pub fn new(
    file_name: OsString,
    file_type: Result<FileType>,
    metadata: Option<Result<Metadata>>,
    parent_spec: Arc<ReadDirSpec>,
    children_spec: Option<Arc<ReadDirSpec>>,
  ) -> DirEntry {
    let metadata_cell = LazyCell::new();
    if let Some(metadata) = metadata {
      metadata_cell.fill(metadata).unwrap();
    }

    DirEntry {
      depth: 0,
      file_name,
      file_type,
      parent_spec,
      metadata: metadata_cell,
      children_spec: children_spec,
      children_error: None,
    }
  }

  pub fn depth(&self) -> usize {
    self.depth
  }

  pub fn file_name(&self) -> &OsStr {
    &self.file_name
  }

  pub fn file_type(&self) -> &Result<FileType> {
    &self.file_type
  }

  pub fn path(&self) -> PathBuf {
    let mut path = self.parent_spec.path.to_path_buf();
    path.push(&self.file_name);
    path
  }

  pub fn metadata(&self) -> &Result<Metadata> {
    if !self.metadata.filled() {
      self.metadata.fill(fs::metadata(self.path())).unwrap();
    }
    self.metadata.borrow().unwrap()
  }

  pub fn parent_spec(&self) -> &Arc<ReadDirSpec> {
    &self.parent_spec
  }

  pub fn children_spec(&self) -> &Option<Arc<ReadDirSpec>> {
    &self.children_spec
  }

  pub fn set_children_spec(&mut self, children_spec: Option<Arc<ReadDirSpec>>) {
    self.children_spec = children_spec;
  }

  pub fn children_error(&self) -> &Option<Error> {
    &self.children_error
  }

  pub fn set_children_error(&mut self, children_error: Option<Error>) {
    self.children_error = children_error;
  }
}

impl RunContext {
  fn stop(&self) {
    self.stop.store(true, AtomicOrdering::SeqCst);
  }

  fn schedule_read_dir_spec(&self, read_dir_spec: Ordered<Arc<ReadDirSpec>>) -> bool {
    !self.read_dir_spec_queue.push(read_dir_spec).is_err()
  }

  fn push_read_dir_result(&self, read_dir_result: Ordered<Result<ReadDir>>) -> bool {
    !self.read_dir_result_queue.push(read_dir_result).is_err()
  }

  fn complete_item(&self) {
    self.read_dir_spec_queue.complete_item()
  }
}

impl Clone for RunContext {
  fn clone(&self) -> Self {
    RunContext {
      stop: self.stop.clone(),
      read_dir_spec_queue: self.read_dir_spec_queue.clone(),
      read_dir_result_queue: self.read_dir_result_queue.clone(),
    }
  }
}
