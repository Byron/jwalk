use lazycell::LazyCell;
use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::fs::{self, FileType, Metadata};
use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::vec;

use crate::core::{self, OrderedQueueIter, ReadDirResult, ReadDirSpec};

/// A builder to create an iterator for recursively walking a directory.
pub struct WalkDir {
  root: PathBuf,
  options: WalkDirOptions,
}

#[derive(Clone)]
struct WalkDirOptions {
  preload_metadata: bool,
}

/// A iterator for recursively descending into a directory.
pub struct WalkDirIter {
  dir_entry_list_iter: OrderedQueueIter<ReadDirResult<Ignore, DirEntry>>,
  dir_entry_iter_stack: Vec<vec::IntoIter<Result<DirEntry>>>,
}

/// A directory entry.
pub struct DirEntry {
  file_name: OsString,
  file_type: Result<FileType>,
  metadata: LazyCell<Result<Metadata>>,
  children: bool,
  children_error: Option<Error>,
  parent_spec: Arc<ReadDirSpec<Ignore>>,
}

// Placeholder for maintaining .gitignore state
#[derive(Clone)]
struct Ignore {}

impl WalkDir {
  pub fn new<P: AsRef<Path>>(root: P) -> Self {
    WalkDir {
      root: root.as_ref().to_path_buf(),
      options: WalkDirOptions {
        preload_metadata: false,
      },
    }
  }

  /// Preload metadata on background thread before returning [DirEntries](struct.DirEntry.html).
  pub fn preload_metadata(mut self, preload_metadata: bool) -> Self {
    self.options.preload_metadata = preload_metadata;
    self
  }
}

impl IntoIterator for WalkDir {
  type Item = Result<DirEntry>;
  type IntoIter = WalkDirIter;

  fn into_iter(self) -> WalkDirIter {
    let preload_metadata = self.options.preload_metadata;

    let dir_entry_list_iter = core::walk(&self.root, Ignore {}, move |read_dir_spec| {
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

          let file_type = dir_entry.file_type();
          let metadata = LazyCell::new();

          if preload_metadata {
            metadata.fill(dir_entry.metadata()).unwrap();
          }

          let children_spec = match file_type {
            Ok(file_type) => {
              if file_type.is_dir() {
                Some(ReadDirSpec::new(dir_entry.path(), Ignore {}))
              } else {
                None
              }
            }
            Err(_) => None,
          };

          Some(Ok(core::DirEntry {
            value: DirEntry {
              parent_spec: Arc::new(read_dir_spec.clone()),
              file_name: dir_entry.file_name(),
              file_type: file_type,
              metadata: metadata,
              children: children_spec.is_some(),
              children_error: None,
            },
            children_spec,
          }))
        })
        .collect();

      // Sort hardcoded right now
      dir_entries.sort_by(|a, b| match (a, b) {
        (Ok(a), Ok(b)) => a.value.file_name.cmp(&b.value.file_name),
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(_), Err(_)) => Ordering::Equal,
      });

      Ok(dir_entries)
    });

    let mut iter = WalkDirIter {
      dir_entry_list_iter,
      dir_entry_iter_stack: Vec::new(),
    };
    iter.push_next_dir_entries();
    iter
  }
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

#[cfg(test)]
mod tests {

  use super::*;

  fn test_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets/test_dir")
  }

  #[test]
  fn test_walk() {
    let paths: Vec<PathBuf> = WalkDir::new(test_dir())
      .into_iter()
      .map(|each_result| each_result.unwrap().path())
      .collect();

    println!("{:?}", paths);
  }
}
