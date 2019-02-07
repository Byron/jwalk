use std::sync::Arc;

use super::*;

/// ReadDirResult Iterator.
pub enum ReadDirIter<F>
where
  F: Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Clone + 'static,
{
  Walk {
    read_dir_spec_stack: Vec<Ordered<Arc<ReadDirSpec>>>,
    client_function: F,
  },
  ParWalk {
    read_dir_result_iter: OrderedQueueIter<Result<ReadDir>>,
  },
}

impl<F> Iterator for ReadDirIter<F>
where
  F: Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Clone + 'static,
{
  type Item = Result<ReadDir>;
  fn next(&mut self) -> Option<Self::Item> {
    match self {
      ReadDirIter::Walk {
        read_dir_spec_stack,
        client_function,
      } => {
        let read_dir_spec = match read_dir_spec_stack.pop() {
          Some(read_dir_spec) => read_dir_spec,
          None => return None,
        };

        let (read_dir_result, children_specs) = run_client_function(client_function, read_dir_spec);

        if let Some(children_specs) = children_specs {
          for each in children_specs {
            read_dir_spec_stack.push(each)
          }
        }

        Some(read_dir_result.value)
      }

      ReadDirIter::ParWalk {
        read_dir_result_iter,
      } => {
        if let Some(ordered_read_dir_result) = read_dir_result_iter.next() {
          Some(ordered_read_dir_result.value)
        } else {
          None
        }
      }
    }
  }
}

/// Result<DirEntry> Iterator.
pub struct DirEntryIter<F>
where
  F: Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Clone + 'static,
{
  read_dir_iter: ReadDirIter<F>,
  read_dir_iter_stack: Vec<vec::IntoIter<Result<DirEntry>>>,
}

impl<F> DirEntryIter<F>
where
  F: Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Clone + 'static,
{
  pub fn new(read_dir_iter: ReadDirIter<F>) -> DirEntryIter<F> {
    DirEntryIter {
      read_dir_iter,
      read_dir_iter_stack: Vec::new(),
    }
  }

  fn push_next_read_dir_iter(&mut self) -> Option<Error> {
    let read_dir_result = self.read_dir_iter.next().unwrap();
    let read_dir = match read_dir_result {
      Ok(read_dir) => read_dir,
      Err(err) => return Some(err),
    };
    self.read_dir_iter_stack.push(read_dir.into_iter());
    None
  }
}

impl<F> Iterator for DirEntryIter<F>
where
  F: Fn(Arc<ReadDirSpec>) -> Result<ReadDir> + Send + Clone + 'static,
{
  type Item = Result<DirEntry>;
  fn next(&mut self) -> Option<Self::Item> {
    loop {
      if self.read_dir_iter_stack.is_empty() {
        return None;
      }

      let top_read_dir_iter = self.read_dir_iter_stack.last_mut().unwrap();

      if let Some(dir_entry_result) = top_read_dir_iter.next() {
        let mut dir_entry = match dir_entry_result {
          Ok(dir_entry) => dir_entry,
          Err(err) => return Some(Err(err)),
        };

        if dir_entry.children_spec().is_some() {
          dir_entry.set_children_error(self.push_next_read_dir_iter());
        }

        return Some(Ok(dir_entry));
      } else {
        self.read_dir_iter_stack.pop();
      }
    }
  }
}
