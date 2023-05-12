use std::future::{IntoFuture};
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::SendError;
use utils::hlist::{Concat, Has, IntoShape};
use crate::events::{EventBus, EventSender, InvalidEvent, Listeners};

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

    pub fn build(self) -> Process<(EventSender, R)> {
        Process::new(self.resources)
    }
}

/// Represents the current process and holds a list of resources, produced by the [Platform], the
/// engine and the application. These resources are passed along to all event handlers when an
/// event is emitted.
pub struct Process<R> {
    resources: R,
    event_bus: EventBus<R>,
}

impl<R: 'static> DerefMut for Process<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.resources_mut()
    }
}

impl<R: 'static> Deref for Process<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.resources()
    }
}

impl<R: 'static> Process<R> {
    fn new(resources: R) -> Process<(EventSender, R)> {
        let (sender, event_bus) = EventBus::new();

        Process {
            resources: (sender, resources),
            event_bus,
        }
    }

    pub fn resources(&self) -> &R {
        &self.resources
    }

    pub fn resources_mut(&mut self) -> &mut R {
        &mut self.resources
    }

    pub fn dispatch_events(&mut self) -> Result<(), InvalidEvent> {
        self.event_bus.dispatch_all(&mut self.resources)
    }

    pub fn send_event<E: 'static, I>(&self, event: E) -> Result<(), SendError<E>>
        where R: Has<EventSender, I> {
        self.get().send(event)
    }

    pub fn event_listeners<E: 'static>(&mut self, listeners: Listeners<E, R>) {
        self.event_bus.register_event(listeners);
    }
}

#[cfg(test)]
mod tests {
    use utils::{hlist, HList};
    use utils::hlist::Has;
    use crate::process::ProcessBuilder;

    #[test]
    fn setup() {
        let process = ProcessBuilder::new()
            .setup(|_| hlist!(25u32))
            .setup(|i: HList!(u32)| hlist!(*i.get(), "string".to_owned()))
            .setup(|s: HList!(String)| hlist!(0.6f32, false))
            .build();
        assert_eq!(hlist!(25u32, 0.6f32, false), process.resources.1);
    }
}
