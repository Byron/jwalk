/*! Provides a more flexible walk function suitable for arbitrary sorting and filtering.

# Example
Recursively iterate over the "foo" directory and print each entry's path:

```no_run
use jwalk::core::walk;

# fn main() {
let dir_list_iter = walk(
  // Directory to walk
  "foo",
  // Initial state value (unused in this example).
  0,
  // Sort, filter, maintain per directory state.
  |path, state, mut entries| {
    entries.sort_by(|a, b| a.file_name().cmp(b.file_name()));
    (state, entries)
  },
  // Continue walk on any error
  |path, error| true,
);

for mut each_dir_list in dir_list_iter {
  for each_entry in each_dir_list.contents.iter() {
    each_dir_list.path.push(each_entry.file_name());
    println!("{}", each_dir_list.path.display());
    each_dir_list.path.pop();
  }
}
# }
```
*/

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
/// The returned iterator yields on `DirList` at a time.
pub fn walk<P, S, F, H>(
  path: P,
  state: S,
  process_entries: F,
  handle_error: H,
) -> ResultsQueueIter<S>
where
  P: AsRef<Path>,
  S: Send + Clone + 'static,
  F: Fn(&Path, S, Vec<DirEntry>) -> (S, Vec<DirEntry>) + Send + Sync + 'static,
  H: Fn(&Path, &Error) -> bool + Send + Sync + 'static,
{
  let (results_queue, results_iterator) = new_results_queue();
  let path = path.as_ref().to_owned();

  rayon::spawn(move || {
    let (work_queue, work_iterator) = new_work_queue();

    let work_context = ReadDirWorkContext {
      work_queue,
      results_queue,
      handle_error: Arc::new(handle_error),
      process_entries: Arc::new(process_entries),
    };

    work_context
      .push_work(ReadDirWork::new(
        path.to_path_buf(),
        IndexPath::with_vec(vec![0]),
        state,
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

pub struct DirList<S> {
  pub state: S,
  pub path: PathBuf,
  pub index_path: IndexPath,
  pub contents: Vec<DirEntry>,
  pub contents_error: Option<Error>,
  pub(crate) scheduled_read_dirs: usize,
}

pub struct DirListIter<S> {
  pub state: S,
  pub path: Arc<PathBuf>,
  pub contents: vec::IntoIter<DirEntry>,
}

pub(crate) struct ReadDirWork<S> {
  dir_state: S,
  dir_path: PathBuf,
  dir_index_path: IndexPath,
}

#[derive(Clone)]
struct ReadDirWorkContext<S>
where
  S: Clone,
{
  work_queue: WorkQueue<S>,
  results_queue: ResultsQueue<S>,
  handle_error: Arc<Fn(&Path, &Error) -> bool + Send + Sync + 'static>,
  process_entries: Arc<Fn(&Path, S, Vec<DirEntry>) -> (S, Vec<DirEntry>) + Send + Sync + 'static>,
}

fn process_work<S>(work: ReadDirWork<S>, work_context: &ReadDirWorkContext<S>)
where
  S: Clone,
{
  let mut read_dir_value = work.read_value(work_context);
  let generated_read_dir_works = read_dir_value.generate_read_dir_works();

  if work_context.push_result(read_dir_value).is_err() {
    work_context.stop_now();
    work_context.completed_work();
    return;
  }

  for each in generated_read_dir_works {
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

impl<S> DirList<S>
where
  S: Clone,
{
  pub fn depth(&self) -> usize {
    self.index_path.len()
  }

  fn generate_read_dir_works(&mut self) -> Vec<ReadDirWork<S>> {
    let mut dir_index = 0;
    let read_dir_works: Vec<_> = self
      .contents
      .iter()
      .filter_map(|each| {
        if each.has_read_dir {
          let mut work_path = self.path.clone();
          let mut work_index_path = self.index_path.clone();
          work_path.push(each.file_name());
          work_index_path.push(dir_index);
          dir_index += 1;
          Some(ReadDirWork::new(
            work_path,
            work_index_path,
            self.state.clone(),
          ))
        } else {
          None
        }
      })
      .collect();

    self.scheduled_read_dirs = read_dir_works.len();

    read_dir_works
  }
}

impl<S> IntoIterator for DirList<S>
where
  S: Default,
{
  type Item = DirEntry;
  type IntoIter = DirListIter<S>;

  fn into_iter(self) -> DirListIter<S> {
    DirListIter {
      state: self.state,
      path: Arc::new(self.path),
      contents: self.contents.into_iter(),
    }
  }
}

impl<S> PartialEq for DirList<S> {
  fn eq(&self, o: &Self) -> bool {
    self.index_path.eq(&o.index_path)
  }
}

impl<S> Eq for DirList<S> {}

impl<S> PartialOrd for DirList<S> {
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.index_path.partial_cmp(&self.index_path)
  }
}

impl<S> Ord for DirList<S> {
  fn cmp(&self, o: &Self) -> Ordering {
    o.index_path.cmp(&self.index_path)
  }
}

impl<S> Iterator for DirListIter<S> {
  type Item = DirEntry;
  fn next(&mut self) -> Option<DirEntry> {
    self.contents.next()
  }
}

impl<S> ReadDirWork<S>
where
  S: Clone,
{
  fn new(dir_path: PathBuf, dir_index_path: IndexPath, dir_state: S) -> ReadDirWork<S> {
    ReadDirWork {
      dir_path,
      dir_index_path,
      dir_state,
    }
  }

  fn read_value(self, work_context: &ReadDirWorkContext<S>) -> DirList<S> {
    let ReadDirWork {
      dir_path,
      dir_index_path,
      dir_state,
    } = self;

    let read_dir = match fs::read_dir(&dir_path) {
      Ok(read_dir) => read_dir,
      Err(err) => {
        if !work_context.handle_error(&dir_path, &err) {
          work_context.stop_now();
        }
        return DirList {
          state: dir_state.clone(),
          path: dir_path,
          index_path: dir_index_path,
          contents: Vec::new(),
          contents_error: Some(err),
          scheduled_read_dirs: 0,
        };
      }
    };

    let (dir_state, dir_entries) = (work_context.process_entries)(
      &dir_path,
      dir_state,
      map_entries(&dir_path, read_dir, work_context),
    );

    DirList {
      state: dir_state.clone(),
      path: dir_path,
      index_path: dir_index_path,
      contents: dir_entries,
      contents_error: None,
      scheduled_read_dirs: 0,
    }
  }
}

fn map_entries<S>(
  dir_path: &Path,
  read_dir: fs::ReadDir,
  work_context: &ReadDirWorkContext<S>,
) -> Vec<DirEntry>
where
  S: Clone,
{
  read_dir
    .filter_map(|entry_result| {
      let entry = match entry_result {
        Ok(entry) => entry,
        Err(err) => {
          if !work_context.handle_error(&dir_path, &err) {
            work_context.stop_now();
          }
          return None;
        }
      };

      let file_type = match entry.file_type() {
        Ok(file_type) => file_type,
        Err(err) => {
          if !work_context.handle_error(&entry.path(), &err) {
            work_context.stop_now();
          }
          return None;
        }
      };

      Some(DirEntry::new(entry.file_name(), file_type))
    })
    .collect()
}

impl<S> ReadDirWorkContext<S>
where
  S: Clone,
{
  fn handle_error(&self, path: &Path, error: &Error) -> bool {
    (self.handle_error)(path, error)
  }

  fn stop_now(&self) {
    self.work_queue.stop_now()
  }

  fn push_work(&self, work: ReadDirWork<S>) -> Result<(), SendError<ReadDirWork<S>>> {
    self.work_queue.push(work)
  }

  fn completed_work(&self) {
    self.work_queue.completed_work()
  }

  fn push_result(&self, result: DirList<S>) -> Result<(), SendError<DirList<S>>> {
    self.results_queue.push(result)
  }
}

impl<S> PartialEq for ReadDirWork<S> {
  fn eq(&self, o: &Self) -> bool {
    self.dir_index_path.eq(&o.dir_index_path)
  }
}

impl<S> Eq for ReadDirWork<S> {}

impl<S> PartialOrd for ReadDirWork<S> {
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.dir_index_path.partial_cmp(&self.dir_index_path)
  }
}

impl<S> Ord for ReadDirWork<S> {
  fn cmp(&self, o: &Self) -> Ordering {
    o.dir_index_path.cmp(&self.dir_index_path)
  }
}
