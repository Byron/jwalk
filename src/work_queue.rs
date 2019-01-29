use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::sync::Arc;
use std::thread;

use crate::walk::Work;

#[derive(Clone)]
pub(crate) struct WorkQueue {
  sender: Sender<Work>,
  work_count: Arc<AtomicUsize>,
  stop_now: Arc<AtomicBool>,
}

pub(crate) struct WorkQueueIterator {
  receiver: Receiver<Work>,
  work_count: Arc<AtomicUsize>,
  stop_now: Arc<AtomicBool>,
}

pub(crate) struct SortedWorkQueueIterator {
  receiver: Receiver<Work>,
  receive_buffer: BinaryHeap<Work>,
  work_count: Arc<AtomicUsize>,
  stop_now: Arc<AtomicBool>,
}

pub(crate) fn new_work_queue() -> (WorkQueue, WorkQueueIterator) {
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
      work_count: work_count.clone(),
      stop_now: stop_now.clone(),
    },
  )
}

pub(crate) fn new_sorted_work_queue() -> (WorkQueue, SortedWorkQueueIterator) {
  let work_count = Arc::new(AtomicUsize::new(0));
  let stop_now = Arc::new(AtomicBool::new(false));
  let (sender, receiver) = mpsc::channel();
  (
    WorkQueue {
      sender,
      work_count: work_count.clone(),
      stop_now: stop_now.clone(),
    },
    SortedWorkQueueIterator {
      receiver,
      receive_buffer: BinaryHeap::new(),
      work_count: work_count.clone(),
      stop_now: stop_now.clone(),
    },
  )
}

impl WorkQueue {
  pub fn push(&self, work: Work) -> std::result::Result<(), SendError<Work>> {
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

impl WorkQueueIterator {
  fn work_count(&self) -> usize {
    self.work_count.load(AtomicOrdering::SeqCst)
  }

  fn is_stop_now(&self) -> bool {
    self.stop_now.load(AtomicOrdering::SeqCst)
  }
}

impl SortedWorkQueueIterator {
  fn work_count(&self) -> usize {
    self.work_count.load(AtomicOrdering::SeqCst)
  }

  fn is_stop_now(&self) -> bool {
    self.stop_now.load(AtomicOrdering::SeqCst)
  }
}

impl Iterator for WorkQueueIterator {
  type Item = Work;
  fn next(&mut self) -> Option<Work> {
    loop {
      if self.is_stop_now() {
        return None;
      }

      if let Ok(work) = self.receiver.try_recv() {
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

impl Iterator for SortedWorkQueueIterator {
  type Item = Work;
  fn next(&mut self) -> Option<Work> {
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
