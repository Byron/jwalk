use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use std::fs::{self, DirEntry};
use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::sync::Arc;
use std::thread;

pub fn walk<P: AsRef<Path>>(path: P) -> Receiver<Result<DirEntry>> {
  let (results_tx, results_rx) = mpsc::channel();
  let path = path.as_ref().to_owned();

  rayon::spawn(move || {
    let (work_queue, work_iterator) = new_work_queue();

    work_queue
      .push(Work::new(path))
      .expect("Iterator owned above");

    work_iterator.par_bridge().for_each_with(
      (work_queue, results_tx),
      |(work_queue, results_tx), work| {
        process_work(work, &work_queue, &results_tx);
      },
    );
  });

  results_rx
}

struct Work {
  path: PathBuf,
}

#[derive(Clone)]
struct WorkQueue {
  sender: Sender<Work>,
  work_count: Arc<AtomicUsize>,
  stop_now: Arc<AtomicBool>,
}

struct WorkQueueIterator {
  receiver: Receiver<Work>,
  work_count: Arc<AtomicUsize>,
  stop_now: Arc<AtomicBool>,
}

fn process_work(work: Work, work_queue: &WorkQueue, results_tx: &Sender<Result<DirEntry>>) {
  let read_dir = match fs::read_dir(&work.path) {
    Ok(read_dir) => read_dir,
    Err(err) => {
      if is_to_many_files_open(&err) {
        work_queue
          .push(work)
          .expect("read_dir called by owning iterator");
      }
      work_queue.completed_work_item();
      return;
    }
  };

  for entry_result in read_dir {
    let entry_result = match entry_result {
      Ok(dent) => Ok(dent),
      Err(err) => Err(err),
    };

    let entry = match entry_result {
      Ok(ref entry) => entry,
      Err(_) => {
        break;
      }
    };

    let file_type = match entry.file_type() {
      Ok(file_type) => file_type,
      Err(_) => {
        break;
      }
    };

    let dir_path = if file_type.is_dir() {
      Some(entry.path())
    } else {
      None
    };

    if let Some(dir_path) = dir_path {
      work_queue
        .push(Work::new(dir_path))
        .expect("read_dir called by owning iterator");
    }

    if results_tx.send(entry_result).is_err() {
      work_queue.stop_now();
    }
  }

  work_queue.completed_work_item();
}

fn new_work_queue() -> (WorkQueue, WorkQueueIterator) {
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

fn is_to_many_files_open(error: &Error) -> bool {
  error.raw_os_error() == Some(24)
}

impl Work {
  fn new(path: PathBuf) -> Work {
    Work { path }
  }
}

impl WorkQueue {
  pub fn push(&self, work: Work) -> std::result::Result<(), SendError<Work>> {
    self.work_count.fetch_add(1, AtomicOrdering::SeqCst);
    self.sender.send(work)
  }

  fn completed_work_item(&self) {
    self.work_count.fetch_sub(1, AtomicOrdering::SeqCst);
  }

  fn stop_now(&self) {
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

impl Iterator for WorkQueueIterator {
  type Item = Work;
  fn next(&mut self) -> Option<Work> {
    loop {
      if self.is_stop_now() {
        return None;
      }
      match self.receiver.try_recv() {
        Ok(work) => return Some(work),
        Err(_) => {
          if self.work_count() == 0 {
            return None;
          } else {
            thread::yield_now();
          }
        }
      }
    }
  }
}
