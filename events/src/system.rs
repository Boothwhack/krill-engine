use std::{collections::{HashMap, hash_map::Entry}, any::{TypeId, Any}, marker::PhantomData};

use crate::{Event, EventHandlers, handlers::{UnhandledEvent, GenericEventHandlers}};

/// Aggregate system for handling multiple types of [Event]s.
pub struct EventSystem<S> {
    handlers: HashMap<TypeId, Box<dyn GenericEventHandlers<S>>>,
    _phantom_state: PhantomData<S>,
}

impl<S: 'static> EventSystem<S> {
    pub fn new() -> Self {
        Self {
            handlers: Default::default(),
            _phantom_state: Default::default(),
        }
    }

    fn handlers_entry_for<M: 'static + Event>(&mut self) -> Entry<TypeId, Box<dyn GenericEventHandlers<S>>> {
        self.handlers.entry(TypeId::of::<M>())
    }

    /// Gets the handler list for the specified [Event] type.
    pub fn handlers_for<M: 'static + Event>(&mut self) -> &mut EventHandlers<M, S> {
        let boxed = self.handlers_entry_for::<M>().or_insert(Box::new(EventHandlers::<M, S>::new()));
        boxed.as_any_mut().downcast_mut::<EventHandlers<M, S>>().unwrap()
    }

    pub fn handle_event<E: 'static + Event>(&mut self, event: E, state: &mut S) -> Result<E::Output, E> {
        let handlers = match self.handlers_entry_for::<E>() {
            Entry::Occupied(e) => e.into_mut()
                .as_any_mut()
                .downcast_mut::<EventHandlers<E, S>>().unwrap(),
            Entry::Vacant(_) => return Err(event),
        };
        handlers.handle_event(event, state)
    }

    pub fn handle_generic_event(&mut self, event: Box<dyn Any>, state: &mut S) -> Result<Box<dyn Any>, UnhandledEvent> {
        match self.handlers.get_mut(&event.as_ref().type_id()) {
            Some(handlers) => handlers.handle_generic_event(event, state),
            None => Err(UnhandledEvent(event)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{EventSystem, Event};

    #[derive(Debug, PartialEq)]
    struct EventA(u32);

    impl Event for EventA {
        type Output = f32;
    }

    #[derive(Debug, PartialEq)]
    struct EventB(&'static str);

    impl Event for EventB {
        type Output = &'static str;
    }

    #[derive(Debug, PartialEq)]
    struct EventC(bool);

    impl Event for EventC {
        type Output = &'static str;
    }

    #[test]
    fn handles_multiple_event_types() {
        let mut system: EventSystem<()> = EventSystem::new();

        system.handlers_for().append(|msg: EventA, _context| {
            msg.0 as f32
        });

        system.handlers_for().append(|msg: EventB, _context| {
            if msg.0.len() > 4 {
                "Too long!"
            } else {
                msg.0
            }
        });

        assert_eq!(Ok(10f32), system.handle_event(EventA(10), &mut ()));
        assert_eq!(Ok("Too long!"), system.handle_event(EventB("Hello, World!"), &mut ()));
        assert_eq!(Err(EventC(true)), system.handle_event(EventC(true), &mut ()));
    }

    #[test]
    fn handles_generic_events() {
        let mut system: EventSystem<()> = EventSystem::new();

        system.handlers_for().append(|msg: EventA, _context| {
            msg.0 as f32
        });

        system.handlers_for().append(|msg: EventB, _context| {
            if msg.0.len() > 4 {
                "Too long!"
            } else {
                msg.0
            }
        });

        assert!(system.handle_generic_event(Box::new(EventA(8)), &mut ()).is_ok());
        assert!(system.handle_generic_event(Box::new(EventB("Test")), &mut ()).is_ok());
        assert!(system.handle_generic_event(Box::new(EventC(false)), &mut ()).is_err());
    }
}
