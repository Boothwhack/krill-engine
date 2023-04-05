use std::collections::{HashMap, HashSet};
use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::store::{ComponentStore};

pub type Generation = u32;

#[derive(Copy, Clone)]
pub struct EntityId {
    pub(crate) index: usize,
    pub(crate) generation: Generation,
}

enum EntityState {
    Alive(Generation),
    Dead(Generation),
}

impl EntityState {
    fn is_alive(&self) -> bool {
        matches!(self, EntityState::Alive(_))
    }

    fn is_dead(&self) -> bool {
        !self.is_alive()
    }

    fn generation(&self) -> Generation {
        *match self {
            EntityState::Alive(gen) => gen,
            EntityState::Dead(gen) => gen,
        }
    }

    fn make_alive(&mut self) -> Generation {
        *self = EntityState::Alive(self.generation() + 1);
        self.generation()
    }

    fn make_dead(&mut self) {
        *self = EntityState::Dead(self.generation());
    }

    fn alive_generation(&self) -> Option<Generation> {
        match self {
            EntityState::Alive(generation) => Some(*generation),
            EntityState::Dead(..) => None,
        }
    }
}

impl PartialEq<EntityId> for &EntityState {
    fn eq(&self, other: &EntityId) -> bool {
        matches!(self, EntityState::Alive(generation) if generation == &other.generation)
    }

    fn ne(&self, other: &EntityId) -> bool {
        !self.eq(other)
    }
}

pub struct GenericComponentStore(Box<dyn Any>);

impl GenericComponentStore {
    fn new<C: 'static>() -> GenericComponentStore {
        let store = ComponentStore::<C>::default();
        GenericComponentStore(Box::new(store))
    }

    fn store_for<C: 'static>(&self) -> &ComponentStore<C> {
        self.0.downcast_ref().expect("component type has already been checked")
    }

    fn store_for_mut<C: 'static>(&mut self) -> &mut ComponentStore<C> {
        self.0.downcast_mut().expect("component type has already been checked")
    }
}

#[derive(Default)]
pub struct World {
    entities: Vec<EntityState>,
    components: HashMap<TypeId, RwLock<GenericComponentStore>>,
}

impl World {
    pub fn new_entity(&mut self) -> EntityId {
        for (index, state) in self.entities.iter_mut().enumerate() {
            if state.is_dead() {
                let generation = state.make_alive();
                return EntityId { index, generation };
            }
        }

        let index = self.entities.len();
        let generation = 0;

        self.entities.push(EntityState::Alive(generation));

        EntityId { index, generation }
    }

    pub fn add_component<C: 'static>(&mut self) {
        self.components.insert(
            TypeId::of::<C>(),
            RwLock::new(GenericComponentStore::new::<C>()),
        );
    }

    pub fn with_component<C:'static>(mut self) -> Self {
        self.add_component::<C>();
        self
    }

    pub fn is_alive(&self, entity: EntityId) -> bool {
        self.entities.get(entity.index).map_or(false, |state| state == entity)
    }

    pub fn is_dead(&self, entity: EntityId) -> bool {
        !self.is_alive(entity)
    }

    pub fn drop_entity(&mut self, entity: EntityId) {
        if let Some(state) = self.entities.get_mut(entity.index) {
            if state.is_alive() {
                state.make_dead();
            }
        }
    }

    pub fn components<C: 'static>(&self) -> ComponentStoreReadLock<'_, C> {
        ComponentStoreReadLock::lock(&self.components[&TypeId::of::<C>()])
    }

    pub fn components_mut<C: 'static>(&self) -> ComponentStoreWriteLock<'_,C> {
        ComponentStoreWriteLock::lock(&self.components[&TypeId::of::<C>()])
    }

    pub fn entity_iter(&self) -> impl Iterator<Item=EntityId> + '_ {
        self.entities.iter()
            .enumerate()
            .filter_map(|(index, state)| state.alive_generation().map(|gen| (index, gen)))
            .map(|(index, generation)| EntityId { index, generation })
    }
}

pub trait LockType {
    type LockGuard<'a>: Deref<Target=GenericComponentStore>;

    fn lock(rwlock: &RwLock<GenericComponentStore>) -> Self::LockGuard<'_>;
}

pub struct ReadLockType;

impl LockType for ReadLockType {
    type LockGuard<'a> = RwLockReadGuard<'a, GenericComponentStore>;

    fn lock(rwlock: &RwLock<GenericComponentStore>) -> Self::LockGuard<'_> {
        rwlock.read().expect("should always be RwLock")
    }
}

pub struct WriteLockType;

impl LockType for WriteLockType {
    type LockGuard<'a> = RwLockWriteGuard<'a, GenericComponentStore>;

    fn lock(rwlock: &RwLock<GenericComponentStore>) -> Self::LockGuard<'_> {
        rwlock.write().expect("should always be RwLock")
    }
}

pub struct ComponentStoreLock<'a, C: 'static, L: LockType> {
    lock_guard: L::LockGuard<'a>,
    phantom_data: PhantomData<C>,
}

pub type ComponentStoreReadLock<'a, C: 'static> = ComponentStoreLock<'a, C, ReadLockType>;
pub type ComponentStoreWriteLock<'a, C: 'static> = ComponentStoreLock<'a, C, WriteLockType>;

impl<'a, C: 'static, L: LockType> ComponentStoreLock<'a, C, L> {
    fn lock(rwlock: &'a RwLock<GenericComponentStore>) -> Self {
        Self {
            lock_guard: L::lock(rwlock),
            phantom_data: PhantomData::default(),
        }
    }
}

impl<'a, C: 'static, L: LockType> Deref for ComponentStoreLock<'a, C, L> {
    type Target = ComponentStore<C>;

    fn deref(&self) -> &Self::Target {
        self.lock_guard.deref().store_for()
    }
}

impl<'a, C: 'static> DerefMut for ComponentStoreLock<'a, C, WriteLockType> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.lock_guard.deref_mut().store_for_mut()
    }
}

#[cfg(test)]
mod tests {
    use crate::world::World;

    #[derive(PartialEq, Eq, Debug)]
    struct Label(String);

    #[test]
    fn world_drop_entity() {
        let mut world = World::default();
        let entity_a = world.new_entity();
        let entity_b = world.new_entity();
        let entity_c = world.new_entity();

        assert!(world.is_alive(entity_a));
        assert!(world.is_alive(entity_b));
        assert!(world.is_alive(entity_c));
        assert!(!world.is_dead(entity_a));
        assert!(!world.is_dead(entity_b));
        assert!(!world.is_dead(entity_c));

        world.drop_entity(entity_b);

        assert!(world.is_alive(entity_a));
        assert!(world.is_dead(entity_b));
        assert!(world.is_alive(entity_c));

        world.drop_entity(entity_c);

        assert!(world.is_alive(entity_a));
        assert!(world.is_dead(entity_b));
        assert!(world.is_dead(entity_c));

        world.drop_entity(entity_a);

        assert!(world.is_dead(entity_a));
        assert!(world.is_dead(entity_b));
        assert!(world.is_dead(entity_c));
    }

    #[test]
    fn single_component() {
        let mut world = World::default().with_component::<Label>();
        let entity_a = world.new_entity();
        let entity_b = world.new_entity();
        let entity_c = world.new_entity();

        {
            let mut labels = world.components_mut::<Label>();
            labels.put(entity_a, Label("Entity A".to_owned()));
            // entity_b does not get label component
            labels.put(entity_c, Label("Entity C".to_owned()));
        }

        {
            let labels = world.components::<Label>();
            assert!(labels.has(entity_a));
            assert_eq!(labels.get(entity_a), Some(&Label("Entity A".to_owned())));

            assert!(!labels.has(entity_b));

            assert!(labels.has(entity_c));
            assert_eq!(labels.get(entity_c), Some(&Label("Entity C".to_owned())));
        }

        {
            let mut labels = world.components_mut::<Label>();
            assert_eq!(labels.remove(entity_a), Some(Label("Entity A".to_owned())));
        }

        {
            let labels = world.components::<Label>();
            assert!(!labels.has(entity_a));

            assert!(!labels.has(entity_b));

            assert!(labels.has(entity_c));
            assert_eq!(labels.get(entity_c), Some(&Label("Entity C".to_owned())));
        }
    }
}
