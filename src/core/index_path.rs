use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct IndexPath {
  pub indices: Vec<usize>,
  // Should use Arc<Vec<usize>>
  //pub child_index: 0, // All children should share parent index
}

impl IndexPath {
  pub fn new(indices: Vec<usize>) -> IndexPath {
    IndexPath { indices }
  }

  pub fn adding(&self, index: usize) -> IndexPath {
    let mut indices = self.indices.clone();
    indices.push(index);
    IndexPath::new(indices)
  }

  pub fn push(&mut self, index: usize) {
    self.indices.push(index);
  }

  pub fn increment_last(&mut self) {
    *self.indices.last_mut().unwrap() += 1;
  }

  pub fn pop(&mut self) -> Option<usize> {
    self.indices.pop()
  }

  pub fn len(&self) -> usize {
    self.indices.len()
  }

  pub fn is_empty(&self) -> bool {
    self.indices.is_empty()
  }
}

impl PartialEq for IndexPath {
  fn eq(&self, o: &Self) -> bool {
    self.indices.eq(&o.indices)
  }
}

impl Eq for IndexPath {}

impl PartialOrd for IndexPath {
  fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
    o.indices.partial_cmp(&self.indices)
  }
}

impl Ord for IndexPath {
  fn cmp(&self, o: &Self) -> Ordering {
    o.indices.cmp(&self.indices)
  }
}
