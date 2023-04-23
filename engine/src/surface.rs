use std::error::Error;
use std::ops::{Deref, DerefMut};
use utils::hlist::{Has, IntoShape};
use crate::process::ProcessBuilder;

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

pub enum SurfaceEventResult {
    Continue,
    Exit(Option<i32>),
    Err(Box<dyn Error>),
}

pub trait RunnableSurface<'a> {
    type Output;

    fn run<F, R>(self, handler: F, resources: R) -> Self::Output
        where R: 'a,
              F: 'a + for<'b> FnMut(SurfaceEvent, &'b mut R) -> SurfaceEventResult;
}

pub trait RunExt<'a, R, S: RunnableSurface<'a>, I> {
    fn run<T, TI, F>(self, handler: F) -> S::Output
        where R: Has<SurfaceResource<S>, I>,
              R::Remainder: IntoShape<T, TI>,
              T: 'a,
              F: 'a + for<'b> FnMut(SurfaceEvent, &'b mut T) -> SurfaceEventResult;
}

impl<'a, R, S, I> RunExt<'a, R, S, I> for ProcessBuilder<R>
    where S: RunnableSurface<'a>, {
    fn run<T, TI, F>(self, handler: F) -> S::Output
        where R: Has<SurfaceResource<S>, I>,
              R::Remainder: IntoShape<T, TI>,
              T: 'a,
              F: 'a + for<'b> FnMut(SurfaceEvent, &'b mut T) -> SurfaceEventResult {
        let (surface, resources) = self.build().pick();
        surface.surface.run(handler, resources.into_shape().0)
    }
}
