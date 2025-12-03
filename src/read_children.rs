use std::path::Path;
use std::sync::Arc;

use crate::{ClientState, Error};

/// A reduced, but public, version of ReadDirSpec
pub struct ReadChildren<C: ClientState> {
    /// Path that will be used to read child entries. This is
    /// automatically set for directories.
    pub path: Arc<Path>,
    /// If the resulting `fs::read_dir` generates an error
    /// then that error is stored here.
    pub error: Option<Error>,
    /// Use this to customize the ReadDirState passed to the next
    /// process_read_dir.
    /// If None, will clone the previous ReadDirState after the parent
    /// call to process_read_dir.
    pub client_read_state: Option<C::ReadDirState>,
}

impl<C: ClientState> Clone for ReadChildren<C> {
    fn clone(&self) -> Self {
        ReadChildren {
            path: self.path.clone(),
            error: self.error.clone(),
            client_read_state: self.client_read_state.clone(),
        }
    }
}

impl<C: ClientState> ReadChildren<C> {
    /// Return the error stored that occurred when reading the directory.
    pub fn error(&self) -> Option<&Error> {
        self.error.as_ref()
    }
}
