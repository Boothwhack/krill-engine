// - Supports adding and removing arbitrary components from any entity.
// - Iterate every entity
// - Iterate every entity that satisfies criteria, only touch those components.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;

#[derive(Default)]
struct Entity {
    components: HashMap<TypeId, Box<dyn Any>>,
}

impl Entity {
    fn component<T: 'static>(&self) -> Option<&T> {
        self.component_from_typeid(&TypeId::of::<T>())
    }

    fn component_from_typeid<T: 'static>(&self, typ: &TypeId) -> Option<&T> {
        self.components.get(typ).and_then(|it| it.downcast_ref())
    }
}

#[derive(Default)]
pub struct World {
    entities: Vec<Option<Entity>>,
}

pub type EntityHandle = usize;

impl World {
    pub fn new_entity(&mut self) -> EntityHandle {
        if let Some((index, slot)) = self.entities
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_none()) {
            *slot = Some(Entity::default());
            index
        } else {
            let index = self.entities.len();
            self.entities.push(Some(Entity::default()));
            index
        }
    }

    pub fn attach<T: 'static>(&mut self, entity: EntityHandle, component: T) {
        if let Some(Some(entity)) = self.entities.get_mut(entity) {
            entity.components.insert(TypeId::of::<T>(), Box::new(component));
        }
    }

    pub fn component<T: 'static>(&self, entity: EntityHandle) -> Option<&T> {
        self.entities.get(entity)?.as_ref()?.component()
    }

    pub fn kill(&mut self, entity: EntityHandle) {
        self.entities[entity] = None;
    }

    pub fn entity_iter(&self) -> impl Iterator<Item=EntityItem> {
        self.entities.iter()
            .enumerate()
            .filter_map(|(handle, entity)| entity.as_ref().map(|entity| (handle, entity)))
            .map(|(handle, entity)| { EntityItem { handle, entity } })
    }

    pub fn component_iter<T: 'static>(&self) -> ComponentIterator<impl Iterator<Item=EntityItem>, 1, &T> {
        ComponentIterator {
            iterator: self.entity_iter(),
            types: [TypeId::of::<T>()],
            phantom_data: PhantomData::default(),
        }
    }

    pub fn component_iter2<T0: 'static, T1: 'static>(&self) -> ComponentIterator<impl Iterator<Item=EntityItem>, 2, (&T0, &T1)> {
        ComponentIterator {
            iterator: self.entity_iter(),
            types: [TypeId::of::<T0>(), TypeId::of::<T1>()],
            phantom_data: PhantomData::default(),
        }
    }
}

pub struct ComponentIterator<'a, I: Iterator<Item=EntityItem<'a>>, const N: usize, R> {
    iterator: I,
    types: [TypeId; N],
    phantom_data: PhantomData<R>,
}

trait CombineComponentTuple<'a, const N: usize, R> {
    fn combine_tuple(entity: &'a Entity, types: &[TypeId; N]) -> Option<R>;
}

impl<'a, I, T: 'static> CombineComponentTuple<'a, 1, &'a T> for ComponentIterator<'a, I, 1, &'a T>
    where I: Iterator<Item=EntityItem<'a>> {
    fn combine_tuple(entity: &'a Entity, types: &[TypeId; 1]) -> Option<&'a T> {
        entity.component_from_typeid(&types[0])
    }
}

impl<'a, I, T0: 'static, T1: 'static> CombineComponentTuple<'a, 2, (&'a T0, &'a T1)> for ComponentIterator<'a, I, 2, (&'a T0, &'a T1)>
    where I: Iterator<Item=EntityItem<'a>> {
    fn combine_tuple(entity: &'a Entity, types: &[TypeId; 2]) -> Option<(&'a T0, &'a T1)> {
        Some((entity.component_from_typeid(&types[0])?, entity.component_from_typeid(&types[1])?))
    }
}

impl<'a, I, const N: usize, R: 'a> Iterator for ComponentIterator<'a, I, N, R>
    where I: Iterator<Item=EntityItem<'a>>, Self: CombineComponentTuple<'a, N, R> {
    type Item = R;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.find_map(|entity| {
            Self::combine_tuple(entity.entity, &self.types)
        })
    }
}

pub struct EntityItem<'a> {
    handle: EntityHandle,
    entity: &'a Entity,
}

impl<'a> EntityItem<'a> {
    pub fn handle(&self) -> EntityHandle {
        self.handle
    }

    pub fn component<T: 'static>(&self) -> Option<&T> {
        self.entity.component()
    }

    pub fn component_from_typeid<T: 'static>(&self, typ: &TypeId) -> Option<&T> {
        self.entity.component_from_typeid(typ)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Position {
        x: f32,
        y: f32,
    }

    #[test]
    fn test_new_entity() {
        let mut world = World::default();
        let entity_1 = world.new_entity();
        let entity_2 = world.new_entity();
        world.attach(entity_1, Position { x: 1.0, y: 2.0 });

        let position_1 = world.component::<Position>(entity_1);
        let position_2 = world.component::<Position>(entity_2);
        assert!(position_1.is_some());
        assert!(position_2.is_none());

        let position_1 = position_1.unwrap();
        assert_eq!(position_1.x, 1.0);
        assert_eq!(position_1.y, 2.0);
    }

    #[test]
    fn test_kill_entity() {
        let mut world = World::default();
        let entity_1 = world.new_entity();
        let entity_2 = world.new_entity();

        world.attach(entity_1, Position { x: 2.0, y: 3.0 });
        world.attach(entity_2, Position { x: 3.0, y: 4.0 });

        world.kill(entity_1);

        let position_1 = world.component::<Position>(entity_1);
        let position_2 = world.component::<Position>(entity_2);
        assert!(position_1.is_none());
        assert!(position_2.is_some());
    }

    #[test]
    fn test_iterate_entities() {
        let mut world = World::default();
        let mut entities: Vec<(bool, EntityHandle)> = (0..10).map(|_| (false, world.new_entity())).collect();

        for entity in world.entity_iter() {
            let (found, _) = entities.iter_mut()
                .find(|(_, handle)| *handle == entity.handle())
                .expect(&format!("world produced unknown entity handle: {:?}", entity.handle()));
            assert!(!*found, "world produced duplicate entity handle: {:?}", entity.handle);
            *found = true;
        }

        for (found, handle) in entities {
            assert!(found, "world did not produce entity handle: {:?}", handle);
        }
    }
}
