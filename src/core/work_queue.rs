use crossbeam::channel::{self, Receiver, SendError, Sender};
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::thread;

use super::ReadDirWork;

#[derive(Clone)]
pub(crate) struct WorkQueue<S> {
  sender: Sender<ReadDirWork<S>>,
  work_count: Arc<AtomicUsize>,
  stop_now: Arc<AtomicBool>,
}

pub(crate) struct WorkQueueIter<S> {
  receiver: Receiver<ReadDirWork<S>>,
  receive_buffer: BinaryHeap<ReadDirWork<S>>,
  work_count: Arc<AtomicUsize>,
  stop_now: Arc<AtomicBool>,
}

pub(crate) fn new_work_queue<S>() -> (WorkQueue<S>, WorkQueueIter<S>) {
  let work_count = Arc::new(AtomicUsize::new(0));
  let stop_now = Arc::new(AtomicBool::new(false));
  let (sender, receiver) = channel::unbounded();
  (
    WorkQueue {
      sender,
      work_count: work_count.clone(),
      stop_now: stop_now.clone(),
    },
    WorkQueueIter {
      receiver,
      receive_buffer: BinaryHeap::new(),
      work_count: work_count.clone(),
      stop_now: stop_now.clone(),
    },
  )
}

impl<S> WorkQueue<S> {
  pub fn push(&self, work: ReadDirWork<S>) -> std::result::Result<(), SendError<ReadDirWork<S>>> {
    self.work_count.fetch_add(1, AtomicOrdering::SeqCst);
    self.sender.send(work)
  }

  pub fn completed_work(&self) {
    self.work_count.fetch_sub(1, AtomicOrdering::SeqCst);
  }

  pub fn stop_now(&self) {
    self.stop_now.store(true, AtomicOrdering::SeqCst);
  }
}

impl<S> WorkQueueIter<S> {
  fn work_count(&self) -> usize {
    self.work_count.load(AtomicOrdering::SeqCst)
  }

  fn is_stop_now(&self) -> bool {
    self.stop_now.load(AtomicOrdering::SeqCst)
  }
}

impl<S> Iterator for WorkQueueIter<S> {
  type Item = ReadDirWork<S>;
  fn next(&mut self) -> Option<ReadDirWork<S>> {
    loop {
      if self.is_stop_now() {
        return None;
      }

      while let Ok(read_dir_work) = self.receiver.try_recv() {
        self.receive_buffer.push(read_dir_work)
      }

      if let Some(read_dir_work) = self.receive_buffer.pop() {
        return Some(read_dir_work);
      } else {
        if self.work_count() == 0 {
          return None;
        } else {
          thread::yield_now();
        }
      }
    }
  }
}
