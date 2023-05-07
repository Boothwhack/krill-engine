use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use utils::hlist::{IntoShape, ToMut};

/// An event listener is a simple function pointer, taking as parameters the event and a [Context].
/// The listener can attempt to propagate the [Event] to the next listener in line, if one is
/// present.
pub type Listener<E, R> = fn(E, Context<E, R>) -> <E as Event>::Output;

pub trait Event: 'static {
    type Output;
}

/// Context for the current event being dispatched.
pub struct Context<'a, E: Event, R> {
    resources: &'a mut R,
    iter: Box<dyn 'a + Iterator<Item=&'a mut Listener<E, R>>>,
    _phantom_event: PhantomData<E>,
}

impl<'a, E: Event, R> Context<'a, E, R> {
    fn new(resources: &'a mut R, iter: impl 'a + Iterator<Item=&'a mut Listener<E, R>>) -> Self {
        Context {
            resources,
            iter: Box::new(iter),
            _phantom_event: Default::default(),
        }
    }

    /// Hands the event off to the next event listener in line and returns its output, or None if
    /// there are no more listeners registered.
    pub fn next(mut self, event: E) -> Option<E::Output> {
        let listener = self.iter.next()?;
        Some(listener(event, Context { ..self }))
    }

    /// Gets a mutable reference to a subset of resources.
    pub fn res<'b, S: 'b, I>(&'b mut self) -> S::Output<'b>
        where R: ToMut,
              S: ToMut,
              <R as ToMut>::Output<'b>: IntoShape<<S as ToMut>::Output<'b>, I> {
        self.resources.to_mut().into_shape().0
    }
}

struct EventDispatcher<E: Event, R> {
    listeners: Vec<Listener<E, R>>,

    _phantom_event: PhantomData<E>,
    _phantom_resources: PhantomData<R>,
}

impl<E: Event, R> EventDispatcher<E, R> {
    fn new() -> Self {
        EventDispatcher {
            listeners: Default::default(),
            _phantom_event: Default::default(),
            _phantom_resources: Default::default(),
        }
    }

    fn dispatch(&mut self, resources: &mut R, event: E) -> Option<E::Output> {
        let iter = self.listeners.iter_mut();
        let context = Context::new(resources, iter);
        context.next(event)
    }
}

pub struct EventBus<R> {
    dispatchers: HashMap<TypeId, Box<dyn Any>>,

    _phantom_resources: PhantomData<R>,
}

impl<R> Default for EventBus<R> {
    fn default() -> Self {
        EventBus {
            dispatchers: Default::default(),
            _phantom_resources: Default::default(),
        }
    }
}

impl<R:'static> EventBus<R> {
    pub fn listener<E: Event>(&mut self, listener: Listener<E, R>) {
        let dispatcher = self.dispatchers.entry(TypeId::of::<E>())
            .or_insert_with(|| Box::new(EventDispatcher::<E, R>::new()));
        dispatcher.downcast_mut::<EventDispatcher<E, R>>().unwrap()
            .listeners.push(listener);
    }

    /// May return None if no listeners are registered for the event type.
    pub fn emit<E: Event>(&mut self, resources: &mut R, event: E) -> Option<E::Output> {
        let dispatcher = self.dispatchers.get_mut(&TypeId::of::<E>())?;
        let dispatcher = dispatcher.downcast_mut::<EventDispatcher<E, R>>()?;
        dispatcher.dispatch(resources, event)
    }
}

#[cfg(test)]
mod tests {
    use utils::{hlist, HList, delist};
    use crate::events::{Event, EventBus};

    struct EventA(u32);

    impl Event for EventA {
        type Output = f32;
    }

    #[test]
    fn event_bus() {
        let mut bus: EventBus<()> = EventBus::default();
        bus.listener(|EventA(value), _context| {
            value as f32 + 0.5
        });

        let output = bus.emit(&mut (), EventA(10));
        assert_eq!(Some(10.5f32), output);
    }

    #[test]
    fn event_bus_with_resources() {
        let mut resources = hlist!("string".to_owned(), 20u32);
        let mut bus: EventBus<HList!(String, u32)> = EventBus::default();
        bus.listener(|EventA(value), mut context| {
            let delist!(_string_res, int_res) = context.res::<HList!(String, u32), _>();

            (*int_res + value) as f32
        });

        let output = bus.emit(&mut resources, EventA(2));
        assert_eq!(Some(22f32), output);
    }
}
