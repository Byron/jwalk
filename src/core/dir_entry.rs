use std::ffi::OsString;
use std::fs::{self, FileType, Metadata};
use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{ClientState, ReadDirSpec};

/// Representation of a file or directory.
///
/// This representation does not wrap a `std::fs::DirEntry`. Instead it copies
/// `file_name`, `file_type`, and optionaly `metadata` out of the underlying
/// `std::fs::DirEntry`. This allows it to quickly drop the underlying file
/// descriptor.
#[derive(Debug)]
pub struct DirEntry<C: ClientState> {
    /// Depth of this entry relative to the root directory where the walk
    /// started.
    pub depth: usize,
    /// File name of this entry without leading path component.
    pub file_name: OsString,
    /// File type for the file/directory that this entry points at.
    pub file_type: FileType,
    /// Metadata result for the file/directory that this entry points at. Defaults
    /// to `None`. Filled in by the walk process when the
    /// [`preload_metadata`](struct.WalkDir.html#method.preload_metadata) option
    /// is set.
    pub metadata_result: Option<Result<Metadata>>,
    /// Field where clients can store state from within the The
    /// [`process_entries`](struct.WalkDirGeneric.html#method.process_entries)
    /// callback. This state will be cloned once for entries that have a
    /// `read_children_path` set.
    pub client_state: C,
    /// Path used by this entry's parent to read this entry.
    pub parent_path: Arc<PathBuf>,
    /// Path that will be used to read child entries. This is automatically set
    /// for directories. The
    /// [`process_entries`](struct.WalkDirGeneric.html#method.process_entries) callback
    /// may set this field to `None` to skip reading the contents of a
    /// particular directory.
    pub read_children_path: Option<Arc<PathBuf>>,
    /// If `read_children_path` is set and resulting `fs::read_dir` generates an error
    /// then that error is stored here.
    pub read_children_error: Option<Error>,
    
    follow_link: bool,
}

impl<C: ClientState> DirEntry<C> {
    pub(crate) fn from_entry(
        depth: usize,
        parent_path: Arc<PathBuf>,
        fs_dir_entry: &fs::DirEntry,
    ) -> Result<Self> {
        let file_type = fs_dir_entry.file_type()?;
        let file_name = fs_dir_entry.file_name();
        let read_children_path = if file_type.is_dir() {
            Some(Arc::new(parent_path.join(&file_name)))
        } else {
            None
        };

        Ok(DirEntry {
            depth,
            file_name,
            file_type,
            follow_link: false,
            metadata_result: None,
            parent_path,
            read_children_path,
            read_children_error: None,
            client_state: C::default(),
        })
    }

    pub(crate) fn from_path(
        depth: usize,
        path: &Path,
        follow: bool,
    ) -> Result<Self> {
        let metadata = if follow {
            fs::metadata(&path)?
        } else {
            fs::symlink_metadata(&path)?
        };

        let root_name = OsString::from("/");
        let file_name = path.file_name().unwrap_or(&root_name);
        let parent_path = Arc::new(path.parent().map(Path::to_path_buf).unwrap_or_default());
        let read_children_path = if metadata.file_type().is_dir() {
            Some(Arc::new(path.into()))
        } else {
            None
        };

        Ok(DirEntry {
            depth,
            file_name: file_name.to_owned(),
            file_type: metadata.file_type(),
            follow_link: false,
            metadata_result: Some(Ok(metadata)),
            parent_path,
            read_children_path,
            read_children_error: None,
            client_state: C::default(),
        })
    }

    /*
    pub(crate) fn new(
        depth: usize,
        file_name: OsString,
        file_type: FileType,
        metadata_result: Option<Result<Metadata>>,
        parent_path: Arc<PathBuf>,
        read_children_path: Option<Arc<PathBuf>>,
        client_state: C,
    ) -> DirEntry<C> {
        DirEntry {
            depth,
            file_name,
            file_type,
            parent_path,
            metadata_result,
            read_children_path,
            read_children_error: None,
            client_state,
            follow_link: false,
        }
    }

    pub(crate) fn new_root_with_path(path: &Path) -> Result<DirEntry<C>> {
        let metadata = fs::symlink_metadata(path)?;
        let root_name = OsString::from("/");
        let file_name = path.file_name().unwrap_or(&root_name);
        let parent_path = Arc::new(path.parent().map(Path::to_path_buf).unwrap_or_default());
        let read_children_path = if metadata.file_type().is_dir() {
            Some(Arc::new(path.into()))
        } else {
            None
        };

        Ok(DirEntry::new(
            0,
            file_name.to_owned(),
            metadata.file_type(),
            Some(Ok(metadata)),
            parent_path,
            read_children_path,
            C::default(),
        ))
    }*/

    /// Path to the file/directory represented by this entry.
    ///
    /// The path is created by joining `parent_path` with `file_name`.
    pub fn path(&self) -> PathBuf {
        self.parent_path.join(&self.file_name)
    }

    pub fn path_is_symlink(&self) -> bool {
        self.file_type.is_symlink() || self.follow_link
    }

    /// Reference to the path of the directory containing this entry.
    pub fn parent_path(&self) -> &Path {
        &self.parent_path
    }

    pub(crate) fn read_children_spec(&self) -> Option<ReadDirSpec<C>> {
        self.read_children_path.as_ref().map({
            |children_path| ReadDirSpec {
                depth: self.depth,
                path: children_path.clone(),
                client_state: self.client_state.clone(),
            }
        })
    }
}
