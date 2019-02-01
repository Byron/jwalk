/*! A flexible walk function suitable for arbitrary sorting/filtering.

# Example
Recursively iterate over the "foo" directory and print each entry's path:

```no_run
use jwalk::core::{walk, Delegate, DirEntry};
use std::io::Error;
use std::path::Path;

# fn main() {
#[derive(Clone)]
pub struct MyDelegate {}

impl Delegate for MyDelegate {
  type State = usize;
  fn handle_error(&self, path: &Path, error: &Error) -> bool {
    eprintln!("{} {}", path.display(), error);
    true
  }
  fn process_entries(&self, path: &Path, state: Self::State, mut entries: Vec<DirEntry>) -> (Self::State, Vec<DirEntry>) {
    entries.sort_by(|a, b| a.file_name().cmp(b.file_name()));
    (state, entries)
  }
}

for mut dir_list in walk("foo", None, MyDelegate {}) {
  for entry in dir_list.contents.iter() {
    dir_list.path.push(entry.file_name());
    println!("{}", dir_list.path.display());
    dir_list.path.pop();
  }
}
# }
```
*/

mod delegate;
mod index_path;
mod results_queue;
mod work_queue;
  
use crossbeam::channel::SendError;
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::fs::{self, FileType};
use std::io::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::vec;

use index_path::*;
use results_queue::*;
use work_queue::*;

pub use delegate::{DefaultDelegate, Delegate};
pub use results_queue::ResultsQueueIter;

/// Recursively walk the given path.
///
/// Directories are processed one `fs::read_dir` at a time. The `read_dir`
/// generates a `DirList` which is a list of `DirEntry` and other directory
/// context such as the dir's `path` and the parameterized `state` value of the
/// parent directory.
///
/// Once a `DirList` is created it is passed to the `process_entries` callback.
/// This callback is provided with the parent dir's `state`, the current dir's
/// `path` and the current dir's entries. It is responsible for returning a new
/// state and a final set of sorted/filtered entries.
///
/// The major intention if the `state` parameter is to support .gitignore style
/// filtering. When each directory is processed it can update ignore state,
/// filter entries based on that state. And then that cloned state is later
/// passed in when processing child `DirList`s.
///
/// Returns iterator of `DirList`s.
pub fn walk<P, D>(path: P, state: Option<D::State>, delegate: D) -> ResultsQueueIter<D>
where
  P: AsRef<Path>,
  D: Delegate + 'static,
  D::State: Clone + Send + Default,
{
  let (results_queue, results_iterator) = new_results_queue();
  let path = path.as_ref().to_owned();

  rayon::spawn(move || {
    let (work_queue, work_iterator) = new_work_queue();

    let work_context = WorkContext {
      delegate,
      work_queue,
      results_queue,
    };

    work_context
      .push_work(Work::new(
        path.to_path_buf(),
        IndexPath::with_vec(vec![0]),
        state.unwrap_or(D::State::default()),
      ))
      .unwrap();

    work_iterator
      .par_bridge()
      .for_each_with(work_context, |work_context, work| {
        process_work(work, &work_context);
      });
  });

  results_iterator
}

pub struct DirEntry {
  pub file_name: OsString,
  pub file_type: FileType,
  pub(crate) has_read_dir: bool,
}

pub struct DirList<D>
where
  D: Delegate,
{
  pub state: D::State,
  pub path: PathBuf,
  pub index_path: IndexPath,
  pub contents: Vec<DirEntry>,
  pub contents_error: Option<Error>,
  pub(crate) scheduled_read_dirs: usize,
}

pub struct DirListIter<D>
where
  D: Delegate,
{
  pub state: D::State,
  pub path: Arc<PathBuf>,
  pub contents: vec::IntoIter<DirEntry>,
}

pub(crate) struct Work<D>
where
  D: Delegate,
{
  state: D::State,
  path: PathBuf,
  index_path: IndexPath,
}

#[derive(Clone)]
struct WorkContext<D>
where
  D: Delegate,
  D::State: Clone + Send,
{
  delegate: D,
  work_queue: WorkQueue<D>,
  results_queue: ResultsQueue<D>,
}

fn process_work<D>(work: Work<D>, work_context: &WorkContext<D>)
where
  D: Delegate + Clone,
  D::State: Clone + Send,
{
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
  }

  work_context.completed_work()
}

impl DirEntry {
  fn new(file_name: OsString, file_type: FileType) -> DirEntry {
    DirEntry {
      file_name,
      file_type,
      has_read_dir: file_type.is_dir(),
    }
  }

  pub fn file_type(&self) -> &FileType {
    &self.file_type
  }

  pub fn file_name(&self) -> &OsStr {
    &self.file_name
  }
}

impl<D> DirList<D>
where
  D: Delegate,
  D::State: Clone + Send,
{
  pub fn depth(&self) -> usize {
    self.index_path.len()
  }

  fn new_work(&mut self) -> Vec<Work<D>> {
    let mut dir_index = 0;
    let new_work: Vec<_> = self
      .contents
      .iter()
      .filter_map(|each| {
        if each.has_read_dir {
          let mut work_path = self.path.clone();
          let mut work_index_path = self.index_path.clone();
          work_path.push(each.file_name());
          work_index_path.push(dir_index);
          dir_index += 1;
          Some(Work::new(work_path, work_index_path, self.state.clone()))
        } else {
          None
        }
      })
      .collect();

    self.scheduled_read_dirs = new_work.len();

    new_work
  }
}

impl<D> IntoIterator for DirList<D>
where
  D: Delegate,
{
  type Item = DirEntry;
  type IntoIter = DirListIter<D>;

  fn into_iter(self) -> DirListIter<D> {
    DirListIter {
      state: self.state,
      path: Arc::new(self.path),
      contents: self.contents.into_iter(),
    }
  }
}

impl<D> PartialEq for DirList<D>
where
  D: Delegate,
{
  fn eq(&self, o: &Self) -> bool {
    self.index_path.eq(&o.index_path)
  }
}

impl<D> Eq for DirList<D> where D: Delegate {}

impl<D> PartialOrd for DirList<D>
where
  D: Delegate,
{
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.index_path.partial_cmp(&self.index_path)
  }
}

impl<D> Ord for DirList<D>
where
  D: Delegate,
{
  fn cmp(&self, o: &Self) -> Ordering {
    o.index_path.cmp(&self.index_path)
  }
}

impl<D> Iterator for DirListIter<D>
where
  D: Delegate,
{
  type Item = DirEntry;
  fn next(&mut self) -> Option<DirEntry> {
    self.contents.next()
  }
}

impl<D> Work<D>
where
  D: Delegate,
  D::State: Clone + Send,
{
  fn new(path: PathBuf, index_path: IndexPath, state: D::State) -> Work<D> {
    Work {
      state,
      path,
      index_path,
    }
  }

  fn read_dir_list(self, work_context: &WorkContext<D>) -> DirList<D> {
    let Work {
      path,
      index_path,
      state,
    } = self;

    let read_dir = match fs::read_dir(&path) {
      Ok(read_dir) => read_dir,
      Err(err) => {
        if !work_context.delegate.handle_error(&path, &err) {
          work_context.stop_now();
        }
        return DirList {
          state: state.clone(),
          path: path,
          index_path: index_path,
          contents: Vec::new(),
          contents_error: Some(err),
          scheduled_read_dirs: 0,
        };
      }
    };

    let (state, entries) = work_context.delegate.process_entries(
      &path,
      state,
      map_entries(&path, read_dir, work_context),
    );

    DirList {
      state: state,
      path: path,
      index_path: index_path,
      contents: entries,
      contents_error: None,
      scheduled_read_dirs: 0,
    }
  }
}

fn map_entries<D>(
  dir_path: &Path,
  read_dir: fs::ReadDir,
  work_context: &WorkContext<D>,
) -> Vec<DirEntry>
where
  D: Delegate,
  D::State: Clone + Send,
{
  read_dir
    .filter_map(|entry_result| {
      let entry = match entry_result {
        Ok(entry) => entry,
        Err(err) => {
          if !work_context.delegate.handle_error(&dir_path, &err) {
            work_context.stop_now();
          }
          return None;
        }
      };

      let file_type = match entry.file_type() {
        Ok(file_type) => file_type,
        Err(err) => {
          if !work_context.delegate.handle_error(&entry.path(), &err) {
            work_context.stop_now();
          }
          return None;
        }
      };

      Some(DirEntry::new(entry.file_name(), file_type))
    })
    .collect()
}

impl<D> WorkContext<D>
where
  D: Delegate,
  D::State: Clone + Send,
{
  //fn handle_error(&self, path: &Path, error: &Error) -> bool {
  //  (self.handle_error)(path, error)
  //}

  fn stop_now(&self) {
    self.work_queue.stop_now()
  }

  fn push_work(&self, work: Work<D>) -> Result<(), SendError<Work<D>>> {
    self.work_queue.push(work)
  }

  fn completed_work(&self) {
    self.work_queue.completed_work()
  }

  fn push_result(&self, result: DirList<D>) -> Result<(), SendError<DirList<D>>> {
    self.results_queue.push(result)
  }
}

impl<D> PartialEq for Work<D>
where
  D: Delegate,
{
  fn eq(&self, o: &Self) -> bool {
    self.index_path.eq(&o.index_path)
  }
}

impl<D> Eq for Work<D> where D: Delegate {}

impl<D> PartialOrd for Work<D>
where
  D: Delegate,
{
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.index_path.partial_cmp(&self.index_path)
  }
}

impl<D> Ord for Work<D>
where
  D: Delegate,
{
  fn cmp(&self, o: &Self) -> Ordering {
    o.index_path.cmp(&self.index_path)
  }
}
