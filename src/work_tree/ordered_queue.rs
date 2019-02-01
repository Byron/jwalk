use crossbeam::channel::{self, Receiver, SendError, Sender};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::thread;

use super::*;

#[derive(Clone)]
pub(crate) struct OrderedQueue<T>
where
  T: Send,
{
  sender: Sender<Ordered<T>>,
  pending_count: Arc<AtomicUsize>,
  stop: Arc<AtomicBool>,
}

pub(crate) struct OrderedQueueIter<T>
where
  T: Send,
{
  receiver: Receiver<Ordered<T>>,
  receive_buffer: BinaryHeap<Ordered<T>>,
  pending_count: Arc<AtomicUsize>,
  stop: Arc<AtomicBool>,
}

pub(crate) fn new_ordered_queue<T>() -> (OrderedQueue<T>, OrderedQueueIter<T>)
where
  T: Send,
{
  let pending_count = Arc::new(AtomicUsize::new(0));
  let stop = Arc::new(AtomicBool::new(false));
  let (sender, receiver) = channel::unbounded();
  (
    OrderedQueue {
      sender,
      pending_count: pending_count.clone(),
      stop: stop.clone(),
    },
    OrderedQueueIter {
      receiver,
      receive_buffer: BinaryHeap::new(),
      pending_count: pending_count.clone(),
      stop: stop.clone(),
    },
  )
}

impl<T> OrderedQueue<T>
where
  T: Send,
{
  pub fn push(&self, ordered: Ordered<T>) -> std::result::Result<(), SendError<Ordered<T>>> {
    self.pending_count.fetch_add(1, AtomicOrdering::SeqCst);
    self.sender.send(ordered)
  }

  pub fn complete_item(&self) {
    self.pending_count.fetch_sub(1, AtomicOrdering::SeqCst);
  }

  pub fn stop(&self) {
    self.stop.store(true, AtomicOrdering::SeqCst);
  }
}

impl<T> OrderedQueueIter<T>
where
  T: Send,
{
  fn pending_count(&self) -> usize {
    self.pending_count.load(AtomicOrdering::SeqCst)
  }

  fn is_stop(&self) -> bool {
    self.stop.load(AtomicOrdering::SeqCst)
  }
}

impl<T> Iterator for OrderedQueueIter<T>
where
  T: Send,
{
  type Item = Ordered<T>;
  fn next(&mut self) -> Option<Ordered<T>> {
    loop {
      if self.is_stop() {
        return None;
      }

      while let Ok(ordered_work) = self.receiver.try_recv() {
        self.receive_buffer.push(ordered_work)
      }

      if let Some(ordered_work) = self.receive_buffer.pop() {
        return Some(ordered_work);
      } else {
        if self.pending_count() == 0 {
          return None;
        } else {
          thread::yield_now();
        }
      }
    }
  }
}
