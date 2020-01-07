use std::io::Result;

use super::{ClientState, DirEntry, IndexPath, Ordered, ReadDirSpec};

/// Results of successfully reading a directory.
#[derive(Debug)]
pub struct ReadDir<C: ClientState> {
    pub(crate) parent_client_state: C,
    pub(crate) dir_entry_results: Vec<Result<DirEntry<C>>>,
}

impl<C: ClientState> ReadDir<C> {
    pub fn new(parent_client_state: C, dir_entry_results: Vec<Result<DirEntry<C>>>) -> ReadDir<C> {
        ReadDir {
            parent_client_state,
            dir_entry_results,
        }
    }

    pub fn read_children_specs(&self) -> Vec<ReadDirSpec<C>> {
        self.dir_entry_results
            .iter()
            .filter_map(|each| each.as_ref().ok()?.read_children_spec())
            .collect()
    }

    pub fn ordered_read_children_specs(
        &self,
        index_path: &IndexPath,
    ) -> Vec<Ordered<ReadDirSpec<C>>> {
        self.dir_entry_results
            .iter()
            .filter_map(|each| each.as_ref().ok()?.read_children_spec())
            .enumerate()
            .map(|(i, spec)| Ordered::new(spec, index_path.adding(i), 0))
            .collect()
    }
}
