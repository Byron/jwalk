#![warn(clippy::all)]

//! Filesystem walk.
//!
//! - Performed in parallel using rayon
//! - Entries streamed in sorted order
//! - Custom sort/filter/skip
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

mod core;

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::io::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::ReadDir;

pub use crate::core::{DirEntry, DirEntryIter, ReadDirSpec};

/// Builder for walking a directory.
///
/// Note that symlinks are always followed when walking.
pub struct WalkDir {
    root: PathBuf,
    options: WalkDirOptions,
}

type ProcessEntriesFunction = dyn Fn(&mut Vec<Result<DirEntry>>) + Send + Sync + 'static;

struct WalkDirOptions {
    sort: bool,
    max_depth: usize,
    skip_hidden: bool,
    num_threads: usize,
    preload_metadata: bool,
    process_entries: Option<Arc<ProcessEntriesFunction>>,
}

impl WalkDir {
    /// Create a builder for a recursive directory iterator starting at the file
    /// path root. If root is a directory, then it is the first item yielded by
    /// the iterator. If root is a file, then it is the first and only item
    /// yielded by the iterator.
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        WalkDir {
            root: root.as_ref().to_path_buf(),
            options: WalkDirOptions {
                sort: false,
                max_depth: ::std::usize::MAX,
                num_threads: 0,
                skip_hidden: true,
                preload_metadata: false,
                process_entries: None,
            },
        }
    }

    /// Root path of the walk.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Sort entries by `file_name` per directory. Defaults to `false`. Use
    /// [`process_entries`](struct.WalkDir.html#method.process_entries) for custom
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
    ///
    /// This is equivalent to calling `std::fs::symlink_metadata` on all
    /// entries.
    pub fn preload_metadata(mut self, preload_metadata: bool) -> Self {
        self.options.preload_metadata = preload_metadata;
        self
    }

    /// Maximum depth of entries yielded by the iterator. `0` corresponds to the
    /// root path of this walk.
    ///
    /// A depth < 2 will automatically change `thread_count` to 1. Parrallelism
    /// happens at the `fs::read_dir` level. It only makes sense to use multiple
    /// threads when reading more then one directory.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.options.max_depth = depth;
        if depth == 1 {
            self.options.num_threads = 1;
        }
        self
    }

    /// - `0` Use rayon global pool.
    /// - `1` Perform walk on calling thread.
    /// - `n > 1` Construct a new rayon ThreadPool to perform the walk.
    pub fn num_threads(mut self, n: usize) -> Self {
        self.options.num_threads = n;
        self
    }

    /// A callback function to process (sort/filter/skip) each directory of
    /// entries before they are yielded. Modify the given array to sort/filter
    /// entries. Use [`entry.content_spec =
    /// None`](struct.DirEntry.html#field.content_spec) to yield an entry but skip
    /// reading its contents.
    pub fn process_entries<F>(mut self, process_by: F) -> Self
    where
        F: Fn(&mut Vec<Result<DirEntry>>) + Send + Sync + 'static,
    {
        self.options.process_entries = Some(Arc::new(process_by));
        self
    }
}

impl IntoIterator for WalkDir {
    type Item = Result<DirEntry>;
    type IntoIter = DirEntryIter;

    fn into_iter(self) -> DirEntryIter {
        let sort = self.options.sort;
        let num_threads = self.options.num_threads;
        let skip_hidden = self.options.skip_hidden;
        let max_depth = self.options.max_depth;
        let preload_metadata = self.options.preload_metadata;
        let process_entries = self.options.process_entries.clone();

        core::walk(&self.root, num_threads, move |read_dir_spec| {
            let depth = read_dir_spec.depth + 1;

            if depth > max_depth {
                return Ok(ReadDir::new(Vec::new()));
            }

            let mut dir_entry_results: Vec<_> = fs::read_dir(&read_dir_spec.path)?
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
                    let metadata = if preload_metadata {
                        Some(dir_entry.metadata())
                    } else {
                        None
                    };

                    let content_spec = match file_type {
                        Ok(file_type) => {
                            if file_type.is_dir() && depth < max_depth {
                                let path = read_dir_spec.path.join(dir_entry.file_name());
                                Some(Arc::new(ReadDirSpec::new(path, depth, None)))
                            } else {
                                None
                            }
                        }
                        Err(_) => None,
                    };

                    Some(Ok(DirEntry::new(
                        depth,
                        file_name,
                        file_type,
                        metadata,
                        read_dir_spec.clone(),
                        content_spec,
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
                process_entries(&mut dir_entry_results);
            }

            Ok(ReadDir::new(dir_entry_results))
        })
    }
}

impl Clone for WalkDirOptions {
    fn clone(&self) -> WalkDirOptions {
        WalkDirOptions {
            sort: false,
            max_depth: self.max_depth,
            num_threads: self.num_threads,
            skip_hidden: self.skip_hidden,
            preload_metadata: self.preload_metadata,
            process_entries: self.process_entries.clone(),
        }
    }
}

fn is_hidden(file_name: &OsStr) -> bool {
    file_name
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}
