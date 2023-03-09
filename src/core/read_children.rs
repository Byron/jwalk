use std::path::Path;
use std::sync::Arc;

use crate::{ClientState, Error};

// A reduced, but public, version of ReadDirSpec
pub struct ReadChildren<C: ClientState> {
    /// Path that will be used to read child entries. This is
    /// automatically set for directories.
    pub(crate) path: Arc<Path>,
    /// If the resulting `fs::read_dir` generates an error
    /// then that error is stored here.
    pub(crate) error: Option<Error>,
    /// Use this to customize the ReadDirState passed to the next
    /// process_read_dir.
    /// If None, will clone the previous ReadDirState after the parent
    /// call to process_read_dir.
    pub client_read_state: Option<C::ReadDirState>,
}

impl<C: ClientState> ReadChildren<C> {
    pub(crate) fn new(path: &Path) -> Self {
        Self {
            path: Arc::from(path),
            error: None,
            client_read_state: None,
        }
    }

    pub fn error(&self) -> Option<&Error> {
        self.error.as_ref()
    }
}
