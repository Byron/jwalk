use crate::{ClientState, DirEntry, Result};

/// DirEntry iterator from `WalkDir.into_iter()`.
///
/// Yields entries from recursive traversal of filesystem.
pub struct DirEntryIter<C: ClientState> {
    _phantom: std::marker::PhantomData<C>,
}

impl<C: ClientState> Iterator for DirEntryIter<C> {
    type Item = Result<DirEntry<C>>;
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
