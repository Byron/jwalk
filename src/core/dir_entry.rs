use lazycell::LazyCell;
use std::ffi::{OsStr, OsString};
use std::fs::{self, FileType, Metadata};
use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::ReadDirSpec;

/// Representation of a file or directory.
#[derive(Debug)]
pub struct DirEntry {
  depth: usize,
  file_name: OsString,
  file_type: Result<FileType>,
  metadata: LazyCell<Result<Metadata>>,
  parent_spec: Option<Arc<ReadDirSpec>>,
  pub(crate) children_spec: Option<Arc<ReadDirSpec>>,
  read_children_error: Option<Error>,
}

impl DirEntry {
  pub(crate) fn new(
    depth: usize,
    file_name: OsString,
    file_type: Result<FileType>,
    metadata: Option<Result<Metadata>>,
    parent_spec: Option<Arc<ReadDirSpec>>,
    children_spec: Option<Arc<ReadDirSpec>>,
  ) -> DirEntry {
    let metadata_cell = LazyCell::new();
    if let Some(metadata) = metadata {
      metadata_cell.fill(metadata).unwrap();
    }
    DirEntry {
      depth,
      file_name,
      file_type,
      parent_spec,
      metadata: metadata_cell,
      children_spec: children_spec,
      read_children_error: None,
    }
  }

  // Should use std::convert::TryFrom when stable
  pub(crate) fn try_from(path: &Path) -> Result<DirEntry> {
    let metadata = fs::metadata(path)?;
    let root_name = OsString::from("/");
    let file_name = path.file_name().unwrap_or(&root_name);
    let parent_spec = path
      .parent()
      .map(|parent| Arc::new(ReadDirSpec::new(parent.to_path_buf(), 0, None)));

    Ok(DirEntry::new(
      0,
      file_name.to_owned(),
      Ok(metadata.file_type()),
      Some(Ok(metadata)),
      parent_spec,
      None,
    ))
  }

  /// Depth of this entry relative to the root directory where the walk started.
  pub fn depth(&self) -> usize {
    self.depth
  }

  /// File name of this entry without leading path component.
  pub fn file_name(&self) -> &OsStr {
    &self.file_name
  }

  /// File type for the file that this entry points at.
  ///
  /// This function will not traverse symlinks.
  pub fn file_type(&self) -> ::std::result::Result<&FileType, &Error> {
    self.file_type.as_ref()
  }

  /// Path to the file that this entry represents.
  ///
  /// The path is created by joining the `parent_spec` path with the filename of
  /// this entry.
  pub fn path(&self) -> PathBuf {
    let mut path = match self.parent_spec.as_ref() {
      Some(parent_spec) => parent_spec.path.to_path_buf(),
      None => PathBuf::from(""),
    };
    path.push(&self.file_name);
    path
  }

  /// Metadata for the file that this entry points at.
  ///
  /// This function will not traverse symlinks.
  pub fn metadata(&self) -> ::std::result::Result<&Metadata, &Error> {
    if !self.metadata.filled() {
      self.metadata.fill(fs::metadata(self.path())).unwrap();
    }
    self.metadata.borrow().unwrap().as_ref()
  }

  pub(crate) fn expects_children(&self) -> bool {
    self.children_spec.is_some()
  }

  /// Set [`ReadDirSpec`](struct.ReadDirSpec.html) used for reading this entry's
  /// children. This is set by default for any directory entry. The
  /// [`process_entries`](struct.WalkDir.html#method.process_entries) callback
  /// may call `entry.set_children_spec(None)` to skip descending into a
  /// particular directory.
  pub fn set_children_spec(&mut self, children_spec: Option<ReadDirSpec>) {
    self.children_spec = children_spec.map(|read_dir_spec| Arc::new(read_dir_spec));
  }

  /// Error generated when reading this entry's children.
  pub fn read_children_error(&self) -> Option<&Error> {
    self.read_children_error.as_ref()
  }

  pub(crate) fn set_read_children_error(&mut self, read_children_error: Option<Error>) {
    self.read_children_error = read_children_error;
  }
}
