use std::ffi::OsString;
use std::fs::{self, FileType, Metadata};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{ClientState, ReadDirSpec};

#[derive(Debug)]
#[cfg(unix)]
pub struct DirEntryExt {
    pub mode: u32,
    pub ino: u64,
    pub dev: u64,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub rdev: u64,
    pub blksize: u64,
    pub blocks: u64,
}

#[derive(Debug)]
#[cfg(windows)]
pub struct DirEntryExt {
    pub mode: u32,
    pub ino: u64,
    pub dev: u32,
    pub nlink: u32,
    pub size: u64,
}

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
    /// File type result for the file/directory that this entry points at.
    pub file_type_result: Result<FileType>,
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
    #[cfg(any(unix, windows))]
    pub ext: Option<Result<DirEntryExt>>,
}

impl<C: ClientState> DirEntry<C> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        depth: usize,
        file_name: OsString,
        file_type_result: Result<FileType>,
        metadata_result: Option<Result<Metadata>>,
        parent_path: Arc<PathBuf>,
        read_children_path: Option<Arc<PathBuf>>,
        client_state: C,
        #[cfg(any(unix, windows))] ext: Option<Result<DirEntryExt>>,
    ) -> DirEntry<C> {
        DirEntry {
            depth,
            file_name,
            file_type_result,
            parent_path,
            metadata_result,
            read_children_path,
            read_children_error: None,
            client_state,
            #[cfg(any(unix, windows))]
            ext,
        }
    }

    pub(crate) fn new_root_with_path(path: &Path) -> Result<DirEntry<C>> {
        let metadata = fs::metadata(path)?;
        let root_name = OsString::from("/");
        let file_name = path.file_name().unwrap_or(&root_name);
        let parent_path = Arc::new(path.parent().map(Path::to_path_buf).unwrap_or_default());
        let read_children_path = if metadata.file_type().is_dir() {
            Some(Arc::new(path.into()))
        } else {
            None
        };
        #[cfg(unix)]
        let ext = DirEntryExt {
            mode: metadata.mode(),
            ino: metadata.ino(),
            dev: metadata.dev(),
            nlink: metadata.nlink() as u32,
            uid: metadata.uid(),
            gid: metadata.gid(),
            size: metadata.size(),
            rdev: metadata.rdev(),
            blksize: metadata.blksize(),
            blocks: metadata.blocks(),
        };
        #[cfg(windows)]
        let ext = DirEntryExt {
            mode: metadata.file_attributes(),
            ino: 0,   //metadata.file_index().unwrap_or(0),
            dev: 0,   //metadata.volume_serial_number().unwrap_or(0),
            nlink: 0, //metadata.number_of_links().unwrap_or(0),
            size: metadata.file_size(),
        };
        Ok(DirEntry::new(
            0,
            file_name.to_owned(),
            Ok(metadata.file_type()),
            Some(Ok(metadata)),
            parent_path,
            read_children_path,
            C::default(),
            #[cfg(any(unix, windows))]
            Some(Ok(ext)),
        ))
    }

    /// Path to the file/directory represented by this entry.
    ///
    /// The path is created by joining `parent_path` with `file_name`.
    pub fn path(&self) -> PathBuf {
        self.parent_path.join(&self.file_name)
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
