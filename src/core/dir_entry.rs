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
/// `std::fs::DirEntry`. This allows it to quickly drop the underlying file
/// descriptor.
#[derive(Debug)]
pub struct DirEntry {
  /// Depth of this entry relative to the root directory where the walk started.
  pub depth: usize,
  /// File name of this entry without leading path component.
  pub file_name: OsString,
  /// File type result for the file/directory that this entry points at.
  pub file_type: Result<FileType>,
  /// Metadata result for the file/directory that this entry points at. Defaults
  /// to `None`. Filled in by the walk process when the
  /// [`preload_metadata`](struct.WalkDir.html#method.preload_metadata) option
  /// is set.
  pub metadata: Option<Result<Metadata>>,
  /// [`ReadDirSpec`](struct.ReadDirSpec.html) used for reading this entry's
  /// content entries. This is automatically set for directories. The
  /// [`process_entries`](struct.WalkDir.html#method.process_entries) callback
  /// may set this field to `None` to skip reading the contents of this
  /// particular directory.
  pub content_spec: Option<Arc<ReadDirSpec>>,
  /// If `fs::read_dir` generates an error when reading this entry's content
  /// entries then that error is stored here.
  pub content_error: Option<Error>,
  /// [`ReadDirSpec`](struct.ReadDirSpec.html) used by this entry's parent to
  /// read this entry. If this is the root entry then depth value of this parent
  /// spec will be `0` even though it should logically be `-1`.
  pub parent_spec: Arc<ReadDirSpec>,
}

impl DirEntry {
  pub(crate) fn new(
    depth: usize,
    file_name: OsString,
    file_type: Result<FileType>,
    metadata: Option<Result<Metadata>>,
    parent_spec: Arc<ReadDirSpec>,
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

  pub(crate) fn new_root_with_path(path: &Path) -> Result<DirEntry> {
    let metadata = fs::metadata(path)?;
    let root_name = OsString::from("/");
    let file_name = path.file_name().unwrap_or(&root_name);
    let parent_path = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let parent_spec = Arc::new(ReadDirSpec::new(parent_path, 0, None));

    Ok(DirEntry::new(
      0,
      file_name.to_owned(),
      Ok(metadata.file_type()),
      Some(Ok(metadata)),
      parent_spec,
      None,
    ))
  }

  /// Path to the file/directory represented by this entry.
  ///
  /// The path is created by joining `parent_path` with `file_name`.
  pub fn path(&self) -> PathBuf {
    self.parent_spec.path.join(&self.file_name)
  }

  /// Reference to the path of the directory containing this entry.
  pub fn parent_path(&self) -> &Path {
    &self.parent_spec.path
  }
}
