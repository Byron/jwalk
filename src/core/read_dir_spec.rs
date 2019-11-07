use std::path::PathBuf;
use std::sync::Arc;

use crate::ClientState;

/// Specification for reading a directory.
///
/// When a directory is read a new `ReadDirSpec` is created for each folder
/// found in that directory. These specs are then sent to a work queue that is
/// used to schedule future directory reads. Use
/// [`max_depth`](struct.WalkDir.html#method.max_depth) and
/// [`process_entries`](struct.WalkDir.html#method.process_entries) to change
/// this default behavior.
#[derive(Debug)]
pub struct ReadDirSpec<C: ClientState> {
    /// Depth of the directory to read relative to root of walk.
    pub depth: usize,
    /// Path of the the directory to read.
    pub path: Arc<PathBuf>,
    /// Client state that was set in the
    /// [`process_entries`](struct.WalkDir.html#method.process_entries) callback
    /// when reading this directories parent. One intended use case is to store
    /// `.gitignore` state to filter entries during the walk.
    pub client_state: C,
}
