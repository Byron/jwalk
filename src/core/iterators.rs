use std::iter::Peekable;
use std::sync::Arc;

use super::*;

/// Result<ReadDir> Iterator.
pub enum ReadDirIter {
  Walk {
    read_dir_spec_stack: Vec<Ordered<Arc<ReadDirSpec>>>,
    client_function: Arc<ClientReadDirFunction>,
  },
  ParWalk {
    read_dir_result_iter: OrderedQueueIter<Result<ReadDir>>,
  },
}

impl Iterator for ReadDirIter {
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
          for each in children_specs.into_iter().rev() {
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
///
/// Flattens a ReadDirIter into an iterator over individual Result<DirEntry>.
pub struct DirEntryIter {
  read_dir_iter_stack: Vec<vec::IntoIter<Result<DirEntry>>>,
  read_dir_iter: Peekable<ReadDirIter>,
  root_entry_result: Option<Result<DirEntry>>,
}

impl DirEntryIter {
  pub fn new(read_dir_iter: ReadDirIter, root_entry_result: Result<DirEntry>) -> DirEntryIter {
    DirEntryIter {
      read_dir_iter: read_dir_iter.peekable(),
      read_dir_iter_stack: Vec::new(),
      root_entry_result: Some(root_entry_result),
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

impl Iterator for DirEntryIter {
  type Item = Result<DirEntry>;
  fn next(&mut self) -> Option<Self::Item> {
    if let Some(root_entry_result) = self.root_entry_result.take() {
      return Some(root_entry_result);
    }

    loop {
      if self.read_dir_iter_stack.is_empty() {
        if self.read_dir_iter.peek().is_some() {
          self.push_next_read_dir_iter();
        } else {
          return None;
        }
      }

      let top_read_dir_iter = self.read_dir_iter_stack.last_mut().unwrap();

      if let Some(dir_entry_result) = top_read_dir_iter.next() {
        let mut dir_entry = match dir_entry_result {
          Ok(dir_entry) => dir_entry,
          Err(err) => return Some(Err(err)),
        };

        if dir_entry.expects_children() {
          dir_entry.set_children_error(self.push_next_read_dir_iter());
        }

        return Some(Ok(dir_entry));
      } else {
        self.read_dir_iter_stack.pop();
      }
    }
  }
}
