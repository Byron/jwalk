use std::ffi::{OsStr, OsString};
use std::fs::{self, FileType, Metadata};
use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::vec;

use crate::core::{self, DirEntry, DirEntryIter, OrderedQueueIter, ReadDir, ReadDirSpec};

/// A builder to create an iterator for recursively walking a directory.
pub struct WalkDir {
  root: PathBuf,
  options: WalkDirOptions,
}

struct WalkDirOptions {
  skip_hidden: bool,
  num_threads: usize,
  preload_metadata: bool,
  process_entries_by: Option<Arc<Fn(&mut Vec<Result<DirEntry>>) + Send + Sync>>,
}

impl WalkDir {
  pub fn new<P: AsRef<Path>>(root: P) -> Self {
    WalkDir {
      root: root.as_ref().to_path_buf(),
      options: WalkDirOptions {
        num_threads: 0,
        skip_hidden: true,
        preload_metadata: false,
        process_entries_by: None,
      },
    }
  }

  /// Number of threads to use for walk. The default setting is `0`, indicating
  /// that the walk is scheduled in the default rayon ThreadPool. A value
  /// greater then `1` will start a new rayon ThreadPool configured with
  /// `num_threads` to run the walk.
  pub fn num_threads(mut self, n: usize) -> Self {
    self.options.num_threads = n;
    self
  }

  /// Enables skipping hidden entries as determined by leading `.` in file name.
  /// Enabled by default.
  pub fn skip_hidden(mut self, skip_hidden: bool) -> Self {
    self.options.skip_hidden = skip_hidden;
    self
  }

  /// Preload metadata on background thread before returning
  /// [DirEntries](struct.DirEntry.html).
  pub fn preload_metadata(mut self, preload_metadata: bool) -> Self {
    self.options.preload_metadata = preload_metadata;
    self
  }

  /// Set a function for processing directory entries. Given a mutable vec of
  /// [DirEntries](struct.DirEntry.html) you can process the vec to filter/sort
  /// those entries.
  ///
  /// You can also skip descending into directories by calling
  /// DirEntry::skip_children(true) on directories you wish to skip.
  pub fn process_entries_by<F>(mut self, process_by: F) -> Self
  where
    F: Fn(&mut Vec<Result<DirEntry>>) + Send + Sync + 'static,
  {
    self.options.process_entries_by = Some(Arc::new(process_by));
    self
  }
}

impl IntoIterator for WalkDir {
  type Item = Result<DirEntry>;
  type IntoIter = DirEntryIter;

  fn into_iter(self) -> DirEntryIter {
    let preload_metadata = self.options.preload_metadata;
    let num_threads = self.options.num_threads;
    let skip_hidden = self.options.skip_hidden;
    let process_entries_by = self.options.process_entries_by.clone();
    let dir_entry_iter = core::walk(&self.root, num_threads, move |read_dir_spec| {
      let mut dir_entry_results: Vec<_> = fs::read_dir(&read_dir_spec.path)?
        .filter_map(|dir_entry_result| {
          let dir_entry = match dir_entry_result {
            Ok(dir_entry) => dir_entry,
            Err(err) => return Some(Err(err)),
          };

          let file_name = dir_entry.file_name();
          if skip_hidden && is_hidden(&file_name) {
            return None;
          }

          let file_type = dir_entry.file_type();

          let metadata = if preload_metadata {
            Some(dir_entry.metadata())
          } else {
            None
          };

          let children_spec = match file_type {
            Ok(file_type) => {
              if file_type.is_dir() {
                let path = read_dir_spec.path.join(dir_entry.file_name());
                Some(Arc::new(ReadDirSpec::new(path)))
              } else {
                None
              }
            }
            Err(_) => None,
          };

          Some(Ok(DirEntry::new(
            file_name,
            file_type,
            metadata,
            read_dir_spec.clone(),
            children_spec,
          )))
        })
        .collect();

      process_entries_by.as_ref().map(|process_entries_by| {
        process_entries_by(&mut dir_entry_results);
      });

      Ok(ReadDir::new(dir_entry_results))
    });

    dir_entry_iter
  }
}

impl Clone for WalkDirOptions {
  fn clone(&self) -> WalkDirOptions {
    WalkDirOptions {
      num_threads: self.num_threads,
      skip_hidden: self.skip_hidden,
      preload_metadata: self.preload_metadata,
      process_entries_by: self.process_entries_by.clone(),
    }
  }
}

fn is_hidden(file_name: &OsStr) -> bool {
  file_name
    .to_str()
    .map(|s| s.starts_with("."))
    .unwrap_or(false)
}

/*
#[cfg(test)]
mod tests {

  use super::*;
  use crossbeam::channel::{self, Receiver, SendError, Sender, TryRecvError};
  use std::cmp::Ordering;

  fn test_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets/test_dir")
  }

  fn local_paths(walk_dir: WalkDir) -> Vec<String> {
    let test_dir = test_dir();
    walk_dir
      .into_iter()
      .map(|each_result| {
        let path = each_result.unwrap().path().to_path_buf();
        let path = path.strip_prefix(&test_dir).unwrap().to_path_buf();
        path.to_str().unwrap().to_string()
      })
      .collect()
  }

  #[test]
  fn test_sorted_walk() {
    assert!(
      local_paths(WalkDir::new(test_dir()).process_entries_by(|entries| {
        entries.sort_by(|a, b| match (a, b) {
          (Ok(a), Ok(b)) => a.file_name.cmp(&b.file_name),
          (Ok(_), Err(_)) => Ordering::Less,
          (Err(_), Ok(_)) => Ordering::Greater,
          (Err(_), Err(_)) => Ordering::Equal,
        });
      }))
        == vec![
          "a.txt",
          "b.txt",
          "c.txt",
          "group 1",
          "group 1/d.txt",
          "group 2",
          "group 2/e.txt",
        ]
    );
  }

  #[test]
  fn test_reverse_sorted_walk() {
    assert!(
      local_paths(WalkDir::new(test_dir()).process_entries_by(move |entries| {
        entries.sort_by(|a, b| match (b, a) {
          (Ok(a), Ok(b)) => a.file_name.cmp(&b.file_name),
          (Ok(_), Err(_)) => Ordering::Less,
          (Err(_), Ok(_)) => Ordering::Greater,
          (Err(_), Err(_)) => Ordering::Equal,
        });
      }))
        == vec![
          "group 2",
          "group 2/e.txt",
          "group 1",
          "group 1/d.txt",
          "c.txt",
          "b.txt",
          "a.txt",
        ]
    );
  }

  #[test]
  fn test_sorted_walk_with_custom_thread_pool() {
    assert!(
      local_paths(
        WalkDir::new(test_dir())
          .num_threads(2)
          .process_entries_by(|entries| {
            entries.sort_by(|a, b| match (a, b) {
              (Ok(a), Ok(b)) => a.file_name.cmp(&b.file_name),
              (Ok(_), Err(_)) => Ordering::Less,
              (Err(_), Ok(_)) => Ordering::Greater,
              (Err(_), Err(_)) => Ordering::Equal,
            });
          })
      ) == vec![
        "a.txt",
        "b.txt",
        "c.txt",
        "group 1",
        "group 1/d.txt",
        "group 2",
        "group 2/e.txt",
      ]
    );
  }

  #[test]
  fn test_skip_walking_into_directories() {
    assert!(
      local_paths(
        WalkDir::new(test_dir())
          .num_threads(2)
          .process_entries_by(|entries| {
            for each in entries.iter_mut() {
              if let Ok(each) = each {
                each.skip_children(true);
              }
            }
            entries.sort_by(|a, b| match (a, b) {
              (Ok(a), Ok(b)) => a.file_name.cmp(&b.file_name),
              (Ok(_), Err(_)) => Ordering::Less,
              (Err(_), Ok(_)) => Ordering::Greater,
              (Err(_), Err(_)) => Ordering::Equal,
            });
          })
      ) == vec!["a.txt", "b.txt", "c.txt", "group 1", "group 2",]
    );
  }

  #[test]
  fn test_see_hidden_files() {
    assert!(local_paths(
      WalkDir::new(test_dir())
        .skip_hidden(false)
        .process_entries_by(|entries| {
          entries.sort_by(|a, b| match (a, b) {
            (Ok(a), Ok(b)) => a.file_name.cmp(&b.file_name),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => Ordering::Equal,
          });
        })
    )
    .contains(&"group 2/.hidden_file.txt".to_owned()));
  }

  #[test]
  fn test_process_entries_data_into_channel() {
    let (tx, rx) = channel::unbounded();
    let _: Vec<_> = WalkDir::new(test_dir())
      .process_entries_by(move |entries| {
        tx.send(entries.len()).unwrap();
        entries.sort_by(|a, b| match (a, b) {
          (Ok(a), Ok(b)) => a.file_name.cmp(&b.file_name),
          (Ok(_), Err(_)) => Ordering::Less,
          (Err(_), Ok(_)) => Ordering::Greater,
          (Err(_), Err(_)) => Ordering::Equal,
        });
      })
      .into_iter()
      .collect();

    let resutls: Vec<_> = rx.into_iter().collect();
    assert!(resutls == vec![5, 1, 1]);
  }
}*/
