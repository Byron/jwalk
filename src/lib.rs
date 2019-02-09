//! Fast recursive directory walk.
//!
//! - Walk is performed in parallel using rayon
//! - Results are streamed in sorted order
//! - Custom sort/filter/skip if needed
//!
//! [![Build Status](https://travis-ci.org/jessegrosjean/jwalk.svg?branch=master)](https://travis-ci.org/jessegrosjean/jwalk)
//!
//! # Example
//!
//! Recursively iterate over the "foo" directory sorting by name:
//!
//! ```no_run
//! # use std::io::Error;
//! use jwalk::{Sort, WalkDir};
//!
//! # fn try_main() -> Result<(), Error> {
//! for entry in WalkDir::new("foo").sort(Some(Sort::Name)) {
//!   println!("{}", entry?.path().display());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Inspiration
//!
//! This crate is inspired by both [`walkdir`](https://crates.io/crates/walkdir)
//! and [`ignore`](https://crates.io/crates/ignore). It attempts to combine the
//! parallelism of `ignore` with `walkdir`s streaming iterator API.
//!
//! # Why use this crate?
//!
//! Speed and flexibility.
//!
//! This crate is particularly fast when you want streamed sorted results. In
//! this case it's much faster then `walkdir` and has much better latency then
//! `ignore`.
//!
//! This crate's `process_entries` callback allows you to arbitrarily
//! sort/filter/skip each directories entries before they are yielded. This
//! processing happens in the thread pool and effects the directory traversal.
//! It can be much faster then post processing the yielded entries.
//!
//! # Why not use this crate?
//!
//! Directory traversal is already pretty fast. If you don't need this crate's
//! speed then `walkdir` provides a smaller and more tested single threaded
//! implementation.
//!
//! # Benchmarks
//!
//! Time to walk linux's source code:
//!
//! | Crate   | Options                        | Time      |
//! |---------|--------------------------------|-----------|
//! | jwalk   | unsorted, parallel             | 60.811 ms |
//! | jwalk   | sorted, parallel               | 61.445 ms |
//! | jwalk   | sorted, parallel, metadata     | 100.95 ms |
//! | jwalk   | unsorted, parallel (2 threads) | 99.998 ms |
//! | jwalk   | unsorted, serial               | 168.68 ms |
//! | jwalk   | sorted, parallel, first 100    | 9.9794 ms |
//! | ignore  | unsorted, parallel             | 74.251 ms |
//! | ignore  | sorted, parallel               | 99.336 ms |
//! | ignore  | sorted, parallel, metadata     | 134.26 ms |
//! | walkdir | unsorted                       | 162.09 ms |
//! | walkdir | sorted                         | 200.09 ms |
//! | walkdir | sorted, metadata               | 422.74 ms |

mod core;

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::io::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::{DirEntryIter, ReadDir};

pub use crate::core::{DirEntry, ReadDirSpec};

/// Builder for walking a directory.
pub struct WalkDir {
  root: PathBuf,
  options: WalkDirOptions,
}

/// Directory sort options.
///
/// If you need more flexibility use
/// [`process_entries`](struct.WalkDir.html#method.process_entries).
#[derive(Clone)]
pub enum Sort {
  Name,
  Access,
  Creation,
  Modification,
}

struct WalkDirOptions {
  sort: Option<Sort>,
  max_depth: usize,
  skip_hidden: bool,
  num_threads: usize,
  preload_metadata: bool,
  process_entries: Option<Arc<Fn(&mut Vec<Result<DirEntry>>) + Send + Sync>>,
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
        sort: None,
        max_depth: ::std::usize::MAX,
        num_threads: 0,
        skip_hidden: true,
        preload_metadata: false,
        process_entries: None,
      },
    }
  }

  /// Set the maximum depth of entries yield by the iterator.
  ///
  /// The smallest depth is `0` and always corresponds to the path given to the
  /// `new` function on this type. Its direct descendents have depth `1`, and
  /// their descendents have depth `2`, and so on.
  ///
  /// Note that a depth < 2 will automatically change `thread_count` to 1.
  /// `jwalks` parrallelism happens at the `fs::read_dir` level, so it only
  /// makes sense to use multiple threads when reading more then one directory.
  pub fn max_depth(mut self, depth: usize) -> Self {
    self.options.max_depth = depth;
    if depth == 1 {
      self.options.num_threads = 1;
    }
    self
  }

  /// Sort entries per directory. Use
  /// [`process_entries`](struct.WalkDir.html#method.process_entries) for custom
  /// sorting or filtering.
  pub fn sort(mut self, sort: Option<Sort>) -> Self {
    self.options.sort = sort;
    self
  }

  /// - `0` Use rayon global pool.
  /// - `1` Perform walk on calling thread.
  /// - `n > 1` Construct a new rayon ThreadPool to perform the walk.
  pub fn num_threads(mut self, n: usize) -> Self {
    self.options.num_threads = n;
    self
  }

  /// Skip hidden entries. Enabled by default.
  pub fn skip_hidden(mut self, skip_hidden: bool) -> Self {
    self.options.skip_hidden = skip_hidden;
    self
  }

  /// Preload metadata before yeilding entries. When running in parrallel the
  /// metadata is loaded in rayon's thread pool.
  pub fn preload_metadata(mut self, preload_metadata: bool) -> Self {
    self.options.preload_metadata = preload_metadata;
    self
  }

  /// Set a function to process (sort/filter/skip) each directory of entries
  /// before they are yeilded. Use
  /// [`entry.set_children_spec(None)`](struct.DirEntry.html#method.children_spec)
  /// to yeild that directory but skip descending into its contents.
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

    let dir_entry_iter = core::walk(&self.root, num_threads, move |read_dir_spec| {
      let depth = read_dir_spec.depth + 1;
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

          let children_spec = match file_type {
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
            Some(read_dir_spec.clone()),
            children_spec,
          )))
        })
        .collect();

      sort
        .as_ref()
        .map(|sort| sort.perform_sort(&mut dir_entry_results));

      process_entries.as_ref().map(|process_entries| {
        process_entries(&mut dir_entry_results);
      });

      Ok(ReadDir::new(dir_entry_results))
    });

    dir_entry_iter
  }
}

impl Sort {
  fn perform_sort(&self, dir_entry_results: &mut Vec<Result<DirEntry>>) {
    dir_entry_results.sort_by(|a, b| match (a, b) {
      (Ok(a), Ok(b)) => a.file_name().cmp(b.file_name()),
      (Ok(_), Err(_)) => Ordering::Less,
      (Err(_), Ok(_)) => Ordering::Greater,
      (Err(_), Err(_)) => Ordering::Equal,
    });
  }
}

impl Clone for WalkDirOptions {
  fn clone(&self) -> WalkDirOptions {
    WalkDirOptions {
      sort: None,
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
    .map(|s| s.starts_with("."))
    .unwrap_or(false)
}
