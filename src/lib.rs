#![warn(clippy::all)]
#![cfg_attr(windows, feature(windows_by_handle))]

//! Filesystem walk.
//!
//! - Performed in parallel using rayon
//! - Entries streamed in sorted order
//! - Custom sort/filter/skip/state
//!
//! # Example
//!
//! Recursively iterate over the "foo" directory sorting by name:
//!
//! ```no_run
//! # use std::io::Error;
//! use jwalk::{WalkDir};
//!
//! # fn try_main() -> Result<(), Error> {
//! for entry in WalkDir::new("foo").sort(true) {
//!   println!("{}", entry?.path().display());
//! }
//! # Ok(())
//! # }
//! ```
//! # Extended Example
//!
//! This example uses the
//! [`process_entries`](struct.WalkDirGeneric.html#method.process_entries)
//! callback for custom:
//! 1. **Sort** Entries by name
//! 2. **Filter** Errors and hidden files
//! 3. **Skip** Content of directories at depth 2
//! 4. **State** Mark first entry in each directory with
//!    [`client_state`](struct.DirEntry.html#field.client_state) true. Also mark
//!    all directories that contain a first entry with true.
//!
//! ```no_run
//! # use std::io::Error;
//! use std::cmp::Ordering;
//! use jwalk::{ WalkDirGeneric };
//!
//! # fn try_main() -> Result<(), Error> {
//! let walk_dir = WalkDirGeneric::<bool>::new("foo")
//!     .process_entries(|parent_client_state, children| {
//!         // 1. Custom sort
//!         children.sort_by(|a, b| match (a, b) {
//!             (Ok(a), Ok(b)) => a.file_name.cmp(&b.file_name),
//!             (Ok(_), Err(_)) => Ordering::Less,
//!             (Err(_), Ok(_)) => Ordering::Greater,
//!             (Err(_), Err(_)) => Ordering::Equal,
//!         });
//!         // 2. Custom filter
//!         children.retain(|dir_entry_result| {
//!             dir_entry_result.as_ref().map(|dir_entry| {
//!                 dir_entry.file_name
//!                     .to_str()
//!                     .map(|s| s.starts_with('.'))
//!                     .unwrap_or(false)
//!             }).unwrap_or(false)
//!         });
//!         // 3. Custom skip
//!         children.iter_mut().for_each(|dir_entry_result| {
//!             if let Ok(dir_entry) = dir_entry_result {
//!                 if dir_entry.depth == 2 {
//!                     dir_entry.read_children_path = None;
//!                 }
//!             }
//!         });
//!         // 4. Custom state
//!         children.first_mut().map(|dir_entry_result| {
//!             *parent_client_state = true;
//!             if let Ok(dir_entry) = dir_entry_result {
//!                 dir_entry.client_state = true;
//!             }
//!         });
//!     });
//!
//! for entry in walk_dir {
//!   println!("{}", entry?.path().display());
//! }
//! # Ok(())
//! # }
//! ```

pub mod core;

use rayon::{ThreadPool, ThreadPoolBuilder};
use std::cmp::Ordering;
use std::default::Default;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs;
use std::io::Result;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::{ReadDir, ReadDirSpec};

#[cfg(any(unix, windows))]
pub use crate::core::DirEntryExt;
pub use crate::core::{DirEntry, DirEntryIter};

/// Builder for walking a directory.
pub type WalkDir = WalkDirGeneric<()>;

/// Generic builder for walking a directory.
///
/// [`ClientState`](trait.ClientState.html) type parameter allows you to specify
/// the type of state that can be stored in DirEntry's
/// [`client_state`](struct.DirEntry.html#field.client_state) field from within
/// the [`process_entries`](struct.WalkDirGeneric.html#method.process_entries)
/// callback.
///
/// Use [`WalkDir`](type.WalkDir.html) if you don't need to store client state
/// into yeilded DirEntries.
pub struct WalkDirGeneric<C: ClientState> {
    root: PathBuf,
    options: WalkDirOptions<C>,
}

type ProcessEntriesFunction<C> =
    dyn Fn(&mut C, &mut Vec<Result<DirEntry<C>>>) + Send + Sync + 'static;

/// Trait for state stored in DirEntry's
/// [`client_state`](struct.DirEntry.html#field.client_state) field.
///
/// Client state can be stored from within the
/// [`process_entries`](struct.WalkDirGeneric.html#method.process_entries) callback.
/// The type of ClientState is determined by WalkDirGeneric type parameter.
pub trait ClientState: Clone + Send + Default + Debug + 'static {}

/// Degree of parallelism to use when performing walk.
///
/// Parallelism happens at the directory level. It will help when walking deep
/// filesystems with many directories. It wont help when reading a single
/// directory with many files.
#[derive(Clone)]
pub enum Parallelism {
    /// Run on calling thread
    Serial,
    /// Run in default rayon thread pool
    RayonDefaultPool,
    /// Run in existing rayon thread pool
    RayonExistingPool(Arc<ThreadPool>),
    /// Run in new rayon thread pool with # threads
    RayonNewPool(usize),
}

struct WalkDirOptions<C: ClientState> {
    sort: bool,
    max_depth: usize,
    skip_hidden: bool,
    parallelism: Parallelism,
    preload_metadata: bool,
    preload_metadata_ext: bool,
    process_entries: Option<Arc<ProcessEntriesFunction<C>>>,
}

impl<C: ClientState> WalkDirGeneric<C> {
    /// Create a builder for a recursive directory iterator starting at the file
    /// path root. If root is a directory, then it is the first item yielded by
    /// the iterator. If root is a file, then it is the first and only item
    /// yielded by the iterator.
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        WalkDirGeneric {
            root: root.as_ref().to_path_buf(),
            options: WalkDirOptions {
                sort: false,
                max_depth: ::std::usize::MAX,
                parallelism: Parallelism::RayonDefaultPool,
                skip_hidden: true,
                preload_metadata: false,
                preload_metadata_ext: false,
                process_entries: None,
            },
        }
    }

    /// Root path of the walk.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Sort entries by `file_name` per directory. Defaults to `false`. Use
    /// [`process_entries`](struct.WalkDirGeneric.html#method.process_entries) for custom
    /// sorting or filtering.
    pub fn sort(mut self, sort: bool) -> Self {
        self.options.sort = sort;
        self
    }

    /// Skip hidden entries. Enabled by default.
    pub fn skip_hidden(mut self, skip_hidden: bool) -> Self {
        self.options.skip_hidden = skip_hidden;
        self
    }

    /// Preload metadata before yielding entries. When running in parrallel the
    /// metadata is loaded in rayon's thread pool.
    pub fn preload_metadata(mut self, preload_metadata: bool) -> Self {
        self.options.preload_metadata = preload_metadata;
        self
    }

    pub fn preload_metadata_ext(mut self, preload_metadata_ext: bool) -> Self {
        self.options.preload_metadata_ext = preload_metadata_ext;
        self
    }

    /// Maximum depth of entries yielded by the iterator. `0` corresponds to the
    /// root path of this walk.
    ///
    /// A depth < 2 will automatically change `parallelism` to
    /// `Parallelism::Serial`. Parrallelism happens at the `fs::read_dir` level.
    /// It only makes sense to use multiple threads when reading more then one
    /// directory.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.options.max_depth = depth;
        if depth == 1 {
            self.options.parallelism = Parallelism::Serial;
        }
        self
    }

    /// Degree of parallelism to use when performing walk. Defaults to
    /// [`Parallelism::RayonDefaultPool`](enum.Parallelism.html#variant.RayonDefaultPool).
    pub fn parallelism(mut self, parallelism: Parallelism) -> Self {
        self.options.parallelism = parallelism;
        self
    }

    /// A callback function to process (sort/filter/skip/state) each directory
    /// of entries before they are yielded. Modify the given array to
    /// sort/filter entries. Use [`entry.read_children_path =
    /// None`](struct.DirEntry.html#field.read_children_path) to yield a
    /// directory entry but skip reading its contents. Use
    /// [`entry.client_state`](struct.DirEntry.html#field.client_state) to store
    /// custom state with an entry.
    pub fn process_entries<F>(mut self, process_by: F) -> Self
    where
        F: Fn(&mut C, &mut Vec<Result<DirEntry<C>>>) + Send + Sync + 'static,
    {
        self.options.process_entries = Some(Arc::new(process_by));
        self
    }
}

impl<C: ClientState> IntoIterator for WalkDirGeneric<C> {
    type Item = Result<DirEntry<C>>;
    type IntoIter = DirEntryIter<C>;

    fn into_iter(self) -> DirEntryIter<C> {
        let sort = self.options.sort;
        let parallelism = self.options.parallelism;
        let skip_hidden = self.options.skip_hidden;
        let max_depth = self.options.max_depth;
        let preload_metadata = self.options.preload_metadata;
        let preload_metadata_ext = self.options.preload_metadata_ext;
        let process_entries = self.options.process_entries.clone();
        let root_entry_results = if let Some(process_entries) = process_entries.as_ref() {
            let mut root_entry_results = vec![DirEntry::new_root_with_path(&self.root)];
            process_entries(&mut C::default(), &mut root_entry_results);
            root_entry_results
        } else {
            vec![DirEntry::new_root_with_path(&self.root)]
        };

        DirEntryIter::new(
            root_entry_results,
            parallelism,
            Arc::new(move |read_dir_spec| {
                let ReadDirSpec {
                    depth,
                    path,
                    mut client_state,
                } = read_dir_spec;

                let depth = depth + 1;

                if depth > max_depth {
                    return Ok(ReadDir::new(client_state, Vec::new()));
                }

                let mut dir_entry_results: Vec<_> = fs::read_dir(path.as_ref())?
                    .filter_map(|dir_entry_result| {
                        let dir_entry = match dir_entry_result {
                            Ok(dir_entry) => dir_entry,
                            Err(err) => return Some(Err(err)),
                        };

                        let file_name = dir_entry.file_name();
                        if skip_hidden && is_hidden(&file_name) {
                            return None;
                        }

                        let file_type = dir_entry.file_type();
                        let metadata = if preload_metadata || preload_metadata_ext {
                            Some(dir_entry.metadata())
                        } else {
                            None
                        };

                        let read_children_path = match file_type {
                            Ok(file_type) => {
                                if file_type.is_dir() && depth < max_depth {
                                    Some(Arc::new(path.as_ref().join(dir_entry.file_name())))
                                } else {
                                    None
                                }
                            }
                            Err(_) => None,
                        };
                        #[cfg(unix)]
                        let ext = if preload_metadata_ext {
                            let metadata_ext = metadata.as_ref().unwrap().as_ref().unwrap();
                            Some(Ok(DirEntryExt {
                                mode: metadata_ext.mode(),
                                ino: metadata_ext.ino(),
                                dev: metadata_ext.dev(),
                                nlink: metadata_ext.nlink() as u32,
                                uid: metadata_ext.uid(),
                                gid: metadata_ext.gid(),
                                size: metadata_ext.size(),
                                rdev: metadata_ext.rdev(),
                                blksize: metadata_ext.blksize(),
                                blocks: metadata_ext.blocks(),
                            }))
                        } else {
                            None
                        };
                        #[cfg(windows)]
                        let ext = if preload_metadata_ext {
                            let metadata_ext = fs::metadata(path.as_ref()).unwrap();
                            Some(Ok(DirEntryExt {
                                mode: metadata_ext.file_attributes(),
                                ino: metadata_ext.file_index().unwrap_or(0),
                                dev: metadata_ext.volume_serial_number().unwrap_or(0),
                                nlink: metadata_ext.number_of_links().unwrap_or(0),
                                size: metadata_ext.file_size(),
                            }))
                        } else {
                            None
                        };
                        Some(Ok(DirEntry::new(
                            depth,
                            file_name,
                            file_type,
                            metadata,
                            path.clone(),
                            read_children_path,
                            C::default(),
                            #[cfg(any(unix, windows))]
                            ext,
                        )))
                    })
                    .collect();

                if sort {
                    dir_entry_results.sort_by(|a, b| match (a, b) {
                        (Ok(a), Ok(b)) => a.file_name.cmp(&b.file_name),
                        (Ok(_), Err(_)) => Ordering::Less,
                        (Err(_), Ok(_)) => Ordering::Greater,
                        (Err(_), Err(_)) => Ordering::Equal,
                    });
                }

                if let Some(process_entries) = process_entries.as_ref() {
                    process_entries(&mut client_state, &mut dir_entry_results);
                }

                Ok(ReadDir::new(client_state, dir_entry_results))
            }),
        )
    }
}

impl<C: ClientState> Clone for WalkDirOptions<C> {
    fn clone(&self) -> WalkDirOptions<C> {
        WalkDirOptions {
            sort: false,
            max_depth: self.max_depth,
            parallelism: self.parallelism.clone(),
            skip_hidden: self.skip_hidden,
            preload_metadata: self.preload_metadata,
            preload_metadata_ext: self.preload_metadata_ext,
            process_entries: self.process_entries.clone(),
        }
    }
}

impl Parallelism {
    pub(crate) fn spawn<OP>(&self, op: OP)
    where
        OP: FnOnce() -> () + Send + 'static,
    {
        match self {
            Parallelism::Serial => op(),
            Parallelism::RayonDefaultPool => rayon::spawn(op),
            Parallelism::RayonNewPool(num_threads) => {
                if let Ok(thread_pool) = ThreadPoolBuilder::new().num_threads(*num_threads).build()
                {
                    thread_pool.spawn(op);
                } else {
                    rayon::spawn(op);
                }
            }
            Parallelism::RayonExistingPool(thread_pool) => thread_pool.spawn(op),
        }
    }
}

fn is_hidden(file_name: &OsStr) -> bool {
    file_name
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

impl<T> ClientState for T where T: Clone + Send + Debug + Default + 'static {}
