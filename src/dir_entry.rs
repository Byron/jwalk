use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, FileType};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{ClientState, ReadChildren, Result};

/// Representation of a file or directory.
///
/// This representation does not wrap a `std::fs::DirEntry`. Instead it copies
/// `file_name`, `file_type`, and optionally `metadata` out of the underlying
/// `std::fs::DirEntry`. This allows it to quickly drop the underlying file
/// descriptor.
#[expect(dead_code)]
pub struct DirEntry<C: ClientState> {
    /// Depth of this entry relative to the root directory where the walk
    /// started.
    pub depth: usize,
    /// File name of this entry without leading path component.
    pub file_name: OsString,
    /// File type for the file/directory that this entry points at.
    pub file_type: FileType,
    /// Field where clients can store state from within the The
    /// [`process_read_dir`](struct.WalkDirGeneric.html#method.process_read_dir)
    /// callback.
    pub client_state: C::DirEntryState,
    /// Path used by this entry's parent to read this entry.
    pub parent_path: Arc<Path>,
    /// Describes how to recurse from this DirEntry.
    /// The [`process_read_dir`](struct.WalkDirGeneric.html#method.process_read_dir)
    /// callback may set this field to `None` to skip reading the
    /// contents of a particular directory.
    pub read_children: Option<ReadChildren<C>>,
    // True if [`follow_links`] is `true` AND was created from a symlink path.
    follow_link: bool,
    // Origins of symlinks followed to get to this entry.
    follow_link_ancestors: Arc<Vec<Arc<Path>>>,
}

impl<C: ClientState> DirEntry<C> {


    /// Return the file type for the file that this entry points to.
    ///
    /// If this is a symbolic link and [`follow_links`] is `true`, then this
    /// returns the type of the target.
    ///
    /// This never makes any system calls.
    ///
    /// [`follow_links`]: struct.WalkDir.html#method.follow_links
    pub fn file_type(&self) -> FileType {
        todo!()
    }

    /// Return the file name of this entry.
    ///
    /// If this entry has no file name (e.g., `/`), then the full path is
    /// returned.
    pub fn file_name(&self) -> &OsStr {
        todo!()
    }

    /// Returns the depth at which this entry was created relative to the root.
    ///
    /// The smallest depth is `0` and always corresponds to the path given
    /// to the `new` function on `WalkDir`. Its direct descendants have depth
    /// `1`, and their descendants have depth `2`, and so on.
    pub fn depth(&self) -> usize {
        todo!()
    }

    /// Path to the file/directory represented by this entry.
    ///
    /// The path is created by joining `parent_path` with `file_name`.
    pub fn path(&self) -> PathBuf {
        todo!()
    }

    /// Returns `true` if and only if this entry was created from a symbolic
    /// link. This is unaffected by the [`follow_links`] setting.
    ///
    /// When `true`, the value returned by the [`path`] method is a
    /// symbolic link name. To get the full target path, you must call
    /// [`std::fs::read_link(entry.path())`].
    ///
    /// [`path`]: struct.DirEntry.html#method.path
    /// [`follow_links`]: struct.WalkDir.html#method.follow_links
    /// [`std::fs::read_link(entry.path())`]: https://doc.rust-lang.org/stable/std/fs/fn.read_link.html
    pub fn path_is_symlink(&self) -> bool {
        todo!()
    }

    /// Return the metadata for the file that this entry points to.
    ///
    /// This will follow symbolic links if and only if the [`WalkDir`] value
    /// has [`follow_links`] enabled.
    ///
    /// # Platform behavior
    ///
    /// This always calls [`std::fs::symlink_metadata`].
    ///
    /// If this entry is a symbolic link and [`follow_links`] is enabled, then
    /// [`std::fs::metadata`] is called instead.
    ///
    /// # Errors
    ///
    /// Similar to [`std::fs::metadata`], returns errors for path values that
    /// the program does not have permissions to access or if the path does not
    /// exist.
    ///
    /// [`WalkDir`]: struct.WalkDir.html
    /// [`follow_links`]: struct.WalkDir.html#method.follow_links
    /// [`std::fs::metadata`]: https://doc.rust-lang.org/std/fs/fn.metadata.html
    /// [`std::fs::symlink_metadata`]: https://doc.rust-lang.org/stable/std/fs/fn.symlink_metadata.html
    pub fn metadata(&self) -> Result<fs::Metadata> {
        todo!()
    }

    /// Reference to the path of the directory containing this entry.
    pub fn parent_path(&self) -> &Path {
        todo!()
    }


}

impl<C: ClientState> fmt::Debug for DirEntry<C> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
