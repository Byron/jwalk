use crossbeam::channel::{self, Receiver, SendError, Sender, TryRecvError};
use std::collections::BinaryHeap;
use std::marker::PhantomData;
use std::thread;

use super::Delegate;
use super::IndexPath;
use super::WorkResults;

#[derive(Clone)]
pub struct WorkResultsQueue<D>
where
  D: Delegate,
{
  sender: Sender<WorkResults<D>>,
}

pub struct WorkResultsQueueIter<D>
where
  D: Delegate,
{
  next_matcher: NextResultMatcher<D>,
  receiver: Receiver<WorkResults<D>>,
  receive_buffer: BinaryHeap<WorkResults<D>>,
}

struct NextResultMatcher<D>
where
  D: Delegate,
{
  looking_for_index_path: IndexPath,
  remaining_work: Vec<usize>,
  phantom: PhantomData<D>,
}

pub fn new_work_results_queue<D>() -> (WorkResultsQueue<D>, WorkResultsQueueIter<D>)
where
  D: Delegate,
{
  //let (sender, receiver) = channel::bounded(100);
  let (sender, receiver) = channel::unbounded();
  (
    WorkResultsQueue { sender },
    WorkResultsQueueIter {
      receiver,
      next_matcher: NextResultMatcher::default(),
      receive_buffer: BinaryHeap::new(),
    },
  )
}

impl<D> WorkResultsQueue<D>
where
  D: Delegate,
{
  pub fn push(&self, dent: WorkResults<D>) -> std::result::Result<(), SendError<WorkResults<D>>> {
    self.sender.send(dent)
  }
}

impl<D> Iterator for WorkResultsQueueIter<D>
where
  D: Delegate,
{
  type Item = WorkResults<D>;
  fn next(&mut self) -> Option<WorkResults<D>> {
    let looking_for = &self.next_matcher.looking_for_index_path;
    loop {
      let top_dir_list = self.receive_buffer.peek();
      if let Some(top_dir_list) = top_dir_list {
        if top_dir_list.index_path.eq(looking_for) {
          break;
        }
      }

      if self.next_matcher.is_none() {
        return None;
      }

      match self.receiver.try_recv() {
        Ok(dir_list) => {
          self.receive_buffer.push(dir_list);
        }
        Err(err) => match err {
          TryRecvError::Empty => thread::yield_now(),
          TryRecvError::Disconnected => break,
        },
      }
    }

    if let Some(dir_list) = self.receive_buffer.pop() {
      self.next_matcher.increment_past(&dir_list);
      Some(dir_list)
    } else {
      None
    }
  }
}

impl<D> NextResultMatcher<D>
where
  D: Delegate,
{
  fn is_none(&self) -> bool {
    self.looking_for_index_path.is_empty()
  }

  fn decrement_remaining_work_at_this_level(&mut self) {
    *self.remaining_work.last_mut().unwrap() -= 1;
  }

  fn increment_past(&mut self, branch: &WorkResults<D>) {
    self.decrement_remaining_work_at_this_level();

    if branch.scheduled_work > 0 {
      // If visited item has children then push 0 index path, since we are now
      // looking for the first child.
      self.looking_for_index_path.push(0);
      self.remaining_work.push(branch.scheduled_work);
    } else {
      // Incrememnt sibling index
      self.looking_for_index_path.increment_last();

      // If no siblings remain at this level unwind stacks
      while !self.remaining_work.is_empty() && *self.remaining_work.last().unwrap() == 0 {
        self.looking_for_index_path.pop();
        self.remaining_work.pop();
        // Finished processing level, so increment sibling index
        if !self.looking_for_index_path.is_empty() {
          self.looking_for_index_path.increment_last();
        }
      }
    }
  }
}

impl<D> Default for NextResultMatcher<D>
where
  D: Delegate,
{
  fn default() -> NextResultMatcher<D> {
    NextResultMatcher {
      looking_for_index_path: IndexPath::new(vec![0]),
      remaining_work: vec![1],
      phantom: PhantomData,
    }
  }
}
