use std::ffi::OsString;
use std::fs::{self, FileType, Metadata};
use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::ReadDirSpec;

/// Representation of a file or directory.
///
/// This representation does not wrap a `std::fs::DirEntry`. Instead it copies
/// `file_name`, `file_type`, and optionaly `metadata` out of the underlying
/// `std::fs::DirEntry` so that it can drop the underlying file descriptor as
/// soon as possible.
#[derive(Debug)]
pub struct DirEntry {
  /// Depth of this entry relative to the root directory where the walk started.
  pub depth: usize,
  /// File name of this entry without leading path component.
  pub file_name: OsString,
  /// File type result for the file/directory that this entry points at.
  pub file_type: Result<FileType>,
  /// Metadata result for the file/directory that this entry points at. Defaults
  /// to `None`. Set when
  /// [`preload_metadata`](struct.WalkDir.html#method.preload_metadata) is set.
  pub metadata: Option<Result<Metadata>>,
  /// [`ReadDirSpec`](struct.ReadDirSpec.html) used for reading this entry's
  /// content. This is automatically set for directory entries. The
  /// [`process_entries`](struct.WalkDir.html#method.process_entries) callback
  /// may set to `None` to skip descending into a particular directory.
  pub content_spec: Option<Arc<ReadDirSpec>>,
  /// `fs::read_dir` error generated after trying to read this entry's content
  /// using the spec in `content_spec`.
  pub content_error: Option<Error>,
  /// [`ReadDirSpec`](struct.ReadDirSpec.html) used by this entry's parent to
  /// read this entry.
  pub parent_spec: Option<Arc<ReadDirSpec>>,
}

impl DirEntry {
  pub(crate) fn new(
    depth: usize,
    file_name: OsString,
    file_type: Result<FileType>,
    metadata: Option<Result<Metadata>>,
    parent_spec: Option<Arc<ReadDirSpec>>,
    content_spec: Option<Arc<ReadDirSpec>>,
  ) -> DirEntry {
    DirEntry {
      depth,
      file_name,
      file_type,
      parent_spec,
      metadata,
      content_spec,
      content_error: None,
    }
  }

  /// Path to the file/directory represented by this entry.
  ///
  /// The path is created by joining `parent_path` with `file_name`.
  pub fn path(&self) -> PathBuf {
    let mut path = match self.parent_spec.as_ref() {
      Some(parent_spec) => parent_spec.path.to_path_buf(),
      None => PathBuf::from(""),
    };
    path.push(&self.file_name);
    path
  }

  /// Reference to the path of the directory containing this entry.
  pub fn parent_path(&self) -> Option<&Path> {
    self
      .parent_spec
      .as_ref()
      .map(|parent_spec| parent_spec.path.as_ref())
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
}
