use std::any::Any;
use std::future::IntoFuture;
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::{Receiver, Sender, channel};
use events::{EventSystem, Event, UnhandledEvent};
use utils::hlist::{Concat, IntoShape};
use crate::resources::Resources;

pub struct ProcessInfo;

pub struct ProcessBuilder<R> {
    resources: R,
}

impl ProcessBuilder<()> {
    pub fn new() -> Self {
        ProcessBuilder { resources: () }
    }
}

impl<R: 'static> ProcessBuilder<R> {
    pub fn setup<F, Input, InputI, Output>(self, setup: F) -> ProcessBuilder<<R::Remainder as Concat>::Concatenated<Output>>
        where Output: 'static,
              R: IntoShape<Input, InputI>,
              R::Remainder: Concat,
              F: FnOnce(Input) -> Output {
        let (input, remainder) = self.resources.into_shape();
        let output = setup(input);
        let resources = remainder.concat(output);
        ProcessBuilder { resources }
    }

    pub async fn setup_async<F, Input, InputI, Output, Fut>(self, setup: F) -> ProcessBuilder<<R::Remainder as Concat>::Concatenated<Output>>
        where Output: 'static,
              R: IntoShape<Input, InputI>,
              R::Remainder: Concat,
              Fut: IntoFuture<Output=Output>,
              F: FnOnce(Input) -> Fut {
        let (input, remainder) = self.resources.into_shape();
        let output = setup(input).await;
        let resources = remainder.concat(output);
        ProcessBuilder { resources }
    }

    pub fn with<T>(self, resource: T) -> ProcessBuilder<(T, R)> {
        ProcessBuilder {
            resources: (resource, self.resources),
        }
    }

    pub fn build(self) -> Process<R> {
        Process::new(self.resources)
    }
}

pub struct MessageSender {
    sender: Sender<Box<dyn Event<Output = ()>>>,
}

impl MessageSender {
    fn new(sender: Sender<Box<dyn Event<Output = ()>>>) -> Self {
        MessageSender { sender }
    }

    pub fn send(&self, message: impl Event<Output = ()>) {
        //self.sender.send(Box::new(message)).unwrap();
    }
}

/// Represents the current process and holds a list of resources, produced by the [Platform], the
/// engine and the application. These resources are passed along to all event handlers when an
/// event is emitted.
pub struct Process<R> {
    resources: Resources<R>,
    event_system: EventSystem<Resources<R>>,
    receiver: Receiver<Box<dyn Event<Output = ()>>>,
}

impl<R: 'static> DerefMut for Process<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.resources()
    }
}

impl<R: 'static> Deref for Process<R> {
    type Target = Resources<R>;

    fn deref(&self) -> &Self::Target {
        &self.resources
    }
}

impl<R: 'static> Process<R> {
    fn new(resources: R) -> Process<R> {
        //let event_bus) = EventBus::new();
        let event_system = EventSystem::new();
        let (sender, receiver) = channel();
        //let resources = (MessageSender::new(sender), resources);

        Process {
            resources: Resources::new(resources),
            event_system,
            receiver,
        }
    }

    pub fn resources(&mut self) -> &mut Resources<R> {
        &mut self.resources
    }

    pub fn event_system(&mut self) -> &mut EventSystem<Resources<R>> {
        &mut self.event_system
    }

    pub fn handle_event<M: 'static + Event>(&mut self, message: M) -> Result<M::Output, M> {
        self.event_system.handle_event(message, &mut self.resources)
    }

    pub fn handle_generic_message(&mut self, message: Box<dyn Any>) -> Result<Box<dyn Any>, UnhandledEvent> {
        self.event_system.handle_generic_event(message, &mut self.resources)
    }
}

#[cfg(test)]
mod tests {
    use utils::{hlist, HList, delist};
    use crate::process::ProcessBuilder;

    struct ResourceA(u32);

    struct ResourceB(f32);

    struct ResourceC(&'static str);

    #[test]
    fn setup() {
        let mut process = ProcessBuilder::new()
            .setup(|_| hlist!(ResourceA(25u32)))
            .setup(|delist!(ResourceA(int)): HList!(ResourceA)| hlist!(ResourceA(int + 5), ResourceC("string")))
            .setup(|_s: HList!(ResourceC)| hlist!(ResourceB(0.7f32)))
            .build();

        let delist!(res_a, res_b) = process.get_some::<HList!(ResourceA, ResourceB), _>();
        assert_eq!(res_a.0, 30u32);
        assert_eq!(res_b.0, 0.7f32);
    }
}
