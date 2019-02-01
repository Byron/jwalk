#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod index_path;
mod ordered;
mod ordered_queue;
mod work_queue;
mod work_results;
mod work_results_queue;

use crossbeam::channel::{self, Receiver, SendError, Sender};
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use std::fs::{self, FileType};
use std::io::Error;
use std::path::{Path, PathBuf};

use index_path::*;
use ordered::*;
use ordered_queue::*;
use work_queue::*;
use work_results::*;
use work_results_queue::*;

pub trait Delegate: Clone + Send
where
  Self::Work: Send,
  Self::Item: Send,
{
  type Work;
  type Item;
  fn perform_work(&self, work: Self::Work, work_context: &WorkContext<Self>);
}

pub enum WorkResult<D>
where
  D: Delegate,
{
  Item(D::Item),
  Work(D::Work),
}

pub fn process<D>(work: Vec<D::Work>, delegate: D) -> OrderedQueueIter<D::Item>
where
  D: Delegate + 'static,
  D::Work: Clone, // WHY iS THIS NEEDED? Work shouldn't be cloned
  D::Item: Clone, // WHY iS THIS NEEDED? Work shouldn't be cloned
{
  let (item_queue, item_queue_iter) = new_ordered_queue();

  rayon::spawn(move || {
    let (work_queue, work_iterator) = new_ordered_queue();

    work.into_iter().enumerate().for_each(|(i, work)| {
      work_queue
        .push(Ordered::new(work, IndexPath::new(vec![i])))
        .unwrap();
    });

    let index_path = IndexPath::new(Vec::new());
    let index = 0;
    let work_context = WorkContext {
      item_queue,
      work_queue,
      index_path,
      index,
    };

    work_iterator.par_bridge().for_each_with(
      (delegate, work_context),
      |(delegate, work_context), work| {
        perform_work(delegate, work, work_context);
      },
    );
  });

  item_queue_iter
}

fn perform_work<D>(delegate: &D, orderd_work: Ordered<D::Work>, work_context: &mut WorkContext<D>)
where
  D: Delegate,
{
  let Ordered { value, index_path } = orderd_work;

  work_context.index_path = index_path;
  work_context.index = 0;
  delegate.perform_work(value, work_context);

  // Don't push

  /*
  let mut dir_list = work.read_dir_list(work_context);
  let new_work = dir_list.new_work();

  if work_context.push_result(dir_list).is_err() {
    work_context.stop_now();
    work_context.completed_work();
    return;
  }

  for each in new_work {
    if work_context.push_work(each).is_err() {
      work_context.stop_now();
      return;
    }
  }*/

  work_context.work_queue.complete_item()
}

#[derive(Clone)]
pub struct WorkContext<D>
where
  D: Delegate,
{
  item_queue: OrderedQueue<D::Item>,
  work_queue: OrderedQueue<D::Work>,
  index_path: IndexPath,
  index: usize,
}

pub struct WorkPlaceholder {
  index_path: IndexPath,
  remaining_items: usize,
}

impl<D> WorkContext<D>
where
  D: Delegate,
{
  fn next_index_path(&mut self) -> IndexPath {
    let index_path = self.index_path.adding(self.index);
    self.index += 1;
    index_path
  }

  fn push_work(&mut self, work: D::Work) -> Result<(), SendError<Ordered<D::Work>>> {
    // When work is pushed also need to push placeholder item to items_queue.
    // This placeholder is used to track how many items the new work generates.
    let index_path = self.next_index_path();
    let ordered_work = Ordered::new(work, index_path);
    self.work_queue.push(ordered_work)
    self.item_queue.push()
  }

  fn push_item(&mut self, item: D::Item) -> Result<(), SendError<Ordered<D::Item>>> {
    let index_path = self.next_index_path();
    let ordered_item = Ordered::new(item, index_path);
    self.item_queue.push(ordered_item)
  }
}
