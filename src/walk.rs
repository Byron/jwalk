use rayon::prelude::*;
use std::ffi::OsString;
use std::fs::FileType;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::{self, DefaultDelegate, DirListIter, ResultsQueueIter};

pub struct WalkDir {
  root: PathBuf,
}

pub struct DirEntry {
  pub file_name: OsString,
  pub file_type: FileType,
  dir_path: Arc<PathBuf>,
}

impl DirEntry {
  pub fn path(&self) -> PathBuf {
    let mut path = self.dir_path.to_path_buf();
    path.push(&self.file_name);
    path
  }
}

impl WalkDir {
  pub fn new<P: AsRef<Path>>(root: P) -> Self {
    WalkDir {
      root: root.as_ref().to_path_buf(),
    }
  }
}

impl IntoIterator for WalkDir {
  type Item = DirEntry;
  type IntoIter = WalkDirIter;

  fn into_iter(self) -> WalkDirIter {
    let mut results_queue_iter = core::walk(&self.root, None, DefaultDelegate {});

    let mut dir_list_stack = Vec::new();
    if let Some(root_dir_list) = results_queue_iter.next() {
      dir_list_stack.push(root_dir_list.into_iter());
    }

    WalkDirIter {
      results_queue_iter,
      dir_list_stack,
    }
  }
}

pub struct WalkDirIter {
  results_queue_iter: ResultsQueueIter<DefaultDelegate>,
  dir_list_stack: Vec<DirListIter<DefaultDelegate>>,
}

impl Iterator for WalkDirIter {
  type Item = DirEntry;
  fn next(&mut self) -> Option<DirEntry> {
    loop {
      if self.dir_list_stack.is_empty() {
        return None;
      }

      let dir_list_iter = self.dir_list_stack.last_mut().unwrap();
      let dir_path = dir_list_iter.path.clone();

      if let Some(dir_entry) = dir_list_iter.next() {
        if dir_entry.has_read_dir {
          let new_dir_list = self.results_queue_iter.next().unwrap();
          self.dir_list_stack.push(new_dir_list.into_iter());
        }

        let core::DirEntry {
          file_name,
          file_type,
          ..
        } = dir_entry;

        return Some(DirEntry {
          file_name,
          file_type,
          dir_path,
        });
      } else {
        self.dir_list_stack.pop();
      }
    }
  }
}

#[cfg(test)]
mod tests {

  use super::*;
  use std::thread;

  fn linux_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benches/assets/linux_checkout")
  }

  fn test_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets/test_dir")
  }

  #[test]
  fn test_walk() {
    for each in WalkDir::new(linux_dir()).into_iter() {
      println!("{}", each.path().display());
    }
  }

  #[test]
  fn test_walk_1() {
    for _ in WalkDir::new(linux_dir()).into_iter().take(1) {}
  }

}
