use std::any::Any;
use std::io::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::vec;

use super::{DirEntry, IndexPath, Ordered};

/// Results of reading a directory returned by `client_function`.
#[derive(Debug)]
pub struct ReadDir {
  dir_entry_results: Vec<Result<DirEntry>>,
}

/// Specification use to read a directory.
#[derive(Debug)]
pub struct ReadDirSpec {
  /// The directory to read.
  pub path: PathBuf,
  /// Depth of the directory to read relative to root of walk.
  pub depth: usize,
  /// Location where
  /// [`process_entries`](struct.WalkDir.html#method.process_entries) callback
  /// function can store walk state. This is a placeholder right now. One
  /// intended use case is to store `.gitignore` state to filter entries during
  /// the walk.
  pub state: Option<Box<Any + Send + Sync>>,
}

impl ReadDir {
  pub fn new(dir_entry_results: Vec<Result<DirEntry>>) -> ReadDir {
    ReadDir { dir_entry_results }
  }

  pub fn dir_entry_results(&self) -> &Vec<Result<DirEntry>> {
    &self.dir_entry_results
  }

  pub(crate) fn ordered_children_specs(
    &self,
    index_path: &IndexPath,
  ) -> Vec<Ordered<Arc<ReadDirSpec>>> {
    self
      .dir_entry_results()
      .iter()
      .filter_map(|each| each.as_ref().ok()?.children_spec.clone())
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

impl ReadDirSpec {
  pub fn new(path: PathBuf, depth: usize, state: Option<Box<Any + Send + Sync>>) -> ReadDirSpec {
    ReadDirSpec { path, depth, state }
  }
}
