use alphanumeric_sort;
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::fs::{self, FileType, Metadata};
use std::path::{Path, PathBuf};

use crate::results_queue::*;
use crate::work_queue::*;

pub fn walk<P: AsRef<Path>>(path: P) -> impl Iterator<Item = DirEntry> {
  let (results_queue, results_iterator) = new_results_queue();
  let path = path.as_ref().to_owned();

  rayon::spawn(move || {
    let (work_queue, work_iterator) = new_work_queue();
    let dent = dir_entry_from_path(&path).unwrap();

    work_queue
      .push(Work::new(dent))
      .expect("Iterator owned above");

    work_iterator.par_bridge().for_each_with(
      (work_queue, results_queue),
      |(work_queue, results_queue), work| {
        process_work(work, &work_queue, &results_queue);
      },
    );
  });

  results_iterator
}

pub struct DirEntry {
  file_name: OsString,
  file_type: FileType,
  path: Option<PathBuf>,
  metadata: Option<Metadata>,
  pub(crate) index_path: Vec<usize>,
  pub(crate) remaining_content_count: usize,
}

pub(crate) struct Work {
  dent: DirEntry,
}

fn process_work(work: Work, work_queue: &WorkQueue, results_queue: &ResultsQueue) {
  let read_dir = match fs::read_dir(&work.dir_path()) {
    Ok(read_dir) => read_dir,
    Err(err) => {
      eprintln!("{}", err);
      work_queue.completed_work_item();
      return;
    }
  };

  let mut entries: Vec<_> = read_dir
    .filter_map(|entry_result| {
      let entry = match entry_result {
        Ok(entry) => entry,
        Err(err) => {
          eprintln!("{}", err);
          return None;
        }
      };

      let file_type = match entry.file_type() {
        Ok(file_type) => file_type,
        Err(err) => {
          eprintln!("{}", err);
          return None;
        }
      };

      let dir_path = if file_type.is_dir() {
        Some(entry.path())
      } else {
        None
      };

      let mut index_path = Vec::with_capacity(work.dent.index_path.len() + 1);
      index_path.extend_from_slice(&work.dent.index_path);
      let entry = DirEntry::new(entry.file_name(), file_type, dir_path, None, index_path);

      Some(entry)
    })
    .collect();

  entries.par_sort_by(|a, b| alphanumeric_sort::compare_os_str(&a.file_name(), &b.file_name()));

  entries.iter_mut().enumerate().for_each(|(i, each)| {
    each.index_path.push(i);
  });

  let mut work = work;
  work.dent.remaining_content_count = entries.len();
  if results_queue.push(work.dent).is_err() {
    work_queue.stop_now();
  }

  for each in entries {
    if each.is_dir() {
      work_queue
        .push(Work::new(each))
        .expect("read_dir called by owning iterator");
    } else {
      if results_queue.push(each).is_err() {
        work_queue.stop_now();
      }
    }
  }

  work_queue.completed_work_item();
}

fn dir_entry_from_path(path: &Path) -> Option<DirEntry> {
  if let Ok(metadata) = fs::metadata(path) {
    let file_type = metadata.file_type();
    return Some(DirEntry::new(
      path.file_name().unwrap().to_owned(),
      file_type,
      Some(path.to_path_buf()),
      Some(metadata),
      Vec::new(),
    ));
  }
  None
}

impl DirEntry {
  fn new(
    file_name: OsString,
    file_type: FileType,
    path: Option<PathBuf>,
    metadata: Option<Metadata>,
    index_path: Vec<usize>,
  ) -> DirEntry {
    DirEntry {
      file_name,
      file_type,
      path,
      metadata,
      index_path,
      remaining_content_count: 0,
    }
  }

  pub fn is_dir(&self) -> bool {
    self.file_type.is_dir()
  }

  pub fn file_type(&self) -> &FileType {
    &self.file_type
  }

  pub fn file_name(&self) -> &OsStr {
    &self.file_name
  }

  pub fn path(&self) -> Option<&Path> {
    match self.path {
      Some(ref path) => Some(path),
      None => None,
    }
  }

  pub fn metadata(&self) -> Option<&Metadata> {
    match self.metadata {
      Some(ref metadata) => Some(metadata),
      None => None,
    }
  }
}

impl PartialEq for DirEntry {
  fn eq(&self, o: &Self) -> bool {
    self.index_path.eq(&o.index_path)
  }
}

impl Eq for DirEntry {}

impl PartialOrd for DirEntry {
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.index_path.partial_cmp(&self.index_path)
  }
}

impl Ord for DirEntry {
  fn cmp(&self, o: &Self) -> Ordering {
    o.index_path.cmp(&self.index_path)
  }
}

impl Work {
  fn new(dent: DirEntry) -> Work {
    Work { dent }
  }

  fn dir_path(&self) -> &Path {
    self.dent.path().unwrap()
  }
}

impl PartialEq for Work {
  fn eq(&self, o: &Self) -> bool {
    self.dent.index_path.eq(&o.dent.index_path)
  }
}

impl Eq for Work {}

impl PartialOrd for Work {
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.dent.index_path.partial_cmp(&self.dent.index_path)
  }
}

impl Ord for Work {
  fn cmp(&self, o: &Self) -> Ordering {
    o.dent.index_path.cmp(&self.dent.index_path)
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  fn test() {
    for each in
      walk("/Users/jessegrosjean/Documents/github/walk/benches/assets/linux_checkout").into_iter()
    {
      eprintln!("{}", each.path().unwrap().display());
    }
  }

}
