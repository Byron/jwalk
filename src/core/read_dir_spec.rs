use std::path::Path;
use std::sync::Arc;

use crate::ClientState;

/// Specification for reading a directory.
///
/// When a directory is read a new `ReadDirSpec` is created for each folder
/// found in that directory. These specs are then sent to a work queue that is
/// used to schedule future directory reads. Use
/// [`max_depth`](struct.WalkDir.html#method.max_depth) and
/// [`process_read_dir`](struct.WalkDir.html#method.process_read_dir) to change
/// this default behavior.
#[derive(Debug)]
pub struct ReadDirSpec<C: ClientState> {
    /// Depth of the directory to read relative to root of walk.
    pub depth: usize,
    /// Path of the the directory to read.
    pub path: Arc<Path>,
    /// Client branch state that was set in the
    /// [`process_read_dir`](struct.WalkDir.html#method.process_read_dir) callback
    /// when reading this directory's parent. One intended use case is to store
    /// `.gitignore` state to filter entries during the walk.
    pub client_read_state: C::ReadDirState,
    // Origins of symlinks followed to get to this entry.
    pub(crate) follow_link_ancestors: Arc<Vec<Arc<Path>>>,
}
