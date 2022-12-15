//! Ordered queue backed by a channel.

use crossbeam::channel::{self, Receiver, SendError, Sender, TryRecvError};
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::thread;

use super::*;

pub(crate) struct OrderedQueue<T>
where
    T: Send,
{
    sender: Sender<Ordered<T>>,
    pending_count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
}

pub enum Ordering {
    Relaxed,
    Strict,
}

pub struct OrderedQueueIter<T>
where
    T: Send,
{
    ordering: Ordering,
    stop: Arc<AtomicBool>,
    receiver: Receiver<Ordered<T>>,
    receive_buffer: BinaryHeap<Ordered<T>>,
    pending_count: Arc<AtomicUsize>,
    ordered_matcher: OrderedMatcher,
}

struct OrderedMatcher {
    looking_for: IndexPath,
    child_count_stack: Vec<usize>,
}

pub(crate) fn new_ordered_queue<T>(
    stop: Arc<AtomicBool>,
    ordering: Ordering,
) -> (OrderedQueue<T>, OrderedQueueIter<T>)
where
    T: Send,
{
    let pending_count = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = channel::unbounded();
    (
        OrderedQueue {
            sender,
            pending_count: pending_count.clone(),
            stop: stop.clone(),
        },
        OrderedQueueIter {
            ordering,
            receiver,
            ordered_matcher: OrderedMatcher::default(),
            receive_buffer: BinaryHeap::new(),
            pending_count,
            stop,
        },
    )
}

impl<T> OrderedQueue<T>
where
    T: Send,
{
    pub fn push(&self, ordered: Ordered<T>) -> Result<(), SendError<Ordered<T>>> {
        self.pending_count.fetch_add(1, AtomicOrdering::SeqCst);
        self.sender.send(ordered)
    }

    pub fn complete_item(&self) {
        self.pending_count.fetch_sub(1, AtomicOrdering::SeqCst);
    }
}

impl<T> Clone for OrderedQueue<T>
where
    T: Send,
{
    fn clone(&self) -> Self {
        OrderedQueue {
            sender: self.sender.clone(),
            pending_count: self.pending_count.clone(),
            stop: self.stop.clone(),
        }
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

    fn try_next_relaxed(&mut self) -> Result<Ordered<T>, TryRecvError> {
        if self.is_stop() {
            return Err(TryRecvError::Disconnected);
        }

        while let Ok(ordered_work) = self.receiver.try_recv() {
            self.receive_buffer.push(ordered_work)
        }

        if let Some(ordered_work) = self.receive_buffer.pop() {
            Ok(ordered_work)
        } else if self.pending_count() == 0 {
            Err(TryRecvError::Disconnected)
        } else {
            Err(TryRecvError::Empty)
        }
    }

    fn try_next_strict(&mut self) -> Result<Ordered<T>, TryRecvError> {
        let looking_for = &self.ordered_matcher.looking_for;

        loop {
            if self.is_stop() {
                return Err(TryRecvError::Disconnected);
            }

            let top_ordered = self.receive_buffer.peek();
            if let Some(top_ordered) = top_ordered {
                if top_ordered.index_path.eq(looking_for) {
                    break;
                }
            }

            if self.ordered_matcher.is_none() {
                return Err(TryRecvError::Disconnected);
            }

            match self.receiver.try_recv() {
                Ok(ordered) => {
                    self.receive_buffer.push(ordered);
                }
                Err(err) => match err {
                    TryRecvError::Empty => thread::yield_now(),
                    TryRecvError::Disconnected => break,
                },
            }
        }

        let ordered = self.receive_buffer.pop().unwrap();
        self.ordered_matcher.advance_past(&ordered);
        Ok(ordered)
    }
}

impl<T> Iterator for OrderedQueueIter<T>
where
    T: Send,
{
    type Item = Ordered<T>;
    fn next(&mut self) -> Option<Ordered<T>> {
        loop {
            let try_next = match self.ordering {
                Ordering::Relaxed => self.try_next_relaxed(),
                Ordering::Strict => self.try_next_strict(),
            };
            match try_next {
                Ok(next) => {
                    return Some(next);
                }
                Err(err) => match err {
                    TryRecvError::Empty => thread::yield_now(),
                    TryRecvError::Disconnected => return None,
                },
            }
        }
    }
}

impl OrderedMatcher {
    fn is_none(&self) -> bool {
        self.looking_for.is_empty()
    }

    fn decrement_remaining_children(&mut self) {
        *self.child_count_stack.last_mut().unwrap() -= 1;
    }

    fn advance_past<T>(&mut self, ordered: &Ordered<T>) {
        self.decrement_remaining_children();

        if ordered.child_count > 0 {
            self.looking_for.push(0);
            self.child_count_stack.push(ordered.child_count);
        } else {
            self.looking_for.increment_last();
            while !self.child_count_stack.is_empty() && *self.child_count_stack.last().unwrap() == 0
            {
                self.looking_for.pop();
                self.child_count_stack.pop();
                if !self.looking_for.is_empty() {
                    self.looking_for.increment_last();
                }
            }
        }
    }
}

impl Default for OrderedMatcher {
    fn default() -> OrderedMatcher {
        OrderedMatcher {
            looking_for: IndexPath::new(vec![0]),
            child_count_stack: vec![1],
        }
    }
}
