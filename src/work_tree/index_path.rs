use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct IndexPath(pub Vec<usize>);

impl IndexPath {
  pub fn new(vec: Vec<usize>) -> IndexPath {
    IndexPath(vec)
  }

  pub fn push(&mut self, index: usize) {
    self.0.push(index);
  }

  pub fn increment_last(&mut self) {
    *self.0.last_mut().unwrap() += 1;
  }

  pub fn pop(&mut self) -> Option<usize> {
    self.0.pop()
  }

  pub fn len(&self) -> usize {
    self.0.len()
  }

  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }
}

impl PartialEq for IndexPath {
  fn eq(&self, o: &Self) -> bool {
    self.0.eq(&o.0)
  }
}

impl Eq for IndexPath {}

impl PartialOrd for IndexPath {
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    self.0.partial_cmp(&o.0)
  }
}

impl Ord for IndexPath {
  fn cmp(&self, o: &Self) -> Ordering {
    self.0.cmp(&o.0)
  }
}
