//! A flexible walk function suitable for arbitrary sorting/filtering.

mod index_path;
mod ordered;
mod ordered_queue;

use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::io::Result;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::vec;

use index_path::*;
use ordered::*;
use ordered_queue::*;

pub use ordered_queue::OrderedQueueIter;

/// A paramaterized recursive directory walk.
///
/// - `path` The path to start walking
/// - `state` State maintained by the client function accross invocations.
/// - `num_threads` Defaults to using rayon global pool if 0 or 1. Otherwise a
///   new rayon ThreadPool with `num_threads` is created to perform the walk.
/// - `f` The client callback function. Given a ReadDirSpec and responsible for
///   reading the specified directory. Return None to cancel the walk. Return
///   ordered/filtered Vec of DirEntry on successful read. Set
///   DirEntry.children_spec on returned DirEntries to recurse the walk into
///   those entries.
pub fn walk<P, S, F, E>(
  path: P,
  state: S,
  num_threads: usize,
  f: F,
) -> OrderedQueueIter<ReadDirResult<S, E>>
where
  P: Into<PathBuf>,
  S: Send + Clone + 'static,
  F: Fn(ReadDirSpec<S>) -> ReadDirResult<S, E> + Send + Clone + 'static,
  E: Send + 'static,
{
  let path = path.into();
  let stop = Arc::new(AtomicBool::new(false));
  let dir_entry_list_queue = new_ordered_queue(stop.clone(), Ordering::Strict);
  let (dir_entry_list_queue, dir_entry_list_iter) = dir_entry_list_queue;

  let walk_closure = move || {
    let read_dir_spec = ReadDirSpec { path, state };
    let read_dir_spec_queue = new_ordered_queue(stop.clone(), Ordering::Relaxed);
    let (read_dir_spec_queue, read_dir_spec_iter) = read_dir_spec_queue;
    let ordered_read_dir_spec = Ordered::new(read_dir_spec, IndexPath::new(vec![0]), 0);

    read_dir_spec_queue.push(ordered_read_dir_spec).unwrap();

    let run_context = RunContext {
      stop,
      read_dir_spec_queue,
      dir_entry_list_queue,
    };

    read_dir_spec_iter.into_iter().par_bridge().for_each_with(
      (f, run_context),
      |(f, run_context), ordered_read_dir_spec| {
        walk_dir(f, ordered_read_dir_spec, run_context);
      },
    );
  };

  // Rayon seems to need at least 2 threads to progress
  if num_threads > 1 {
    if let Ok(thread_pool) = ThreadPoolBuilder::new().num_threads(num_threads).build() {
      thread_pool.spawn(walk_closure);
    } else {
      rayon::spawn(walk_closure);
    }
  } else {
    rayon::spawn(walk_closure);
  }

  dir_entry_list_iter
}

fn walk_dir<F, S, E>(
  f: &F,
  ordered_read_dir_spec: Ordered<ReadDirSpec<S>>,
  run_context: &mut RunContext<S, E>,
) where
  F: Fn(ReadDirSpec<S>) -> ReadDirResult<S, E> + Send + Clone + 'static,
  S: Send + Clone + 'static,
  E: Send,
{
  let Ordered {
    value: read_dir_spec,
    index_path,
    child_count: _,
  } = ordered_read_dir_spec;

  // 1. Get read_dir_result from f
  let read_dir_result = f(read_dir_spec);

  // 2. Generate ordered_children_specs from read_dir_result
  let children_specs: Option<Vec<_>> = read_dir_result.as_ref().ok().map(|dir_entries| {
    dir_entries
      .iter()
      .filter_map(|dir_entry_result| {
        if let Ok(dir_entry) = dir_entry_result {
          dir_entry.children_spec.clone()
        } else {
          None
        }
      })
      .collect()
  });

  let ordered_children_specs: Option<Vec<_>> = children_specs.map(|specs| {
    specs
      .into_iter()
      .enumerate()
      .map(|(i, spec)| Ordered::new(spec, index_path.adding(i), 0))
      .collect()
  });

  // 3. Order the read_dir_result
  let ordered_read_dir_result = Ordered::new(
    read_dir_result,
    index_path,
    ordered_children_specs
      .as_ref()
      .map_or(0, |specs| specs.len()),
  );

  // 4. Send ordered_read_dir_result to results
  if !run_context.push_read_dir_result(ordered_read_dir_result) {
    run_context.stop();
    return;
  }

  // 5. Schedule ordered_children_specs
  if let Some(ordered_children_specs) = ordered_children_specs {
    for each in ordered_children_specs {
      if !run_context.push_read_dir_spec(each) {
        run_context.stop();
        return;
      }
    }
  }

  run_context.complete_item();
}

pub trait DirEntryTrait: Send {
  fn is_skip_children(&self) -> bool;
  fn skip_children(&mut self, yes: bool);
}

pub struct WalkOptions {
  pub num_threads: usize,
}

/// A specification for a call to the client function to read DirEntries.
///
/// - The `path` field is the path who's DirEntries should be read.
/// - The `state` field is used by the client function as needed.
#[derive(Clone)]
pub struct ReadDirSpec<S> {
  pub path: PathBuf,
  pub state: S,
}

/// Results of client function.
pub type ReadDirResult<S, E> = Result<Vec<Result<DirEntry<S, E>>>>;

/// The DirEntry values created and returned by the client function.
///
/// - The `value` field is used store client entry value.
/// - The `children_spec` field is optional. If present the client function is
///    scheduled to read those DirEntries in the future.
pub struct DirEntry<S, E> {
  pub value: E,
  pub children_spec: Option<ReadDirSpec<S>>,
}

struct RunContext<S, E>
where
  S: Send,
  E: Send,
{
  stop: Arc<AtomicBool>,
  read_dir_spec_queue: OrderedQueue<ReadDirSpec<S>>,
  dir_entry_list_queue: OrderedQueue<ReadDirResult<S, E>>,
}

impl<S> ReadDirSpec<S> {
  pub fn new(path: PathBuf, state: S) -> ReadDirSpec<S> {
    ReadDirSpec { path, state }
  }
}

impl<S, E> RunContext<S, E>
where
  S: Send,
  E: Send,
{
  fn stop(&self) {
    self.stop.store(true, AtomicOrdering::SeqCst);
  }

  fn push_read_dir_spec(&self, read_dir_spec: Ordered<ReadDirSpec<S>>) -> bool {
    !self.read_dir_spec_queue.push(read_dir_spec).is_err()
  }

  fn push_read_dir_result(&self, ordered_dir_entry_list: Ordered<ReadDirResult<S, E>>) -> bool {
    !self
      .dir_entry_list_queue
      .push(ordered_dir_entry_list)
      .is_err()
  }

  fn complete_item(&self) {
    self.read_dir_spec_queue.complete_item()
  }
}

impl<S, E> Clone for RunContext<S, E>
where
  S: Send,
  E: Send,
{
  fn clone(&self) -> Self {
    RunContext {
      stop: self.stop.clone(),
      read_dir_spec_queue: self.read_dir_spec_queue.clone(),
      dir_entry_list_queue: self.dir_entry_list_queue.clone(),
    }
  }
}

#[cfg(test)]
mod tests {

  use super::*;
  use std::fs;
  use std::path::PathBuf;

  fn test_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets/test_dir")
  }

  #[test]
  fn test_walk() {
    let dir_entry_lists: Vec<_> = walk(test_dir(), 0, 0, |read_dir_spec: ReadDirSpec<usize>| {
      let mut dir_entry_results = Vec::new();
      let read_dir_spec_iter = match fs::read_dir(&read_dir_spec.path) {
        Ok(read_dir_spec_iter) => read_dir_spec_iter,
        Err(err) => return Err(err),
      };

      for each in read_dir_spec_iter {
        let each = each.unwrap();
        let file_type = each.file_type().unwrap();
        let path = each.path();
        let children_spec = if file_type.is_dir() {
          Some(ReadDirSpec::new(path.clone(), 0))
        } else {
          None
        };
        dir_entry_results.push(Ok(DirEntry {
          value: path,
          children_spec,
        }));
      }
      Ok(dir_entry_results)
    })
    .collect();

    assert!(dir_entry_lists.len() == 3);
  }
}
