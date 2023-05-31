use std::marker::PhantomData;

pub struct CompactList<T> {
    storage: Vec<Entry<T>>,
}

impl<T> Default for CompactList<T> {
    fn default() -> Self {
        CompactList { storage: Vec::new() }
    }
}

struct Entry<T> {
    generation: u32,
    value: Option<T>,
}

impl<T> From<T> for Entry<T> {
    fn from(value: T) -> Self {
        Entry {
            generation: 0,
            value: Some(value),
        }
    }
}

impl<T> Entry<T> {
    fn is_empty(&self) -> bool {
        self.value.is_none()
    }

    fn place(&mut self, value: T) {
        self.value = Some(value);
    }

    fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        self.value.as_mut()
    }

    fn is_generation(&self, generation: u32) -> bool {
        self.generation == generation
    }

    fn increment(&mut self) {
        self.generation += 1;
    }

    fn remove(&mut self) {
        self.increment();
        self.value = None;
    }

    fn take(&mut self) -> Option<T> {
        self.increment();
        self.value.take()
    }
}

#[derive(PartialOrd, PartialEq, Hash)]
pub struct Handle<T> {
    index: usize,
    generation: u32,
    phantom: PhantomData<T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Handle {
            index: self.index,
            generation: self.generation,
            phantom: PhantomData::default(),
        }
    }
}

impl<T> Copy for Handle<T> {}

impl<T> CompactList<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, value: T) -> Handle<T> {
        match self.storage.iter_mut()
            .enumerate()
            .find(|(_, entry)| { entry.is_empty() }) {
            None => {
                let index = self.storage.len();
                let entry = Entry::from(value);
                self.storage.push(entry);
                Handle {
                    index,
                    generation: 0,
                    phantom: Default::default(),
                }
            }
            Some((index, entry)) => {
                entry.place(value);
                Handle {
                    index,
                    generation: entry.generation,
                    phantom: Default::default(),
                }
            }
        }
    }

    fn get_entry_mut(&mut self, handle: Handle<T>) -> Option<&mut Entry<T>> {
        self.storage.get_mut(handle.index)
            .filter(|entry| entry.is_generation(handle.generation))
    }

    fn get_entry(&self, handle: Handle<T>) -> Option<&Entry<T>> {
        self.storage.get(handle.index)
            .filter(|entry| entry.is_generation(handle.generation))
    }

    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.get_entry(handle).and_then(Entry::get)
    }

    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.get_entry_mut(handle).and_then(Entry::get_mut)
    }

    pub fn remove(&mut self, handle: Handle<T>) {
        if let Some(entry) = self.get_entry_mut(handle) {
            entry.remove();
        }
    }

    pub fn take(&mut self, handle: Handle<T>) -> Option<T> {
        self.get_entry_mut(handle).and_then(Entry::take)
    }
}
