use std::cmp::Ordering;

use super::*;

pub struct WorkResults<D>
where
  D: Delegate,
{
  results: Vec<WorkResultsItem<D>>,
  pub(crate) scheduled_work: usize,
  pub(crate) index_path: IndexPath,
}

impl<D> WorkResults<D>
where
  D: Delegate,
{
  fn push_item(&mut self, item: D::Item) {
    self.results.push(WorkResultsItem::Item(item));
  }

  fn push_work(&mut self, work: D::Work) {
    self.results.push(WorkResultsItem::Work(work));
  }
}

impl<D> PartialEq for WorkResults<D>
where
  D: Delegate,
{
  fn eq(&self, o: &Self) -> bool {
    self.index_path.eq(&o.index_path)
  }
}

impl<D> Eq for WorkResults<D> where D: Delegate {}

impl<D> PartialOrd for WorkResults<D>
where
  D: Delegate,
{
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.index_path.partial_cmp(&self.index_path)
  }
}

impl<D> Ord for WorkResults<D>
where
  D: Delegate,
{
  fn cmp(&self, o: &Self) -> Ordering {
    o.index_path.cmp(&self.index_path)
  }
}

enum WorkResultsItem<D>
where
  D: Delegate,
{
  Item(D::Item),
  Work(D::Work),
}
