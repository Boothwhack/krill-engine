use std::vec::Drain;

#[derive(Default)]
pub struct EventBus<E> {
    events: Vec<E>,
}

pub type EventBusDrain<'a, E> = Drain<'a, E>;

impl<E> EventBus<E> {
    pub fn drain(&mut self) -> EventBusDrain<E> {
        self.events.drain(0..)
    }

    pub fn push(&mut self, event: E) {
        self.events.push(event);
    }
}

pub enum Event {
    InputEvent(InputEvent),
}

pub enum InputEvent {
    Keyboard { key: u32 },
}
