pub struct SparseVec<T> {
    storage: Vec<Option<T>>,
}

impl<T> Default for SparseVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SparseVec<T> {
    pub fn new() -> Self {
        SparseVec { storage: Vec::new() }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.storage.get(index)?.as_ref()
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.storage.get_mut(index)?.as_mut()
    }

    pub fn set(&mut self, index: usize, value: T) -> Option<T> {
        if index >= self.storage.len() {
            self.storage.resize_with(index + 1, || None);
        }

        self.storage[index].replace(value)
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        self.storage.get_mut(index)?.take()
    }

    pub fn remove_if<F: FnOnce(&T) -> bool>(&mut self, index: usize, predicate: F) -> Option<T> {
        let slot = self.storage.get_mut(index)?;
        match slot {
            Some(component) if predicate(component) => slot.take(),
            _ => None,
        }
    }

    pub fn contains(&self, index: usize) -> bool {
        self.get(index).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item=&T> {
        self.storage.iter()
            .filter_map(Option::as_ref)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut T> {
        self.storage.iter_mut()
            .filter_map(Option::as_mut)
    }
}
