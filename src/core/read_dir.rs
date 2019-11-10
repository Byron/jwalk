use super::{ClientState, DirEntry, IndexPath, Ordered, ReadDirSpec};
use crate::Result;

/// Results of successfully reading a directory.
#[derive(Debug)]
pub struct ReadDir<C: ClientState> {
    pub(crate) read_dir_state: C::ReadDirState,
    pub(crate) results_list: Vec<Result<DirEntry<C>>>,
}

impl<C: ClientState> ReadDir<C> {
    pub fn new(
        read_dir_state: C::ReadDirState,
        results_list: Vec<Result<DirEntry<C>>>,
    ) -> ReadDir<C> {
        ReadDir {
            read_dir_state,
            results_list,
        }
    }

    pub fn read_children_specs(&self) -> impl Iterator<Item = ReadDirSpec<C>> + '_ {
        self.results_list.iter().filter_map(move |each| {
            each.as_ref()
                .ok()?
                .read_children_spec(self.read_dir_state.clone())
        })
    }

    pub fn ordered_read_children_specs(
        &self,
        index_path: &IndexPath,
    ) -> Vec<Ordered<ReadDirSpec<C>>> {
        self.read_children_specs()
            .enumerate()
            .map(|(i, spec)| Ordered::new(spec, index_path.adding(i), 0))
            .collect()
    }
}
