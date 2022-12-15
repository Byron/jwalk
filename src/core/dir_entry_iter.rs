use std::iter::Peekable;

use super::*;
use crate::Result;

/// DirEntry iterator from `WalkDir.into_iter()`.
///
/// Yields entries from recursive traversal of filesystem.
pub struct DirEntryIter<C: ClientState> {
    min_depth: usize,
    // iterator yielding next ReadDir results when needed
    pub(crate) read_dir_iter: Option<Peekable<ReadDirIter<C>>>,
    // stack of ReadDir results, track location in filesystem traversal
    read_dir_results_stack: Vec<vec::IntoIter<Result<DirEntry<C>>>>,
}

impl<C: ClientState> DirEntryIter<C> {
    pub(crate) fn new(
        root_entry_results: Vec<Result<DirEntry<C>>>,
        parallelism: Parallelism,
        min_depth: usize,
        root_read_dir_state: C::ReadDirState,
        core_read_dir_callback: Arc<ReadDirCallback<C>>,
    ) -> DirEntryIter<C> {
        // 1. Gather read_dir_specs from root level
        let read_dir_specs: Vec<_> = root_entry_results
            .iter()
            .flat_map(|dir_entry_result| {
                dir_entry_result
                    .as_ref()
                    .ok()?
                    .read_children_spec(root_read_dir_state.clone())
            })
            .collect();

        // 2. Init new read_dir_iter from those specs
        let read_dir_iter =
            ReadDirIter::try_new(read_dir_specs, parallelism, core_read_dir_callback)
                .map(|iter| iter.peekable());

        // 3. Return DirEntryIter that will return initial root entries and then
        //    fill and process read_dir_iter until complete
        DirEntryIter {
            min_depth,
            read_dir_iter,
            read_dir_results_stack: vec![root_entry_results.into_iter()],
        }
    }

    fn push_next_read_dir_results(
        iter: &mut Peekable<ReadDirIter<C>>,
        results: &mut Vec<vec::IntoIter<Result<DirEntry<C>>>>,
    ) -> Result<()> {
        // Push next read dir results or return error if read failed
        let read_dir_result = iter.next().unwrap();
        let read_dir = match read_dir_result {
            Ok(read_dir) => read_dir,
            Err(err) => return Err(err),
        };

        let ReadDir { results_list, .. } = read_dir;
        results.push(results_list.into_iter());

        Ok(())
    }
}

impl<C: ClientState> Iterator for DirEntryIter<C> {
    type Item = Result<DirEntry<C>>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // 1. Get current read dir results iter from top of stack
            let top_read_dir_results = self.read_dir_results_stack.last_mut()?;

            // 2. If more results in current read dir then process
            if let Some(dir_entry_result) = top_read_dir_results.next() {
                // 2.1 Handle error case
                let mut dir_entry = match dir_entry_result {
                    Ok(dir_entry) => dir_entry,
                    Err(err) => return Some(Err(err)),
                };
                // 2.2 If dir_entry has a read_children_path means we need to read a new
                // directory and push those results onto read_dir_results_stack
                if dir_entry.read_children_path.is_some() {
                    let iter = match self.read_dir_iter.as_mut().ok_or_else(Error::busy) {
                        Ok(iter) => iter,
                        Err(err) => return Some(Err(err)),
                    };
                    if let Err(err) =
                        Self::push_next_read_dir_results(iter, &mut self.read_dir_results_stack)
                    {
                        dir_entry.read_children_error = Some(err);
                    }
                }

                if dir_entry.depth >= self.min_depth {
                    // 2.3 Finished, return dir_entry
                    return Some(Ok(dir_entry));
                }
            } else {
                // If no more results in current then pop stack
                self.read_dir_results_stack.pop();
            }
        }
    }
}
