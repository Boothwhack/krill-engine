use async_trait::async_trait;
use winit::event_loop::{EventLoop, EventLoopBuilder};

use crate::platform::{Platform, SurfaceProvidingPlatform};

/// Platform backed by a winit event loop, and is capable of providing surfaces
/// using winit. Default backend for Desktop and Web platforms.
/// Could eventually be replaced with dedicated implementations for Windows, 
/// Linux (Wayland and X11), Mac, consoles, etc. platforms.
pub struct WinitPlatform {
    event_loop: EventLoop<PlatformEvent>,
}

impl WinitPlatform {
    pub fn new() -> Self {
        WinitPlatform { event_loop: EventLoopBuilder::with_user_event().build() }
    }
}

struct PlatformEvent {}

impl Platform for WinitPlatform {
    fn spawn_local<F, Fut>(self, _f: F)
        where Self: Sized,
              Fut: 'static + std::future::Future<Output=()>,
              F: FnOnce(Self) -> Fut {
        todo!()
    }
}

#[async_trait(?Send)]
impl SurfaceProvidingPlatform for WinitPlatform {
}
