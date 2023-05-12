use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::mpsc::{channel, Receiver, Sender, SendError};
use thiserror::Error;
use utils::hlist::{IntoShape, ToMut};

/// An event listener is a simple function pointer, taking as parameters the event and a [Context].
/// The listener can attempt to propagate the [Event] to the next listener in line, if one is
/// present.
pub type Listener<E, R> = dyn FnMut(E, Context<E, R>);

/// Context for the current event being dispatched.
pub struct Context<'a, E, R> {
    resources: &'a mut R,
    iter: Box<dyn 'a + Iterator<Item=&'a mut Listener<E, R>>>,
    _phantom_event: PhantomData<E>,
}

pub trait ContextWith<S, I>
    where S: ToMut {
    /// Gets a mutable reference to a subset of resources.
    fn resources_mut(&mut self) -> S::Output<'_>;
}

impl<'a, E, R> Context<'a, E, R> {
    fn new(resources: &'a mut R, iter: impl 'a + Iterator<Item=&'a mut Listener<E, R>>) -> Self {
        Context {
            resources,
            iter: Box::new(iter),
            _phantom_event: Default::default(),
        }
    }

    /// Hands the event off to the next event listener in line and returns its output, or None if
    /// there are no more listeners registered.
    pub fn next(mut self, event: E) {
        if let Some(listener) = self.iter.next() {
            listener(event, Context { ..self })
        }
    }

    /// Gets a mutable reference to a subset of resources.
    pub fn res_mut<'b, S: 'b, I>(&'b mut self) -> S::Output<'b>
        where R: ToMut,
              S: ToMut,
              <R as ToMut>::Output<'b>: IntoShape<<S as ToMut>::Output<'b>, I> {
        self.resources.to_mut().into_shape().0
    }
}

impl<'a, S, E, R, I> ContextWith<S, I> for Context<'a, E, R>
    where S: ToMut,
          R: 'static + ToMut,
          for<'b> <R as ToMut>::Output<'b>: IntoShape<S::Output<'b>, I> {
    fn resources_mut(&mut self) -> S::Output<'_> {
        self.resources.to_mut().into_shape().0
    }
}

pub struct Listeners<E, R> {
    listeners: Vec<Box<Listener<E, R>>>,
}

impl<E, R> Listeners<E, R> {
    pub fn new() -> Self {
        Listeners { listeners: Vec::new() }
    }

    pub fn listener(&mut self, listener: impl FnMut(E, Context<E, R>) + 'static) {
        self.listeners.push(Box::new(listener));
    }

    pub fn with_listener(mut self, listener: impl FnMut(E, Context<E, R>) + 'static) -> Self {
        self.listener(listener);
        self
    }

    fn iter_mut(&mut self) -> impl Iterator<Item=&'_ mut Listener<E, R>> {
        self.listeners.iter_mut().map(Box::as_mut)
    }
}

#[derive(Error, Debug)]
#[error("this event cannot be handled by this EventBus")]
pub struct InvalidEvent;

trait GenericListeners<R> {
    fn dispatch(&mut self, event: Box<dyn Any>, resources: &mut R) -> Result<(), InvalidEvent>;
}

impl<E: 'static, R> GenericListeners<R> for Listeners<E, R> {
    fn dispatch(&mut self, event: Box<dyn Any>, resources: &mut R) -> Result<(), InvalidEvent> {
        let event: Box<E> = event.downcast().map_err(|_| InvalidEvent)?;
        let listener_iter = self.listeners
            .iter_mut()
            .map(Box::as_mut);
        let context = Context::new(resources, listener_iter);
        context.next(*event);
        Ok(())
    }
}

pub struct EventBus<R> {
    listeners: HashMap<TypeId, Box<dyn GenericListeners<R>>>,
    receiver: Receiver<Box<dyn Any>>,

    _phantom_resources: PhantomData<R>,
}

#[derive(Clone, Debug)]
pub struct EventSender {
    sender: Sender<Box<dyn Any>>,
}

impl EventSender {
    pub fn send<E: 'static>(&self, event: E) -> Result<(), SendError<E>> {
        self.sender.send(Box::new(event))
            .map_err(|err| SendError(*err.0.downcast().unwrap()))
    }
}

impl<R: 'static> EventBus<R> {
    pub fn new() -> (EventSender, Self) {
        let (sender, receiver) = channel();
        let sender = EventSender { sender };

        (sender, EventBus {
            listeners: Default::default(),
            receiver,
            _phantom_resources: Default::default(),
        })
    }

    /// Registers an event in this bus with the given listeners. The listeners are called front to
    /// back, and the listeners can delegate to the next.
    pub fn register_event<E: 'static>(&mut self, listeners: Listeners<E, R>) {
        self.listeners.insert(TypeId::of::<E>(), Box::new(listeners));
    }

    /// Dispatches all events that are queued up by the sender(s)
    pub fn dispatch_all(&mut self, resources: &mut R) -> Result<(), InvalidEvent> {
        while let Ok(event) = self.receiver.try_recv() {
            if let Some(listeners) = self.listeners.get_mut(&event.as_ref().type_id()) {
                listeners.dispatch(event, resources)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};
    use utils::{hlist, HList, delist};
    use crate::events::{EventBus, Listeners};

    struct EventA(u32);

    struct EventAdd(u32);

    struct EventMul(u32);

    #[test]
    fn event_bus() {
        let (sender, mut bus) = EventBus::new();
        let outer_value = Arc::new(RwLock::new(0));

        let inner_value = outer_value.clone();
        bus.register_event(
            Listeners::new().with_listener(move |EventA(value), _context| {
                *inner_value.write().unwrap() = value;
            })
        );

        sender.send(EventA(10)).unwrap();
        bus.dispatch_all(&mut ()).unwrap();
        assert_eq!(10u32, *outer_value.read().unwrap());
    }

    #[test]
    fn event_bus_with_resources() {
        let mut resources = hlist!("string".to_owned(), 20u32);
        let (sender, mut bus) = EventBus::new();
        let outer_value = Arc::new(RwLock::new(0));

        let inner_value = outer_value.clone();
        bus.register_event(
            Listeners::new().with_listener(move |EventA(..), mut context| {
                let delist!(_string_res, int_res) = context.res_mut::<HList!(String, u32), _>();
                *inner_value.write().unwrap() = *int_res;
            })
        );

        sender.send(EventA(10)).unwrap();
        bus.dispatch_all(&mut resources).unwrap();

        assert_eq!(20u32, *outer_value.read().unwrap());
    }

    #[test]
    fn event_order() {
        let (sender, mut bus) = EventBus::new();
        let outer_value = Arc::new(RwLock::new(0));

        let inner_a = outer_value.clone();
        let inner_b = outer_value.clone();
        bus.register_event(
            Listeners::new().with_listener(move |EventAdd(value), _context| {
                *inner_a.write().unwrap() += value;
            })
        );
        bus.register_event(
            Listeners::new().with_listener(move |EventMul(value), _context| {
                *inner_b.write().unwrap() *= value;
            })
        );

        sender.send(EventAdd(10)).unwrap();
        sender.send(EventMul(2)).unwrap();
        sender.send(EventAdd(10)).unwrap();
        bus.dispatch_all(&mut ()).unwrap();

        assert_ne!(40, *outer_value.read().unwrap());
        assert_eq!(30, *outer_value.read().unwrap());
    }
}
