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
  Self::Item: Send,
  Self::Work: Send,
{
  type Item;
  type Work;
  fn perform_work(&self, work: Self::Work, work_context: &WorkContext<Self>);
}

pub enum WorkResult<D>
where
  D: Delegate,
{
  Item(D::Item),
  Work(D::Work),
}

pub fn process<D>(work: Vec<D::Work>, delegate: D) -> WorkResultsQueueIter<D>
where
  D: Delegate + 'static,
  D::Work: Clone, // WHY iS THIS NEEDED? Work shouldn't be cloned
{
  let (work_results_queue, work_results_iterator) = new_work_results_queue();

  rayon::spawn(move || {
    let (work_queue, work_iterator) = new_ordered_queue();

    work.into_iter().enumerate().for_each(|(i, work)| {
      work_queue
        .push(Ordered::new(work, IndexPath::new(vec![i])))
        .unwrap();
    });

    let work_context = WorkContext {
      work_results_queue,
      work_queue,
    };

    work_iterator.par_bridge().for_each_with(
      (delegate, work_context),
      |(delegate, work_context), work| {
        perform_work(delegate, work, work_context);
      },
    );
  });

  work_results_iterator
}

fn perform_work<D>(delegate: &D, orderd_work: Ordered<D::Work>, work_context: &mut WorkContext<D>)
where
  D: Delegate,
{
  let Ordered { value, index_path } = orderd_work;

  //Goal is that when delegate peroforms work it schedules it into the work context in order.
  //this means index_path nees to be associated with work_context so that push_work nad push_result
  //can get automatically assigned index paths

  delegate.perform_work(value, work_context);

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
  work_results_queue: WorkResultsQueue<D>,
  work_queue: OrderedQueue<D::Work>,
}

impl<D> WorkContext<D>
where
  D: Delegate,
{
  //fn push_work(&self, work: D::Work, index_path: IndexPath) -> Result<(), SendError<D::Work>> {
  //self.work_queue.push(work, index_path)
  //}

  fn push_result(&self, result: WorkResults<D>) -> Result<(), SendError<WorkResults<D>>> {
    self.work_results_queue.push(result)
  }
}
