use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::mpsc::{self, Receiver, SendError, Sender, TryRecvError};
use std::sync::Arc;
use std::thread;

use crate::ordered_walk::DirEntry;

#[derive(Clone)]
pub struct ResultsQueue {
  sender: Sender<DirEntry>,
}

pub struct ResultsQueueIterator {
  receiver: Receiver<DirEntry>,
  receive_buffer: BinaryHeap<DirEntry>,
  next_matcher: ResultsQueueNextMatcher,
}

struct ResultsQueueNextMatcher {
  index_path: Vec<usize>,
  remaining_siblings: Vec<usize>,
}

pub fn new_results_queue() -> (ResultsQueue, ResultsQueueIterator) {
  let (sender, receiver) = mpsc::channel();
  (
    ResultsQueue { sender },
    ResultsQueueIterator {
      receiver,
      next_matcher: ResultsQueueNextMatcher::default(),
      receive_buffer: BinaryHeap::new(),
    },
  )
}

impl ResultsQueue {
  pub fn push(&self, dent: DirEntry) -> std::result::Result<(), SendError<DirEntry>> {
    self.sender.send(dent)
  }
}

impl Iterator for ResultsQueueIterator {
  type Item = DirEntry;
  fn next(&mut self) -> Option<DirEntry> {
    /*match self.receiver.recv() {
      Ok(entry) => Some(entry),
      Err(_) => None,
    }*/

    while self.receive_buffer.peek().map(|i| &i.index_path) != Some(&self.next_matcher.index_path) {
      if self.next_matcher.is_none() {
        return None;
      }

      match self.receiver.try_recv() {
        Ok(dentry) => {
          self.receive_buffer.push(dentry);
          return self.receive_buffer.pop();
        }
        Err(err) => match err {
          TryRecvError::Empty => thread::yield_now(),
          TryRecvError::Disconnected => break,
        },
      }
    }

    if let Some(item) = self.receive_buffer.pop() {
      self.next_matcher.increment_past(&item);
      Some(item)
    } else {
      None
    }
  }
}

impl ResultsQueueNextMatcher {
  fn is_none(&self) -> bool {
    self.index_path.is_empty()
  }
  fn increment_past(&mut self, entry: &DirEntry) {
    // Decrement remaining siblings at this level
    *self.remaining_siblings.last_mut().unwrap() -= 1;

    if entry.remaining_content_count > 0 {
      // If visited item has children then push 0 index path, since we are now
      // looking for the first child.
      self.index_path.push(0);
      self.remaining_siblings.push(entry.remaining_content_count);
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

impl Default for ResultsQueueNextMatcher {
  fn default() -> ResultsQueueNextMatcher {
    ResultsQueueNextMatcher {
      index_path: vec![0],
      remaining_siblings: vec![1],
    }
  }
}
