use std::any::{Any, TypeId};
use std::mem::replace;
use std::ops::{Deref, DerefMut};

use crate::Event;

/// Passed into event handlers as the last parameter. Use the member function
/// `delegate` to send the event to the next handler in line.
pub struct Context<'a, 's, E: Event, S> {
    state: DelegationState<'a,'s, E, S>,
}

enum DelegationState<'a, 's, E: Event, S> {
    NotDelegated {
        upstream: Option<Upstream<'a, E, S>>,
        state: &'s mut S,
    },
    Delegating,
    Delegated {
        state: &'s mut S,
    }
}

impl<'a, 's, E: Event, S> Context<'a, 's, E, S> {
    fn new(upstream: Option<Upstream<'a, E, S>>, state: &'s mut S) -> Self {
        Self { state: DelegationState::NotDelegated { upstream, state } }
    }

    /// Passes the event to the next event handler in the chain, if there is 
    /// one. Returns [None] if there are no handlers to delegate to, otherwise
    /// returns the next handler's output.
    pub fn delegate(&mut self, event: E) -> Option<E::Output> {
        let (state, upstream) = match replace(&mut self.state, DelegationState::Delegating) {
            DelegationState::NotDelegated { upstream, state } => (state, upstream),
            _ => panic!("Already delegated!")
        };

        let Upstream { next, queue } = upstream?;

        let next_upstream = queue.split_first_mut().map(Upstream::from);
        let next_delegator = Context::new(next_upstream, state);

        let output = Some(next(event, next_delegator));

        self.state = DelegationState::Delegated { state };
        output
    }
}

impl<'a, 's, M: Event, S> DerefMut for Context<'a, 's, M, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.state {
            DelegationState::NotDelegated { state, .. } => state,
            DelegationState::Delegating => panic!("should not have access to context while delegating"),
            DelegationState::Delegated { state } => state,
        }
    }
}

impl<'a, 's, M: Event, S> Deref for Context<'a, 's, M, S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        match &self.state {
            DelegationState::NotDelegated { state, .. } => state,
            DelegationState::Delegating => panic!("should not have access to context while delegating"),
            DelegationState::Delegated { state } => state,
        }
    }
}

struct Upstream<'a, M: Event, C> {
    next: &'a mut dyn FnMut(M, Context<'_, '_, M, C>) -> M::Output,
    queue: &'a mut [Box<dyn FnMut(M, Context<'_, '_, M, C>) -> M::Output>],
}

impl<'a, M: Event, C> From<(&'a mut Box<dyn FnMut(M, Context<'_, '_, M, C>) -> M::Output>, &'a mut [Box<dyn FnMut(M, Context<'_, '_, M, C>) -> M::Output>])> for Upstream<'a, M, C> {
    fn from((next, queue): (&'a mut Box<dyn FnMut(M, Context<'_, '_, M, C>) -> M::Output>, &'a mut [Box<dyn FnMut(M, Context<'_, '_, M, C>) -> M::Output>])) -> Self {
        Self { next, queue }
    }
}

/// List of handlers for a specific type of [Event].
pub struct EventHandlers<M: Event, S> {
    handlers: Vec<Box<dyn FnMut(M, Context<'_, '_, M, S>) -> M::Output>>,
}

impl<M: Event, S> EventHandlers<M, S> {
    pub fn new() -> Self {
        EventHandlers { handlers: vec![] }
    }

    /// Appends a handler to this handler list. This handler will be called 
    /// after all previously registered handlers.
    pub fn append(&mut self, handler: impl 'static + FnMut(M, Context<'_, '_, M, S>) -> M::Output) {
        self.handlers.push(Box::new(handler));
    }

    /// Prepends a handler to this handler list. This handler will be called 
    /// before all previously registered handlers.
    pub fn prepend(&mut self, handler: impl 'static + FnMut(M, Context<'_, '_, M, S>) -> M::Output) {
        self.handlers.insert(0, Box::new(handler));
    }

    pub fn handle_event(&mut self, event: M, state: &mut S) -> Result<M::Output, M> {
        if self.handlers.is_empty() {
            return Err(event);
        }

        let handlers = self.handlers.as_mut_slice();

        let upstream = handlers.split_first_mut().map(Upstream::from);
        let mut context = Context::new(upstream, state);
        
        Ok(context.delegate(event).expect("handlers is not empty"))
    }
}

#[derive(Debug)]
pub struct UnhandledEvent(pub Box<dyn Any>);

impl UnhandledEvent {
    pub fn event_type(&self) -> TypeId {
        self.0.as_ref().type_id()
    }
}

/// Allows [EventHandlers] to attampt to handle boxed events of any type.
pub trait GenericEventHandlers<S> {
    /// Attempts to handle the given event. Returns [UnhandledEvent], passing
    /// the original event back to the caller, if this instance is not capable 
    /// of handling the event.
    fn handle_generic_event(&mut self, event: Box<dyn Any>, context: &mut S) -> Result<Box<dyn Any>, UnhandledEvent>;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<E: 'static + Event, S: 'static> GenericEventHandlers<S> for EventHandlers<E, S> {
    fn handle_generic_event(&mut self, event: Box<dyn Any>, state: &mut S) -> Result<Box<dyn Any>, UnhandledEvent> {
        let event = *event.downcast::<E>().map_err(UnhandledEvent)?;

        self.handle_event(event, state)
            .map(|output| -> Box<dyn Any> { Box::new(output) })
            .map_err(|unhandled| UnhandledEvent(Box::new(unhandled)))
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::{Event, EventHandlers, handlers::GenericEventHandlers};

    #[derive(Debug, PartialEq)]
    struct EventA(u32);

    impl Event for EventA {
        type Output = f32;
    }

    #[derive(Debug, PartialEq)]
    struct EventB(f32);

    impl Event for EventB {
        type Output = u32;
    }

    #[test]
    fn single_handler() {
        let mut handlers: EventHandlers<EventA, ()> = EventHandlers::new();

        handlers.append(|msg, _context| {
            msg.0 as f32
        });

        assert_eq!(Ok(13f32), handlers.handle_event(EventA(13u32), &mut ()));
        assert_eq!(Ok(5f32), handlers.handle_event(EventA(5u32), &mut ()));
    }

    #[test]
    fn basic_delegation() {
        let mut handlers: EventHandlers<EventA, ()> = EventHandlers::new();

        handlers.append(|msg, mut context| {
            context.delegate(msg).unwrap() * 2f32
        });
        handlers.append(|msg, _context| {
            msg.0 as f32
        });

        assert_eq!(Ok(26f32), handlers.handle_event(EventA(13u32), &mut ()));
        assert_eq!(Ok(10f32), handlers.handle_event(EventA(5u32), &mut ()));
    }

    #[test]
    fn delegate_returns_none_on_last() {
        let mut handlers: EventHandlers<EventA, ()> = EventHandlers::new();

        handlers.append(|msg, mut context| {
            context.delegate(msg).unwrap() + 2f32
        });
        handlers.append(|msg, mut context| {
            assert!(context.delegate(msg).is_none());
            5f32
        });

        assert_eq!(Ok(7f32), handlers.handle_event(EventA(0u32), &mut ()))
    }

    #[test]
    fn err_on_empty() {
        let mut handlers: EventHandlers<EventA, ()> = EventHandlers::new();

        assert_eq!(Err(EventA(10)), handlers.handle_event(EventA(10), &mut ()));
    }

    #[test]
    fn handle_generic_events() {
        let mut handlers: EventHandlers<EventA, ()> = EventHandlers::new();

        handlers.append(|msg, _context| msg.0 as f32);

        let output = handlers.handle_generic_event(Box::new(EventA(10u32)), &mut ());
        let output = output.expect("should be ok");
        assert!(output.is::<f32>());

        let output = handlers.handle_generic_event(Box::new(EventB(1.5f32)), &mut ());
        let unknown_event_type = output.expect_err("should be err");
        assert!(unknown_event_type.0.is::<EventB>());
    }
}
