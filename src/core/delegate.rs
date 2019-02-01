use rayon::prelude::*;
use std::fs::{self, FileType};
use std::io::{Error, Result};
use std::path::{Path, PathBuf};

//use super::WorkContext;
use crate::core::DirEntry;

pub struct WorkContext<D>
where
  D: Delegate,
{
  items: Vec<WorkContextItem<D>>,
}

enum WorkContextItem<D>
where
  D: Delegate,
{
  Item(D::Item),
  Work(D::Work),
}

impl<D> WorkContext<D>
where
  D: Delegate,
{
  fn send_item(&mut self, item: D::Item) {
    self.items.push(WorkContextItem::Item(item));
  }
  fn schedule_work(&mut self, work: D::Work) {
    self.items.push(WorkContextItem::Work(work));
  }
}

pub trait Delegate: Clone + Send {
  type State; // don't need... include state in "work"
  type Item;
  type Work;

  fn process_work(&self, work: Self::Work, context: &mut WorkContext<Self>) -> Result<()>;

  fn handle_error(&self, path: &Path, error: &Error) -> bool;
  fn process_entries(
    &self,
    path: &Path,
    state: Self::State,
    entries: Vec<DirEntry>,
  ) -> (Self::State, Vec<DirEntry>);
}

#[derive(Clone)]
pub struct DefaultDelegate {}

impl Delegate for DefaultDelegate {
  type State = usize;
  type Item = PathBuf;
  type Work = PathBuf;

  fn process_work(&self, work: Self::Work, context: &mut WorkContext<Self>) -> Result<()> {
    fs::read_dir(&work)?.for_each(|entry_result| {
      let entry = match entry_result {
        Ok(entry) => entry,
        Err(_) => {
          return;
        }
      };

      let file_type = match entry.file_type() {
        Ok(file_type) => file_type,
        Err(_) => {
          return;
        }
      };

      context.send_item(entry.path());

      if file_type.is_dir() {
        context.schedule_work(entry.path());
      }
    });

    Ok(())
  }

  fn handle_error(&self, path: &Path, error: &Error) -> bool {
    eprintln!("{} {}", path.display(), error);
    true
  }
  fn process_entries(
    &self,
    _path: &Path,
    state: Self::State,
    mut entries: Vec<DirEntry>,
  ) -> (Self::State, Vec<DirEntry>) {
    entries.par_sort_by(|a, b| a.file_name().cmp(b.file_name()));
    (state, entries)
  }
}
