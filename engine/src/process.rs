use std::future::{IntoFuture};
use std::ops::{Deref, DerefMut};
use utils::hlist::{Concat, IntoShape};
use crate::events::{Event, EventBus};

pub struct ProcessInfo;

pub struct ProcessBuilder<R> {
    resources: R,
}

impl ProcessBuilder<()> {
    pub fn new() -> ProcessBuilder<()> {
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

    pub fn build(self) -> Process<R> {
        Process::new(self.resources)
    }
}

/// Represents the current process and holds a list of resources, produced by the [Platform], the
/// engine and the application. These resources are passed along to all event handlers when an
/// [Event] is emitted.
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
    fn new(resources: R) -> Self {
        Process {
            resources,
            event_bus: EventBus::default(),
        }
    }

    pub fn resources(&self) -> &R {
        &self.resources
    }

    pub fn resources_mut(&mut self) -> &mut R {
        &mut self.resources
    }

    pub fn event_bus(&mut self) -> &mut EventBus<R> {
        &mut self.event_bus
    }

    pub fn emit_event<E: Event>(&mut self, event: E) -> Option<E::Output> {
        self.event_bus.emit(&mut self.resources, event)
    }
}

#[cfg(test)]
mod tests {
    use utils::{hlist, HList};
    use utils::hlist::Has;
    use crate::process::ProcessBuilder;

    #[test]
    fn setup() {
        let list = ProcessBuilder::new()
            .setup(|_| hlist!(25u32))
            .setup(|i: HList!(u32)| hlist!(*i.get(), "string".to_owned()))
            .setup(|s: HList!(String)| hlist!(0.6f32, false))
            .build();
        assert_eq!(hlist!(25u32, 0.6f32, false), list.resources);
    }
}
