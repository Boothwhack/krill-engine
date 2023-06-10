use std::any::{Any, type_name, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use utils::hlist::{FnMapHList, Mappable, Prepend};

use crate::store::ComponentStore;

pub type Generation = u32;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
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

    pub fn with_component<C: 'static>(mut self) -> Self {
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
        ComponentStoreReadLock::lock(&self.components.get(&TypeId::of::<C>())
            .expect(&format!("unknown component type: {}", type_name::<C>())))
    }

    pub fn components_mut<C: 'static>(&self) -> ComponentStoreWriteLock<'_, C> {
        ComponentStoreWriteLock::lock(&self.components[&TypeId::of::<C>()])
    }

    pub fn entity_iter(&self) -> impl Iterator<Item=EntityId> + '_ {
        self.entities.iter()
            .enumerate()
            .filter_map(|(index, state)| state.alive_generation().map(|gen| (index, gen)))
            .map(|(index, generation)| EntityId { index, generation })
    }
}

pub struct ComponentBinding<T, R> {
    typ: PhantomData<T>,
    requirement: PhantomData<R>,
}

impl<T, R> Default for ComponentBinding<T, R> {
    fn default() -> Self {
        ComponentBinding {
            typ: Default::default(),
            requirement: Default::default(),
        }
    }
}

impl<T, R> Clone for ComponentBinding<T, R> {
    fn clone(&self) -> Self {
        ComponentBinding::default()
    }
}

impl<T, R> Copy for ComponentBinding<T, R> {}

pub trait BindingRequirement {
    type Resolved<T, C>: Prepend
        where C: Prepend;

    fn resolve<T, C>(component: Option<T>, list: C) -> Result<Self::Resolved<T, C>, ()>
        where C: Prepend;
}

pub struct Required;

impl BindingRequirement for Required {
    type Resolved<T, C> = (T, C)
        where C: Prepend;

    fn resolve<T, C>(component: Option<T>, list: C) -> Result<(T, C), ()>
        where C: Prepend {
        let list = match component {
            Some(component) => Ok(list.prepend(component)),
            None => Err(())
        };
        list
    }
}

pub struct Optional;

impl BindingRequirement for Optional {
    type Resolved<T, C> = (Option<T>, C)
        where C: Prepend;

    fn resolve<T, C>(component: Option<T>, list: C) -> Result<(Option<T>, C), ()>
        where C: Prepend {
        Ok(list.prepend(component))
    }
}

pub struct Marked;

impl BindingRequirement for Marked {
    type Resolved<T, C> = C
        where C: Prepend;

    fn resolve<T, C>(component: Option<T>, list: C) -> Result<C, ()>
        where C: Prepend {
        component.map(|_| list).ok_or(())
    }
}

pub struct Bound<'v, T: 'static, R: BindingRequirement> {
    store: ComponentStoreLock<'v, T, ReadLockType>,
    binding: ComponentBinding<T, R>,
}

pub struct StoreLocker<'a> {
    world: &'a World,
}

impl<'a, T, R, Tail, RTail> FnMapHList<(ComponentBinding<T, R>, Tail), (Bound<'a, T, R>, RTail)> for StoreLocker<'a>
    where T: 'static,
          R: BindingRequirement,
          Self: FnMapHList<Tail, RTail> {
    fn invoke(self, list: (ComponentBinding<T, R>, Tail)) -> (Bound<'a, T, R>, RTail) {
        let (binding, tail) = list;
        let store = self.world.components();
        (Bound { store, binding }, self.invoke(tail))
    }
}

impl<'a> FnMapHList<(), ()> for StoreLocker<'a> {
    fn invoke(self, _list: ()) -> () {
        ()
    }
}

pub struct ViewBuilder<C> {
    components: C,
}

impl ViewBuilder<()> {
    fn new() -> Self {
        Self { components: () }
    }
}

impl<C> ViewBuilder<C>
    where C: Prepend {
    fn with_binding<T: 'static, R>(self, binding: ComponentBinding<T, R>) -> ViewBuilder<(ComponentBinding<T, R>, C)> {
        ViewBuilder { components: self.components.prepend(binding) }
    }

    pub fn required<T: 'static>(self) -> ViewBuilder<(ComponentBinding<T, Required>, C)> {
        self.with_binding(ComponentBinding::default())
    }

    pub fn optional<T: 'static>(self) -> ViewBuilder<(ComponentBinding<T, Optional>, C)> {
        self.with_binding(ComponentBinding::default())
    }

    pub fn marked<T: 'static>(self) -> ViewBuilder<(ComponentBinding<T, Marked>, C)> {
        self.with_binding(ComponentBinding::default())
    }

    pub fn build<'a, R>(self, world: &'a World) -> View<'a, R>
        where C: Mappable,
              R: Bounds,
              StoreLocker<'a>: FnMapHList<C, R> {
        let stores = self.components.map(StoreLocker { world });
        View { world, bounds: stores }
    }
}

pub struct View<'w, B: Bounds> {
    world: &'w World,
    bounds: B,
}

impl<'w> View<'w, ()> {
    pub fn builder() -> ViewBuilder<()> {
        ViewBuilder::new()
    }
}

impl<'w, B: Bounds> View<'w, B> {
    pub fn iter<'v>(&'v self) -> EntityIterator<'w, 'v, B, impl 'w + Iterator<Item=EntityId>>
        where 'w: 'v {
        let iter = self.world.entity_iter();
        EntityIterator {
            view: self,
            iter,
        }
    }
}

pub trait Bounds {
    type Result<'a, C>
        where Self: 'a,
              C: 'a + Prepend;

    fn match_entity<'v, C>(&'v self, entity: EntityId, list: C) -> Option<Self::Result<'v, C>>
        where C: 'v + Prepend;
}

impl<'b, T: 'static, R, Tail> Bounds for (Bound<'b, T, R>, Tail)
    where R: BindingRequirement,
          Tail: Bounds {
    type Result<'a, C> = Tail::Result<'a, R::Resolved<&'a T, C>>
        where Self: 'a,
              C: 'a + Prepend;

    fn match_entity<'v, C>(&'v self, entity: EntityId, list: C) -> Option<Self::Result<'v, C>>
        where C: 'v + Prepend {
        let component = self.0.store.get(entity);
        let list = match R::resolve(component, list) {
            Ok(list) => list,
            Err(_) => return None,
        };

        self.1.match_entity(entity, list)
    }
}

impl Bounds for () {
    type Result<'a, C> = C
        where Self: 'a,
              C: 'a + Prepend;

    fn match_entity<'w, C>(&self, _entity: EntityId, list: C) -> Option<C>
        where C: 'w + Prepend {
        Some(list)
    }
}

pub struct EntityIterator<'w, 'v, B: Bounds, I: 'w + Iterator<Item=EntityId>> {
    view: &'v View<'w, B>,
    iter: I,
}

impl<'w, 'v, B, I> Iterator for EntityIterator<'w, 'v, B, I>
    where B: Bounds,
          I: Iterator<Item=EntityId> {
    type Item = (EntityId, B::Result<'v, ()>);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(candidate) = self.iter.next() {
            if let Some(matched) = self.view.bounds.match_entity(candidate, ()) {
                return Some((candidate, matched));
            }
        }
        None
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

pub type ComponentStoreReadLock<'a, C> = ComponentStoreLock<'a, C, ReadLockType>;
pub type ComponentStoreWriteLock<'a, C> = ComponentStoreLock<'a, C, WriteLockType>;

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
    use utils::hlist;

    use crate::world::{ViewBuilder, World};

    #[derive(PartialEq, Eq, Debug)]
    struct Label(String);

    struct Player {
        health: f32,
    }

    struct Enemy;

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

    #[test]
    fn system() {
        let mut world = World::default()
            .with_component::<Label>()
            .with_component::<Player>()
            .with_component::<Enemy>();

        let entity_a = world.new_entity();
        let entity_b = world.new_entity();
        let entity_c = world.new_entity();

        {
            let mut labels = world.components_mut::<Label>();
            labels.put(entity_a, Label("Entity A".to_owned()));
            labels.put(entity_b, Label("Entity B".to_owned()));
            labels.put(entity_c, Label("Entity C".to_owned()));

            let mut players = world.components_mut::<Player>();
            players.put(entity_a, Player { health: 5.5 });

            let mut enemies = world.components_mut::<Enemy>();
            enemies.put(entity_c, Enemy);
        }

        let view = ViewBuilder::new()
            .required::<Label>()
            .build(&world);

        let labels = view.iter().collect::<Vec<_>>();
        assert_eq!(vec![
            (entity_a, hlist!(&Label("Entity A".to_owned()))),
            (entity_b, hlist!(&Label("Entity B".to_owned()))),
            (entity_c, hlist!(&Label("Entity C".to_owned()))),
        ], labels);

        let view = ViewBuilder::new()
            .required::<Label>()
            .marked::<Player>()
            .build(&world);
        let players = view.iter().collect::<Vec<_>>();
        assert_eq!(vec![
            (entity_a, hlist!(&Label("Entity A".to_owned()))),
        ], players);

        let view = ViewBuilder::new()
            .required::<Label>()
            .marked::<Enemy>()
            .build(&world);
        let enemies = view.iter().collect::<Vec<_>>();
        assert_eq!(vec![
            (entity_c, hlist!(&Label("Entity C".to_owned()))),
        ], enemies);
    }
}
