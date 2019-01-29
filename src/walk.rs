#![allow(dead_code)]

use alphanumeric_sort;
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

pub fn walk<P>(path: P) -> impl Iterator<Item = DirEntryContents>
where
  P: AsRef<Path>,
{
  let (results_queue, results_iterator) = new_sorted_results_queue();
  let path = path.as_ref().to_owned();

  rayon::spawn(move || {
    let (work_queue, work_iterator) = new_sorted_work_queue();

    let continue_walk = Arc::new(|_path: &Path, _error: Error| true);
    let filter_entries = Arc::new(|_path: &Path, _entries: &mut Vec<DirEntry>| {});
    let sort_entries = Arc::new(|_path: &Path, entries: &mut Vec<DirEntry>| {
      entries.par_sort_by(|a, b| alphanumeric_sort::compare_os_str(&a.file_name(), &b.file_name()));
    });

    let work_context = WorkContext {
      work_queue,
      results_queue,
      continue_walk: Some(continue_walk),
      filter_entries: Some(filter_entries),
      sort_entries: Some(sort_entries),
    };

    work_context
      .push_work(Work::new(path.to_path_buf(), Vec::new()))
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

pub struct DirEntryContents {
  pub path: PathBuf,
  pub index_path: Vec<usize>,
  pub contents: Vec<DirEntry>,
  pub(crate) remaining_folders_with_contents: usize,
}

pub(crate) struct Work {
  dir_index_path: Vec<usize>,
  dir_path: PathBuf,
}

#[derive(Clone)]
struct WorkContext {
  work_queue: WorkQueue,
  results_queue: ResultsQueue,
  continue_walk: Option<Arc<Fn(&Path, Error) -> bool + Send + Sync + 'static>>,
  filter_entries: Option<Arc<Fn(&Path, &mut Vec<DirEntry>) + Send + Sync + 'static>>,
  sort_entries: Option<Arc<Fn(&Path, &mut Vec<DirEntry>) + Send + Sync + 'static>>,
}

fn process_work_new(work: Work, work_context: &WorkContext) {
  let Work {
    dir_path,
    dir_index_path,
  } = work;

  let read_dir = match fs::read_dir(&dir_path) {
    Ok(read_dir) => read_dir,
    Err(err) => {
      if !work_context.continue_walk(&dir_path, err) {
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
          if !work_context.continue_walk(&dir_path, err) {
            work_context.stop_now();
          }
          return None;
        }
      };

      let file_type = match entry.file_type() {
        Ok(file_type) => file_type,
        Err(err) => {
          if !work_context.continue_walk(&entry.path(), err) {
            work_context.stop_now();
          }
          return None;
        }
      };

      Some(DirEntry::new(entry.file_name(), file_type))
    })
    .collect();

  if let Some(filter_entries) = &work_context.filter_entries {
    filter_entries(&dir_path, &mut entries);
  }

  if let Some(sort_entries) = &work_context.sort_entries {
    sort_entries(&dir_path, &mut entries);
  }

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
        Some(Work::new(work_path, work_index_path))
      } else {
        None
      }
    })
    .collect();

  let dir_entry_contents = DirEntryContents {
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

fn dir_entry_from_path(path: &Path) -> Option<DirEntry> {
  if let Ok(metadata) = fs::metadata(path) {
    let file_type = metadata.file_type();
    return Some(DirEntry::new(
      path.file_name().unwrap().to_owned(),
      file_type,
    ));
  }
  None
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

impl PartialEq for DirEntryContents {
  fn eq(&self, o: &Self) -> bool {
    self.index_path.eq(&o.index_path)
  }
}

impl Eq for DirEntryContents {}

impl PartialOrd for DirEntryContents {
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.index_path.partial_cmp(&self.index_path)
  }
}

impl Ord for DirEntryContents {
  fn cmp(&self, o: &Self) -> Ordering {
    o.index_path.cmp(&self.index_path)
  }
}

impl Work {
  fn new(dir_path: PathBuf, dir_index_path: Vec<usize>) -> Work {
    Work {
      dir_path,
      dir_index_path,
    }
  }
}

impl WorkContext {
  fn continue_walk(&self, path: &Path, error: Error) -> bool {
    (self.continue_walk.as_ref()).map_or(true, |f| f(path, error))
  }

  fn stop_now(&self) {
    self.work_queue.stop_now()
  }

  fn push_work(&self, work: Work) -> Result<(), SendError<Work>> {
    self.work_queue.push(work)
  }

  fn completed_work_item(&self) {
    self.work_queue.completed_work_item()
  }

  fn push_result(&self, result: DirEntryContents) -> Result<(), SendError<DirEntryContents>> {
    self.results_queue.push(result)
  }
}

impl PartialEq for Work {
  fn eq(&self, o: &Self) -> bool {
    self.dir_index_path.eq(&o.dir_index_path)
  }
}

impl Eq for Work {}

impl PartialOrd for Work {
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.dir_index_path.partial_cmp(&self.dir_index_path)
  }
}

impl Ord for Work {
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
      for each_entry in each_dir_contents.contents.iter() {
        each_dir_contents.path.push(each_entry.file_name());
        eprintln!("{}", each_dir_contents.path.display());
        each_dir_contents.path.pop();
      }
    }
  }

}
