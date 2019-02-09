//! A flexible walk function suitable for arbitrary sorting/filtering.

mod dir_entry;
mod index_path;
mod iterators;
mod ordered;
mod ordered_queue;
mod read_dir;

use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::io::{Error, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::vec;

use index_path::*;
use iterators::*;
use ordered::*;
use ordered_queue::*;

pub use dir_entry::DirEntry;
pub use iterators::DirEntryIter;
pub use read_dir::{ReadDir, ReadDirSpec};

/// Orchestrates a parallel (optional) and recursive directory walk.
pub(crate) fn walk<P, F>(path: P, num_threads: usize, client_function: F) -> DirEntryIter
where
  P: Into<PathBuf>,
  F: Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Sync + Clone + 'static,
{
  let path = path.into();
  let root_entry_result = DirEntry::try_from(&path);
  let ordered_read_dir_spec = root_entry_result.as_ref().ok().and_then(|root_entry| {
    if root_entry.file_type().ok()?.is_dir() {
      let read_dir_spec = Arc::new(ReadDirSpec::new(path, 0, None));
      return Some(Ordered::new(read_dir_spec, IndexPath::new(vec![0]), 0));
    }
    None
  });

  if num_threads == 1 {
    single_threaded_walk(
      ordered_read_dir_spec,
      Arc::new(client_function),
      root_entry_result,
    )
  } else {
    multi_threaded_walk(
      num_threads,
      ordered_read_dir_spec,
      Arc::new(client_function),
      root_entry_result,
    )
  }
}

/// Client's read dir function.
pub(crate) type ClientReadDirFunction =
  Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Sync + 'static;

struct RunContext {
  stop: Arc<AtomicBool>,
  read_dir_spec_queue: OrderedQueue<Arc<ReadDirSpec>>,
  read_dir_result_queue: OrderedQueue<Result<ReadDir>>,
  client_function: Arc<ClientReadDirFunction>,
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
  ordered_read_dir_spec: Option<Ordered<Arc<ReadDirSpec>>>,
  client_function: Arc<ClientReadDirFunction>,
  root_entry_result: Result<DirEntry>,
) -> DirEntryIter {
  let read_dir_spec_stack = ordered_read_dir_spec.map_or_else(|| vec![], |spec| vec![spec]);
  DirEntryIter::new(
    ReadDirIter::Walk {
      read_dir_spec_stack,
      client_function,
    },
    root_entry_result,
  )
}

fn multi_threaded_walk(
  num_threads: usize,
  ordered_read_dir_spec: Option<Ordered<Arc<ReadDirSpec>>>,
  client_function: Arc<ClientReadDirFunction>,
  root_entry_result: Result<DirEntry>,
) -> DirEntryIter {
  let stop = Arc::new(AtomicBool::new(false));
  let read_dir_result_queue = new_ordered_queue(stop.clone(), Ordering::Strict);
  let (read_dir_result_queue, read_dir_result_iter) = read_dir_result_queue;

  let walk_closure = move || {
    let ordered_read_dir_spec = match ordered_read_dir_spec {
      Some(ordered_read_dir_spec) => ordered_read_dir_spec,
      None => return,
    };

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

  DirEntryIter::new(
    ReadDirIter::ParWalk {
      read_dir_result_iter,
    },
    root_entry_result,
  )
}

fn multi_threaded_walk_dir(read_dir_spec: Ordered<Arc<ReadDirSpec>>, run_context: &mut RunContext) {
  let (read_dir_result, content_specs) =
    run_client_function(&run_context.client_function, read_dir_spec);

  if !run_context.push_read_dir_result(read_dir_result) {
    run_context.stop();
    return;
  }

  if let Some(content_specs) = content_specs {
    for each in content_specs {
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

  let ordered_content_specs = read_dir_result
    .as_ref()
    .ok()
    .map(|read_dir| read_dir.ordered_content_specs(&index_path));

  let ordered_read_dir_result = Ordered::new(
    read_dir_result,
    index_path,
    ordered_content_specs
      .as_ref()
      .map_or(0, |specs| specs.len()),
  );

  (ordered_read_dir_result, ordered_content_specs)
}
