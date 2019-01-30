#![allow(dead_code)]

use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::fs::{self, FileType};
use std::io::Error;
use std::path::{Path, PathBuf};
use std::sync::mpsc::SendError;
use std::sync::Arc;

use crate::results_queue::*;
use crate::work_queue::*;

pub fn walk<P>(path: P) -> impl Iterator<Item = DirEntryContents<usize>>
where
  P: AsRef<Path>,
{
  parameterized_walk(
    path,
    0,
    |state, _path, entries| {
      entries.par_sort_by(|a, b| a.file_name().cmp(b.file_name()));
      state
    },
    |_path, _error| true,
  )
}

pub fn parameterized_walk<P, S, F, H>(
  path: P,
  walk_state: S,
  process_entries: F,
  handle_error: H,
) -> impl Iterator<Item = DirEntryContents<S>>
where
  P: AsRef<Path>,
  S: Send + Clone + 'static,
  F: Fn(S, &Path, &mut Vec<DirEntry>) -> S + Send + Sync + 'static,
  H: Fn(&Path, Error) -> bool + Send + Sync + 'static,
{
  let (results_queue, results_iterator) = new_results_queue();
  let path = path.as_ref().to_owned();

  rayon::spawn(move || {
    let (work_queue, work_iterator) = new_work_queue();

    let work_context = WorkContext {
      work_queue,
      results_queue,
      handle_error: Arc::new(handle_error),
      process_entries: Arc::new(process_entries),
    };

    work_context
      .push_work(Work::new(path.to_path_buf(), Vec::new(), walk_state))
      .unwrap();

    work_iterator
      .par_bridge()
      .for_each_with(work_context, |work_context, work| {
        process_work_new(work, &work_context);
      });
  });

  results_iterator
}

pub struct DirEntry {
  file_name: OsString,
  file_type: FileType,
  skip_contents: bool,
}

pub struct DirEntryContents<S> {
  pub walk_state: S,
  pub path: PathBuf,
  pub index_path: Vec<usize>,
  pub contents: Vec<DirEntry>,
  pub(crate) remaining_folders_with_contents: usize,
}

pub(crate) struct Work<S> {
  walk_state: S,
  dir_path: PathBuf,
  dir_index_path: Vec<usize>,
}

#[derive(Clone)]
struct WorkContext<S>
where
  S: Clone,
{
  work_queue: WorkQueue<S>,
  results_queue: ResultsQueue<S>,
  handle_error: Arc<Fn(&Path, Error) -> bool + Send + Sync + 'static>,
  process_entries: Arc<Fn(S, &Path, &mut Vec<DirEntry>) -> S + Send + Sync + 'static>,
}

fn process_work_new<S>(work: Work<S>, work_context: &WorkContext<S>)
where
  S: Send + Clone,
{
  let Work {
    dir_path,
    dir_index_path,
    walk_state,
  } = work;

  let read_dir = match fs::read_dir(&dir_path) {
    Ok(read_dir) => read_dir,
    Err(err) => {
      if !work_context.handle_error(&dir_path, err) {
        work_context.stop_now();
      }
      work_context.completed_work_item();
      return;
    }
  };

  let mut entries: Vec<_> = read_dir
    .filter_map(|entry_result| {
      let entry = match entry_result {
        Ok(entry) => entry,
        Err(err) => {
          if !work_context.handle_error(&dir_path, err) {
            work_context.stop_now();
          }
          return None;
        }
      };

      let file_type = match entry.file_type() {
        Ok(file_type) => file_type,
        Err(err) => {
          if !work_context.handle_error(&entry.path(), err) {
            work_context.stop_now();
          }
          return None;
        }
      };

      Some(DirEntry::new(entry.file_name(), file_type))
    })
    .collect();

  let walk_state = (work_context.process_entries)(walk_state, &dir_path, &mut entries);

  let mut dir_index = 0;
  let generated_work: Vec<_> = entries
    .iter()
    .filter_map(|each| {
      if each.file_type().is_dir() && !each.skip_contents {
        let mut work_path = dir_path.clone();
        let mut work_index_path = dir_index_path.clone();
        work_path.push(each.file_name());
        work_index_path.push(dir_index);
        dir_index += 1;
        Some(Work::new(work_path, work_index_path, walk_state.clone()))
      } else {
        None
      }
    })
    .collect();

  let dir_entry_contents = DirEntryContents {
    walk_state: walk_state.clone(),
    path: dir_path,
    index_path: dir_index_path,
    contents: entries,
    remaining_folders_with_contents: generated_work.len(),
  };

  if work_context.push_result(dir_entry_contents).is_err() {
    work_context.stop_now();
  }

  for each in generated_work {
    if work_context.push_work(each).is_err() {
      work_context.stop_now();
      return;
    }
  }

  work_context.completed_work_item();
}

impl DirEntry {
  fn new(file_name: OsString, file_type: FileType) -> DirEntry {
    DirEntry {
      file_name,
      file_type,
      skip_contents: false,
    }
  }

  pub fn file_type(&self) -> &FileType {
    &self.file_type
  }

  pub fn file_name(&self) -> &OsStr {
    &self.file_name
  }

  pub fn skip_contents(&mut self) {
    self.skip_contents = true
  }
}

impl<S> PartialEq for DirEntryContents<S> {
  fn eq(&self, o: &Self) -> bool {
    self.index_path.eq(&o.index_path)
  }
}

impl<S> Eq for DirEntryContents<S> {}

impl<S> PartialOrd for DirEntryContents<S> {
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.index_path.partial_cmp(&self.index_path)
  }
}

impl<S> Ord for DirEntryContents<S> {
  fn cmp(&self, o: &Self) -> Ordering {
    o.index_path.cmp(&self.index_path)
  }
}

impl<S> DirEntryContents<S> {
  fn depth(&self) -> usize {
    self.index_path.len()
  }
}

impl<S> Work<S>
where
  S: Clone,
{
  fn new(dir_path: PathBuf, dir_index_path: Vec<usize>, walk_state: S) -> Work<S> {
    Work {
      dir_path,
      dir_index_path,
      walk_state,
    }
  }
}

impl<S> WorkContext<S>
where
  S: Clone,
{
  fn handle_error(&self, path: &Path, error: Error) -> bool {
    (self.handle_error)(path, error)
  }

  fn stop_now(&self) {
    self.work_queue.stop_now()
  }

  fn push_work(&self, work: Work<S>) -> Result<(), SendError<Work<S>>> {
    self.work_queue.push(work)
  }

  fn completed_work_item(&self) {
    self.work_queue.completed_work_item()
  }

  fn push_result(&self, result: DirEntryContents<S>) -> Result<(), SendError<DirEntryContents<S>>> {
    self.results_queue.push(result)
  }
}

impl<S> PartialEq for Work<S> {
  fn eq(&self, o: &Self) -> bool {
    self.dir_index_path.eq(&o.dir_index_path)
  }
}

impl<S> Eq for Work<S> {}

impl<S> PartialOrd for Work<S> {
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.dir_index_path.partial_cmp(&self.dir_index_path)
  }
}

impl<S> Ord for Work<S> {
  fn cmp(&self, o: &Self) -> Ordering {
    o.dir_index_path.cmp(&self.dir_index_path)
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  fn linux_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benches/assets/linux_checkout")
  }

  #[test]
  fn test() {
    for mut each_dir_contents in walk(linux_dir()).into_iter() {
      let indent = "  ".repeat(each_dir_contents.depth());
      println!("{}{:?}", indent, each_dir_contents.index_path);
      for each_entry in each_dir_contents.contents.iter() {
        each_dir_contents.path.push(each_entry.file_name());
        println!("{}{}", indent, each_dir_contents.path.display());
        each_dir_contents.path.pop();
      }
      println!("");
    }
  }

}
