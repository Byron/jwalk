use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::sync::Arc;
use std::thread;

use crate::walk::Work;

#[derive(Clone)]
pub(crate) struct WorkQueue<S> {
  sender: Sender<Work<S>>,
  work_count: Arc<AtomicUsize>,
  stop_now: Arc<AtomicBool>,
}

pub(crate) struct WorkQueueIterator<S> {
  receiver: Receiver<Work<S>>,
  receive_buffer: BinaryHeap<Work<S>>,
  work_count: Arc<AtomicUsize>,
  stop_now: Arc<AtomicBool>,
}

pub(crate) fn new_work_queue<S>() -> (WorkQueue<S>, WorkQueueIterator<S>) {
  let work_count = Arc::new(AtomicUsize::new(0));
  let stop_now = Arc::new(AtomicBool::new(false));
  let (sender, receiver) = mpsc::channel();
  (
    WorkQueue {
      sender,
      work_count: work_count.clone(),
      stop_now: stop_now.clone(),
    },
    WorkQueueIterator {
      receiver,
      receive_buffer: BinaryHeap::new(),
      work_count: work_count.clone(),
      stop_now: stop_now.clone(),
    },
  )
}

impl<S> WorkQueue<S> {
  pub fn push(&self, work: Work<S>) -> std::result::Result<(), SendError<Work<S>>> {
    self.work_count.fetch_add(1, AtomicOrdering::SeqCst);
    self.sender.send(work)
  }

  pub fn completed_work_item(&self) {
    self.work_count.fetch_sub(1, AtomicOrdering::SeqCst);
  }

  pub fn stop_now(&self) {
    self.stop_now.store(true, AtomicOrdering::SeqCst);
  }
}

impl<S> WorkQueueIterator<S> {
  fn work_count(&self) -> usize {
    self.work_count.load(AtomicOrdering::SeqCst)
  }

  fn is_stop_now(&self) -> bool {
    self.stop_now.load(AtomicOrdering::SeqCst)
  }
}

impl<S> Iterator for WorkQueueIterator<S> {
  type Item = Work<S>;
  fn next(&mut self) -> Option<Work<S>> {
    loop {
      if self.is_stop_now() {
        return None;
      }

      while let Ok(work) = self.receiver.try_recv() {
        self.receive_buffer.push(work)
      }

      if let Some(work) = self.receive_buffer.pop() {
        return Some(work);
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
