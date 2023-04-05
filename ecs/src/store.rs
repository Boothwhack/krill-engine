use crate::sparse_vec::SparseVec;
use crate::world::{EntityId, Generation};

pub struct ComponentStore<T> {
    components: SparseVec<(Generation, T)>,
}

impl<T> Default for ComponentStore<T> {
    fn default() -> Self {
        Self { components: SparseVec::new() }
    }
}

impl<T> ComponentStore<T> {
    pub fn get(&self, entity: EntityId) -> Option<&T> {
        match self.components.get(entity.index) {
            Some((generation, component)) if generation == &entity.generation => Some(component),
            _ => None,
        }
    }

    pub fn put(&mut self, entity: EntityId, component: T) {
        self.components.set(entity.index, (entity.generation, component));
    }

    pub fn remove(&mut self, entity: EntityId) -> Option<T> {
        self.components
            .remove_if(entity.index, |(generation, _)| *generation == entity.generation)
            .map(|(_, component)| component)
    }

    pub fn has(&self, entity: EntityId) -> bool {
        self.get(entity).is_some()
    }
}
