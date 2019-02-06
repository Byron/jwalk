use lazycell::LazyCell;
use std::ffi::{OsStr, OsString};
use std::fs::{self, FileType, Metadata};
use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::vec;

use crate::core::{self, DirEntryTrait, OrderedQueueIter, ReadDirResult, ReadDirSpec};

/// A builder to create an iterator for recursively walking a directory.
pub struct WalkDir {
  root: PathBuf,
  options: WalkDirOptions,
}

/// A iterator for recursively descending into a directory.
pub struct WalkDirIter {
  dir_entry_list_iter: OrderedQueueIter<ReadDirResult<ReadDirState, DirEntry>>,
  dir_entry_iter_stack: Vec<vec::IntoIter<Result<DirEntry>>>,
}

/// A directory entry.
pub struct DirEntry {
  skip_children: bool,
  file_name: OsString,
  file_type: Result<FileType>,
  metadata: LazyCell<Result<Metadata>>,
  children: bool,
  children_error: Option<Error>,
  parent_spec: Arc<ReadDirSpec<ReadDirState>>,
}

/// State associated with each ReadDirSpec
#[derive(Clone)]
struct ReadDirState {
  pub depth: usize,
  //pub ignore: Ignore,
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
  type IntoIter = WalkDirIter;

  fn into_iter(self) -> WalkDirIter {
    let preload_metadata = self.options.preload_metadata;
    let num_threads = self.options.num_threads;
    let skip_hidden = self.options.skip_hidden;
    let process_entries_by = self.options.process_entries_by.clone();
    let state = ReadDirState { depth: 0 };

    let dir_entry_list_iter = core::walk(&self.root, state, num_threads, move |read_dir_spec| {
      let state = read_dir_spec.state.clone();
      let read_dir_iter = match fs::read_dir(&read_dir_spec.path) {
        Ok(read_dir_iter) => read_dir_iter,
        Err(err) => return Err(err),
      };

      let mut dir_entries: Vec<_> = read_dir_iter
        .filter_map(|dir_entry_result| {
          let dir_entry = match dir_entry_result {
            Ok(dir_entry) => dir_entry,
            Err(err) => return Some(Err(err)),
          };

          let file_name = dir_entry.file_name();
          if skip_hidden && is_hidden(&file_name) {
            return None;
          }

          let metadata = LazyCell::new();
          if preload_metadata {
            metadata.fill(dir_entry.metadata()).unwrap();
          }

          Some(Ok(DirEntry {
            parent_spec: Arc::new(read_dir_spec.clone()),
            file_name: file_name,
            file_type: dir_entry.file_type(),
            metadata: metadata,
            skip_children: false,
            children: false,
            //children: children_spec.is_some(),
            children_error: None,
          }))
        })
        .collect();

      process_entries_by.as_ref().map(|process_entries_by| {
        process_entries_by(&mut dir_entries);
      });

      Ok(
        dir_entries
          .into_iter()
          .map(|dir_entry_result| {
            let mut dir_entry = match dir_entry_result {
              Ok(dir_entry) => dir_entry,
              Err(err) => return Err(err),
            };

            let children_spec = match dir_entry.file_type() {
              Ok(file_type) => {
                if file_type.is_dir() && !dir_entry.is_skip_children() {
                  let path = read_dir_spec.path.join(dir_entry.file_name());
                  let state = ReadDirState {
                    depth: state.depth + 1,
                  };
                  Some(ReadDirSpec::new(path, state))
                } else {
                  None
                }
              }
              Err(_) => None,
            };

            dir_entry.children = children_spec.is_some();

            Ok(core::DirEntry {
              value: dir_entry,
              children_spec,
            })
          })
          .collect(),
      )
    });

    let mut iter = WalkDirIter {
      dir_entry_list_iter,
      dir_entry_iter_stack: Vec::new(),
    };
    iter.push_next_dir_entries();
    iter
  }
}

fn is_hidden(file_name: &OsStr) -> bool {
  file_name
    .to_str()
    .map(|s| s.starts_with("."))
    .unwrap_or(false)
}

impl WalkDirIter {
  fn push_next_dir_entries(&mut self) -> Option<Error> {
    let read_dir_result = self.dir_entry_list_iter.next().unwrap().value;

    let core_read_dir_result = match read_dir_result {
      Ok(core_read_dir_result) => core_read_dir_result,
      Err(err) => return Some(err),
    };

    let dir_entry_results: Vec<_> = core_read_dir_result
      .into_iter()
      .map(|core_dir_entry_result| core_dir_entry_result.map(|core_dir_entry| core_dir_entry.value))
      .collect();

    self
      .dir_entry_iter_stack
      .push(dir_entry_results.into_iter());

    None
  }
}

impl Iterator for WalkDirIter {
  type Item = Result<DirEntry>;
  fn next(&mut self) -> Option<Self::Item> {
    loop {
      if self.dir_entry_iter_stack.is_empty() {
        return None;
      }

      let top_dir_entry_iter = self.dir_entry_iter_stack.last_mut().unwrap();

      if let Some(dir_entry_result) = top_dir_entry_iter.next() {
        let mut dir_entry = match dir_entry_result {
          Ok(dir_entry) => dir_entry,
          Err(err) => return Some(Err(err)),
        };

        if dir_entry.children {
          dir_entry.children_error = self.push_next_dir_entries();
        }

        return Some(Ok(dir_entry));
      } else {
        self.dir_entry_iter_stack.pop();
      }
    }
  }
}

impl DirEntry {
  pub fn depth(&self) -> usize {
    self.parent_spec.state.depth + 1
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
}

impl DirEntryTrait for DirEntry {
  fn is_skip_children(&self) -> bool {
    self.skip_children
  }

  fn skip_children(&mut self, yes: bool) {
    self.skip_children = yes
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
}
