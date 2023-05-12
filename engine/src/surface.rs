use std::error::Error;
use std::ops::{Deref, DerefMut};
use utils::hlist::{Has};
use crate::events::EventSender;
use crate::process::{Process};

pub struct SurfaceResource<S> {
    surface: S,
}

impl<S> SurfaceResource<S> {
    pub fn new(surface: S) -> Self {
        SurfaceResource { surface }
    }
}

impl<S> DerefMut for SurfaceResource<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.surface
    }
}

impl<S> Deref for SurfaceResource<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.surface
    }
}

pub mod input {
    pub use winit::event::DeviceEvent;
    pub use winit::event::MouseScrollDelta;
    pub use winit::event::AxisId;
    pub use winit::event::ButtonId;
    pub use winit::event::ElementState;
    pub use winit::event::KeyboardInput;
    pub use winit::event::ScanCode;
    pub use winit::event::VirtualKeyCode;
}

pub enum SurfaceEvent {
    Resize {
        width: u32,
        height: u32,
    },
    Draw,
    CloseRequested,
    DeviceEvent(input::DeviceEvent),
}

pub enum Exit {
    Exit,
    Status(i32),
    Err(Box<dyn Error>),
}

impl<E: 'static + Error> From<E> for Exit {
    fn from(value: E) -> Self {
        Exit::Err(Box::new(value))
    }
}

impl Default for Exit {
    fn default() -> Self {
        Exit::Status(0)
    }
}

/// A surface that is able to be executed and produce [SurfaceEvents](SurfaceEvent) with the
/// resources available in the process.
pub trait RunnableSurface {
    type Output;

    fn run<R, IS, IE>(process: Process<R>) -> Self::Output
        where Self: Sized,
              R: 'static + Has<SurfaceResource<Self>, IS> + Has<EventSender, IE>;

    fn set_exit(&mut self, exit: Exit);
}

pub trait RunExt<R, S: RunnableSurface, IS, IE> {
    fn run(self) -> S::Output;
}

impl<R, S, IS, IE> RunExt<R, S, IS, IE> for Process<R>
    where S: RunnableSurface,
          R: 'static + Has<SurfaceResource<S>, IS> + Has<EventSender, IE> {
    fn run(self) -> S::Output {
        S::run(self)
    }
}
