use std::cmp::Ordering;

use super::IndexPath;

pub struct Ordered<T> {
  pub value: T,
  pub index_path: IndexPath,
}

impl<T> Ordered<T> {
  pub fn new(value: T, index_path: IndexPath) -> Ordered<T> {
    Ordered { value, index_path }
  }
}

impl<T> PartialEq for Ordered<T> {
  fn eq(&self, o: &Self) -> bool {
    self.index_path.eq(&o.index_path)
  }
}

impl<T> Eq for Ordered<T> {}

impl<T> PartialOrd for Ordered<T> {
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    self.index_path.partial_cmp(&o.index_path)
  }
}

impl<T> Ord for Ordered<T> {
  fn cmp(&self, o: &Self) -> Ordering {
    self.index_path.cmp(&o.index_path)
  }
}
