//! A flexible walk function suitable for arbitrary sorting/filtering.

mod index_path;
mod iterators;
mod ordered;
mod ordered_queue;

use lazycell::LazyCell;
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::any::Any;
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

/// Orchestrates a parallel (optional) and recursive directory walk.
pub(crate) fn walk<P, F>(path: P, num_threads: usize, client_function: F) -> DirEntryIter
where
  P: Into<PathBuf>,
  F: Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Sync + Clone + 'static,
{
  let path = path.into();
  let read_dir_spec = Arc::new(ReadDirSpec::new(path, None));
  let ordered_read_dir_spec = Ordered::new(read_dir_spec, IndexPath::new(vec![0]), 0);

  if num_threads == 1 {
    single_threaded_walk(ordered_read_dir_spec, Arc::new(client_function))
  } else {
    multi_threaded_walk(
      num_threads,
      ordered_read_dir_spec,
      Arc::new(client_function),
    )
  }
}

/// Clients read dir function.
pub(crate) type ClientReadDirFunction =
  Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Sync + 'static;

/// Specification use to read a directory.
pub struct ReadDirSpec {
  /// The directory to read.
  pub path: PathBuf,
  /// Location where
  /// [`process_entries`](struct.WalkDir.html#method.process_entries) callback
  /// function can store walk state.
  ///
  /// One intended use case is to store `.gitignore` state to filter entries
  /// during the walk... in theory, haven't actually implemented anything using
  /// this field yet.
  pub state: Option<Box<Any + Send + Sync>>,
}

/// Results of reading a directory returned by `client_function`.
pub struct ReadDir {
  dir_entry_results: Vec<Result<DirEntry>>,
}

/// Representation of a file or directory on filesystem.
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
  client_function: Arc<ClientReadDirFunction>,
}

impl ReadDirSpec {
  pub fn new(path: PathBuf, state: Option<Box<Any + Send + Sync>>) -> ReadDirSpec {
    ReadDirSpec { path, state }
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

  /// Depth of this entry relative to the root directory where the walk started.
  pub fn depth(&self) -> usize {
    self.depth
  }

  /// File name of this entry without leading path component.
  pub fn file_name(&self) -> &OsStr {
    &self.file_name
  }

  /// File type for the file that this entry points at.
  ///
  /// This function will not traverse symlinks.
  pub fn file_type(&self) -> &Result<FileType> {
    &self.file_type
  }

  /// Full path to the file that this entry represents.
  ///
  /// The full path is created by joining the `parent_spec` path with the
  /// filename of this entry.
  pub fn path(&self) -> PathBuf {
    let mut path = self.parent_spec.path.to_path_buf();
    path.push(&self.file_name);
    path
  }

  /// Metadata for the file that this entry points at.
  ///
  /// This function will not traverse symlinks.
  pub fn metadata(&self) -> &Result<Metadata> {
    if !self.metadata.filled() {
      self.metadata.fill(fs::metadata(self.path())).unwrap();
    }
    self.metadata.borrow().unwrap()
  }

  /// [`ReadDirSpec`](struct.ReadDirSpec.html) used to read this entry.
  pub fn parent_spec(&self) -> &Arc<ReadDirSpec> {
    &self.parent_spec
  }

  /// [`ReadDirSpec`](struct.ReadDirSpec.html) used to read this entry's
  /// children.
  pub fn children_spec(&self) -> &Option<Arc<ReadDirSpec>> {
    &self.children_spec
  }

  /// Set [`ReadDirSpec`](struct.ReadDirSpec.html) used for reading this entry's
  /// children. By default this is automatically set for any entry that is a
  /// directory. The
  /// [`process_entries`](struct.WalkDir.html#method.process_entries) callback
  /// function call `entry.set_children_spec(None)` to skip descending into that
  /// directory.
  pub fn set_children_spec(&mut self, children_spec: Option<Arc<ReadDirSpec>>) {
    self.children_spec = children_spec;
  }

  /// Error generated when reading this entry's children.
  pub fn children_error(&self) -> &Option<Error> {
    &self.children_error
  }

  pub(crate) fn set_children_error(&mut self, children_error: Option<Error>) {
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
      client_function: self.client_function.clone(),
    }
  }
}

fn single_threaded_walk(
  ordered_read_dir_spec: Ordered<Arc<ReadDirSpec>>,
  client_function: Arc<ClientReadDirFunction>,
) -> DirEntryIter {
  DirEntryIter::new(ReadDirIter::Walk {
    read_dir_spec_stack: vec![ordered_read_dir_spec],
    client_function,
  })
}

fn multi_threaded_walk(
  num_threads: usize,
  ordered_read_dir_spec: Ordered<Arc<ReadDirSpec>>,
  client_function: Arc<ClientReadDirFunction>,
) -> DirEntryIter {
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
      client_function: client_function,
    };

    read_dir_spec_iter.into_iter().par_bridge().for_each_with(
      run_context,
      |run_context, ordered_read_dir_spec| {
        multi_threaded_walk_dir(ordered_read_dir_spec, run_context);
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

  DirEntryIter::new(ReadDirIter::ParWalk {
    read_dir_result_iter,
  })
}

fn multi_threaded_walk_dir(read_dir_spec: Ordered<Arc<ReadDirSpec>>, run_context: &mut RunContext) {
  let (read_dir_result, children_specs) =
    run_client_function(&run_context.client_function, read_dir_spec);

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

pub(crate) fn run_client_function(
  client_function: &Arc<ClientReadDirFunction>,
  ordered_read_dir_spec: Ordered<Arc<ReadDirSpec>>,
) -> (
  Ordered<Result<ReadDir>>,
  Option<Vec<Ordered<Arc<ReadDirSpec>>>>,
) {
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
