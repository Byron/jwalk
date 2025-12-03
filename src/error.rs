use std::error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;



/// An error produced by recursively walking a directory.
///
/// This error type is a light wrapper around [`std::io::Error`]. In
/// particular, it adds the following information:
///
/// * The depth at which the error occurred in the file tree, relative to the
/// root.
/// * The path, if any, associated with the IO error.
/// * An indication that a loop occurred when following symbolic links. In this
/// case, there is no underlying IO error.
///
/// To maintain good ergonomics, this type has a
/// [`impl From<Error> for std::io::Error`][impl] defined which preserves the original context.
/// This allows you to use an [`io::Result`] with methods in this crate if you don't care about
/// accessing the underlying error data in a structured form.
///
/// [`std::io::Error`]: https://doc.rust-lang.org/stable/std/io/struct.Error.html
/// [`io::Result`]: https://doc.rust-lang.org/stable/std/io/type.Result.html
/// [impl]: struct.Error.html#impl-From%3CError%3E
#[derive(Debug, Clone)]
#[expect(dead_code)]
pub struct Error {
    depth: usize,
    inner: ErrorInner,
}

#[derive(Debug, Clone)]
#[expect(dead_code)]
enum ErrorInner {
    Io {
        path: Option<PathBuf>,
        err: Arc<io::Error>,
    },
    Loop {
        ancestor: PathBuf,
        child: PathBuf,
    },
    ThreadpoolBusy,
}

impl Error {
    pub(crate) fn from_path(depth: usize, pb: PathBuf, err: io::Error) -> Self {
        Error {
            depth,
            inner: ErrorInner::Io {
                path: Some(pb),
                err: Arc::new(err),
            },
        }
    }

    pub(crate) fn from_io(depth: usize, err: io::Error) -> Self {
        Error {
            depth,
            inner: ErrorInner::Io {
                path: None,
                err: Arc::new(err),
            },
        }
    }

    pub(crate) fn from_loop(depth: usize, ancestor: PathBuf, child: PathBuf) -> Self {
        Error {
            depth,
            inner: ErrorInner::Loop { ancestor, child },
        }
    }

    pub(crate) fn busy() -> Self {
        Error {
            depth: 0,
            inner: ErrorInner::ThreadpoolBusy,
        }
    }

    /// Returns the path associated with this error if one exists.
    ///
    /// For example, if an error occurred while opening a directory handle,
    /// the error will include the path passed to [`std::fs::read_dir`].
    ///
    /// [`std::fs::read_dir`]: https://doc.rust-lang.org/stable/std/fs/fn.read_dir.html
    pub fn path(&self) -> Option<&Path> {
        match &self.inner {
            ErrorInner::Io { path, .. } => path.as_ref().map(|p| p.as_path()),
            ErrorInner::Loop { child, .. } => Some(child.as_path()),
            ErrorInner::ThreadpoolBusy => None,
        }
    }

    /// Returns the path at which a cycle was detected.
    ///
    /// If no cycle was detected, [`None`] is returned.
    ///
    /// A cycle is detected when a directory entry is equivalent to one of
    /// its ancestors.
    ///
    /// To get the path to the child directory entry in the cycle, use the
    /// [`path`] method.
    ///
    /// [`None`]: https://doc.rust-lang.org/stable/std/option/enum.Option.html#variant.None
    /// [`path`]: struct.Error.html#path
    pub fn loop_ancestor(&self) -> Option<&Path> {
        match &self.inner {
            ErrorInner::Loop { ancestor, .. } => Some(ancestor.as_path()),
            _ => None,
        }
    }

    /// Returns the depth at which this error occurred relative to the root.
    ///
    /// The smallest depth is `0` and always corresponds to the path given to
    /// the [`new`] function on [`WalkDir`]. Its direct descendants have depth
    /// `1`, and their descendants have depth `2`, and so on.
    ///
    /// [`new`]: struct.WalkDir.html#method.new
    /// [`WalkDir`]: struct.WalkDir.html
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Inspect the original [`io::Error`] if there is one.
    ///
    /// [`None`] is returned if the [`Error`] doesn't correspond to an
    /// [`io::Error`]. This might happen, for example, when the error was
    /// produced because a cycle was found in the directory tree while
    /// following symbolic links.
    ///
    /// This method returns a borrowed value that is bound to the lifetime of the [`Error`]. To
    /// obtain an owned value, the [`into_io_error`] can be used instead.
    ///
    /// > This is the original [`io::Error`] and is _not_ the same as
    /// > [`impl From<Error> for std::io::Error`][impl] which contains additional context about the
    /// error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::io;
    /// use std::path::Path;
    ///
    /// use walkdir::WalkDir;
    ///
    /// for entry in WalkDir::new("foo") {
    ///     match entry {
    ///         Ok(entry) => println!("{}", entry.path().display()),
    ///         Err(err) => {
    ///             let path = err.path().unwrap_or(Path::new("")).display();
    ///             println!("failed to access entry {}", path);
    ///             if let Some(inner) = err.io_error() {
    ///                 match inner.kind() {
    ///                     io::ErrorKind::InvalidData => {
    ///                         println!(
    ///                             "entry contains invalid data: {}",
    ///                             inner)
    ///                     }
    ///                     io::ErrorKind::PermissionDenied => {
    ///                         println!(
    ///                             "Missing permission to read entry: {}",
    ///                             inner)
    ///                     }
    ///                     _ => {
    ///                         println!(
    ///                             "Unexpected error occurred: {}",
    ///                             inner)
    ///                     }
    ///                 }
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// [`None`]: https://doc.rust-lang.org/stable/std/option/enum.Option.html#variant.None
    /// [`io::Error`]: https://doc.rust-lang.org/stable/std/io/struct.Error.html
    /// [`From`]: https://doc.rust-lang.org/stable/std/convert/trait.From.html
    /// [`Error`]: struct.Error.html
    /// [`into_io_error`]: struct.Error.html#method.into_io_error
    /// [impl]: struct.Error.html#impl-From%3CError%3E
    pub fn io_error(&self) -> Option<&io::Error> {
        match &self.inner {
            ErrorInner::Io { err, .. } => Some(err),
            _ => None,
        }
    }

    /// Returns true if this error is due to a busy thread-pool that prevented its effective use.
    ///
    /// Note that business detection is timeout based, and we don't know if it would have been a deadlock or not.
    pub fn is_busy(&self) -> bool {
        matches!(&self.inner, ErrorInner::ThreadpoolBusy)
    }

    /// Similar to [`io_error`] except consumes self to convert to the original
    /// [`io::Error`] if one exists.
    ///
    /// [`io_error`]: struct.Error.html#method.io_error
    /// [`io::Error`]: https://doc.rust-lang.org/stable/std/io/struct.Error.html
    pub fn into_io_error(self) -> Option<io::Error> {
        match self.inner {
            ErrorInner::Io { err, .. } => {
                Some(Arc::try_unwrap(err).unwrap_or_else(|arc| {
                    io::Error::new(arc.kind(), format!("{}", arc))
                }))
            }
            _ => None,
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.io_error().map(|e| e as &(dyn error::Error + 'static))
    }

    #[allow(deprecated)]
    fn description(&self) -> &str {
        match &self.inner {
            ErrorInner::Io { err, .. } => err.description(),
            ErrorInner::Loop { .. } => "filesystem loop found",
            ErrorInner::ThreadpoolBusy => "threadpool is busy",
        }
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn error::Error> {
        self.source()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            ErrorInner::Io { path: Some(path), err } => {
                write!(f, "IO error for {}: {}", path.display(), err)
            }
            ErrorInner::Io { path: None, err } => {
                write!(f, "IO error: {}", err)
            }
            ErrorInner::Loop { child, ancestor } => {
                write!(
                    f,
                    "filesystem loop found: {} points to ancestor {}",
                    child.display(),
                    ancestor.display()
                )
            }
            ErrorInner::ThreadpoolBusy => {
                write!(f, "threadpool is busy")
            }
        }
    }
}

impl From<Error> for io::Error {
    /// Convert the [`Error`] to an [`io::Error`], preserving the original
    /// [`Error`] as the ["inner error"]. Note that this also makes the display
    /// of the error include the context.
    ///
    /// This is different from [`into_io_error`] which returns the original
    /// [`io::Error`].
    ///
    /// [`Error`]: struct.Error.html
    /// [`io::Error`]: https://doc.rust-lang.org/stable/std/io/struct.Error.html
    /// ["inner error"]: https://doc.rust-lang.org/std/io/struct.Error.html#method.into_inner
    /// [`into_io_error`]: struct.WalkDir.html#method.into_io_error
    fn from(walk_err: Error) -> io::Error {
        match walk_err.inner {
            ErrorInner::Io { err, .. } => {
                Arc::try_unwrap(err).unwrap_or_else(|arc| {
                    io::Error::new(arc.kind(), format!("{}", arc))
                })
            }
            ErrorInner::Loop { .. } => io::Error::new(io::ErrorKind::Other, walk_err),
            ErrorInner::ThreadpoolBusy => io::Error::new(io::ErrorKind::WouldBlock, walk_err),
        }
    }
}
