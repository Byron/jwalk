use crossbeam::channel::{self, Receiver, SendError, Sender};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::thread;

use super::*;

#[derive(Clone)]
pub(crate) struct WorkQueue<D>
where
  D: Delegate,
{
  sender: Sender<OrderedWork<D>>,
  work_count: Arc<AtomicUsize>,
  stop_now: Arc<AtomicBool>,
}

struct OrderedWork<D>
where
  D: Delegate,
{
  index_path: IndexPath,
  work: D::Work,
}

pub(crate) struct WorkQueueIter<D>
where
  D: Delegate,
{
  receiver: Receiver<OrderedWork<D>>,
  receive_buffer: BinaryHeap<OrderedWork<D>>,
  work_count: Arc<AtomicUsize>,
  stop_now: Arc<AtomicBool>,
}

pub(crate) fn new_work_queue<D>() -> (WorkQueue<D>, WorkQueueIter<D>)
where
  D: Delegate,
{
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

impl<D> WorkQueue<D>
where
  D: Delegate,
{
  pub fn push(
    &self,
    work: D::Work,
    index_path: IndexPath,
  ) -> std::result::Result<(), SendError<D::Work>> {
    self.work_count.fetch_add(1, AtomicOrdering::SeqCst);
    if let Err(err) = self.sender.send(OrderedWork { work, index_path }) {
      Err(SendError(err.0.work))
    } else {
      Ok(())
    }
  }

  pub fn completed_work(&self) {
    self.work_count.fetch_sub(1, AtomicOrdering::SeqCst);
  }

  pub fn stop_now(&self) {
    self.stop_now.store(true, AtomicOrdering::SeqCst);
  }
}

impl<D> WorkQueueIter<D>
where
  D: Delegate,
{
  fn work_count(&self) -> usize {
    self.work_count.load(AtomicOrdering::SeqCst)
  }

  fn is_stop_now(&self) -> bool {
    self.stop_now.load(AtomicOrdering::SeqCst)
  }
}

impl<D> Iterator for WorkQueueIter<D>
where
  D: Delegate,
{
  type Item = D::Work;
  fn next(&mut self) -> Option<D::Work> {
    loop {
      if self.is_stop_now() {
        return None;
      }

      while let Ok(ordered_work) = self.receiver.try_recv() {
        self.receive_buffer.push(ordered_work)
      }

      if let Some(ordered_work) = self.receive_buffer.pop() {
        return Some(ordered_work.work);
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

impl<D> PartialEq for OrderedWork<D>
where
  D: Delegate,
{
  fn eq(&self, o: &Self) -> bool {
    self.index_path.eq(&o.index_path)
  }
}

impl<D> Eq for OrderedWork<D> where D: Delegate {}

impl<D> PartialOrd for OrderedWork<D>
where
  D: Delegate,
{
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.index_path.partial_cmp(&self.index_path)
  }
}

impl<D> Ord for OrderedWork<D>
where
  D: Delegate,
{
  fn cmp(&self, o: &Self) -> Ordering {
    o.index_path.cmp(&self.index_path)
  }
}
