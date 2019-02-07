//! Parallel recursive directory traversal.
//!
//! - Walk is performed in parallel using rayon
//! - Results are streamed in sorted order
//!
//! This crate is inspired by both [`walkdir`](https://crates.io/crates/walkdir)
//! and [`ignore`](https://crates.io/crates/ignore). I attempts to match the
//! performance of `ignore::WalkParallel` while also providing streamed sorted
//! results like `walkdir`.
//!
//! It's a work in progress...
//!
//! The [`WalkDir`] type builds iterators. The [`DirEntry`] type describes
//! values yielded by the iterator.
//!
//! [`WalkDir`]: struct.WalkDir.html
//! [`DirEntry`]: struct.DirEntry.html
//!
//! # Example
//!
//! Recursively iterate over the "foo" directory and print each entry's path:
//!
//! COMMETN tHIS OUt```no_run
//! # use std::io::Error;
//! use jwalk::WalkDir;
//!
//! # fn try_main() -> Result<(), Error> {
//! for entry in WalkDir::new("foo") {
//!     println!("{}", entry?.path().display());
//! }
//! # Ok(())
//! # }
//! ```

mod walk;

pub mod core;

pub use crate::walk::*;
