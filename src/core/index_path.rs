use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct IndexPath {
    pub indices: im::Vector<usize>,
}

impl IndexPath {
    pub fn new(indices: im::Vector<usize>) -> IndexPath {
        IndexPath { indices }
    }

    pub fn adding(&self, index: usize) -> IndexPath {
        let mut indices = self.indices.clone();
        indices.push_back(index);
        IndexPath::new(indices)
    }

    pub fn push(&mut self, index: usize) {
        self.indices.push_back(index);
    }

    pub fn increment_last(&mut self) {
        *self.indices.back_mut().unwrap() += 1;
    }

    pub fn pop(&mut self) -> Option<usize> {
        self.indices.pop_back()
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
