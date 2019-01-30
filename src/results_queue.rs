use std::collections::BinaryHeap;
use std::marker::PhantomData;
use std::sync::mpsc::{self, Receiver, SendError, Sender, TryRecvError};
use std::thread;

use crate::walk::DirEntryContents;

#[derive(Clone)]
pub struct ResultsQueue<S>
where
  S: Clone,
{
  sender: Sender<DirEntryContents<S>>,
}

pub struct ResultsQueueIterator<S> {
  next_matcher: NextResultMatcher<S>,
  receiver: Receiver<DirEntryContents<S>>,
  receive_buffer: BinaryHeap<DirEntryContents<S>>,
}

struct NextResultMatcher<S> {
  index_path: Vec<usize>,
  remaining_siblings: Vec<usize>,
  phantom: PhantomData<S>,
}

pub fn new_results_queue<S>() -> (ResultsQueue<S>, ResultsQueueIterator<S>)
where
  S: Clone,
{
  let (sender, receiver) = mpsc::channel();
  (
    ResultsQueue { sender },
    ResultsQueueIterator {
      receiver,
      next_matcher: NextResultMatcher::default(),
      receive_buffer: BinaryHeap::new(),
    },
  )
}

impl<S> ResultsQueue<S>
where
  S: Clone,
{
  pub fn push(
    &self,
    dent: DirEntryContents<S>,
  ) -> std::result::Result<(), SendError<DirEntryContents<S>>> {
    self.sender.send(dent)
  }
}

impl<S> Iterator for ResultsQueueIterator<S> {
  type Item = DirEntryContents<S>;
  fn next(&mut self) -> Option<DirEntryContents<S>> {
    while self.receive_buffer.peek().map(|i| &i.index_path) != Some(&self.next_matcher.index_path) {
      if self.next_matcher.is_none() {
        return None;
      }

      match self.receiver.try_recv() {
        Ok(dir_entry_contents) => {
          self.receive_buffer.push(dir_entry_contents);
        }
        Err(err) => match err {
          TryRecvError::Empty => thread::yield_now(),
          TryRecvError::Disconnected => break,
        },
      }
    }

    if let Some(dir_entry_contents) = self.receive_buffer.pop() {
      self.next_matcher.increment_past(&dir_entry_contents);
      Some(dir_entry_contents)
    } else {
      None
    }
  }
}

impl<S> NextResultMatcher<S> {
  fn is_none(&self) -> bool {
    self.index_path.is_empty()
  }

  fn increment_past(&mut self, dir_entry_contents: &DirEntryContents<S>) {
    // Decrement remaining siblings at this level
    *self.remaining_siblings.last_mut().unwrap() -= 1;

    if dir_entry_contents.remaining_folders_with_contents > 0 {
      // If visited item has children then push 0 index path, since we are now
      // looking for the first child.
      self.index_path.push(0);
      self
        .remaining_siblings
        .push(dir_entry_contents.remaining_folders_with_contents);
    } else {
      // Incrememnt sibling index
      *self.index_path.last_mut().unwrap() += 1;

      // If no siblings remain at this level unwind stacks
      while !self.remaining_siblings.is_empty() && *self.remaining_siblings.last().unwrap() == 0 {
        self.index_path.pop();
        self.remaining_siblings.pop();
        // Finished processing level, so increment sibling index
        if !self.index_path.is_empty() {
          *self.index_path.last_mut().unwrap() += 1;
        }
      }
    }
  }
}

impl<S> Default for NextResultMatcher<S> {
  fn default() -> NextResultMatcher<S> {
    NextResultMatcher {
      index_path: vec![0],
      remaining_siblings: vec![1],
      phantom: PhantomData,
    }
  }
}
