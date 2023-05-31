use std::mem::swap;

use log::debug;
use never_say_never::Never;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use utils::{hlist, HList};
use utils::hlist::{Concat, Has, IntoShape};

use crate::events::EventSender;
use crate::process::{Process, ProcessBuilder};
use crate::surface::{Exit, RunnableSurface, SurfaceEvent, SurfaceResource};
use crate::wgpu_render::WGPUCompatible;

enum EventLoopState {
    Attached(EventLoop<()>),
    Detached,
}

impl From<EventLoop<()>> for EventLoopState {
    fn from(value: EventLoop<()>) -> Self {
        EventLoopState::Attached(value)
    }
}

impl EventLoopState {
    fn detach(&mut self) -> Option<EventLoop<()>> {
        let mut state = EventLoopState::Detached;
        swap(self, &mut state);

        match state {
            EventLoopState::Attached(event_loop) => Some(event_loop),
            _ => None,
        }
    }
}

pub struct WinitSurface {
    event_loop: EventLoopState,
    window: Window,
    exit: Option<Exit>,
}

impl WGPUCompatible for WinitSurface {
    type RawWindow = Window;

    fn raw_window(&self) -> &Self::RawWindow {
        &self.window
    }

    fn size(&self) -> (u32, u32) {
        let PhysicalSize { width, height } = self.window.inner_size();
        (width, height)
    }
}

pub fn setup_winit_resource() -> SurfaceResource<WinitSurface> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    SurfaceResource::new(WinitSurface {
        event_loop: event_loop.into(),
        window,
        exit: None,
    })
}

pub trait WinitSetupExt<R, I>
    where
        R: 'static + IntoShape<(), I>,
        R::Remainder: Concat,
{
    type Output;

    fn setup_winit(self) -> Self::Output;
}

impl<R, I> WinitSetupExt<R, I> for ProcessBuilder<R>
    where
        R: 'static + IntoShape<(), I>,
        R::Remainder: Concat,
{
    type Output = ProcessBuilder<<R::Remainder as Concat>::Concatenated<HList!(SurfaceResource<WinitSurface>)>>;

    fn setup_winit(self) -> Self::Output
    {
        self.setup(|_: HList!()| hlist!(setup_winit_resource()))
    }
}

impl RunnableSurface for WinitSurface {
    type Output = Never;

    fn run<R, IS, IE>(mut process: Process<R>) -> Self::Output
        where R: 'static + Has<SurfaceResource<WinitSurface>, IS> + Has<EventSender, IE> {
        let surface: &mut SurfaceResource<_> = process.get_mut();
        let event_loop = surface
            .event_loop
            .detach()
            .expect("this is the only place that detaches, and never returns");
        let window = surface.window.id();

        debug!(target: "krill::surface::winit", "Starting event loop.");

        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::RedrawRequested(window_id) if window_id == window => {
                    process.send_event(SurfaceEvent::Draw).unwrap();
                }
                Event::RedrawEventsCleared => {
                    process.dispatch_events().unwrap();

                    let surface: &SurfaceResource<_> = process.get();
                    surface.window.request_redraw();
                }
                Event::WindowEvent { event, window_id } if window_id == window => {
                    match event {
                        WindowEvent::Resized(PhysicalSize { width, height }) => {
                            process.send_event(SurfaceEvent::Resize { width, height }).unwrap();
                        }
                        WindowEvent::CloseRequested => {
                            process.send_event(SurfaceEvent::CloseRequested).unwrap();
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            process.send_event(SurfaceEvent::DeviceEvent(DeviceEvent::Key(input))).unwrap();
                        }
                        _ => {}
                    }
                }
                Event::DeviceEvent { event, .. } => {
                    process.send_event(SurfaceEvent::DeviceEvent(event)).unwrap();
                }
                _ => {},
            };

            let surface: &mut SurfaceResource<_> = process.resources_mut().get_mut();
            match surface.exit.take() {
                Some(Exit::Exit) => control_flow.set_exit(),
                Some(Exit::Status(code)) => control_flow.set_exit_with_code(code),
                Some(Exit::Err(err)) => panic!("error in surface event handler: {}", err),
                _ => {}
            };
        })
    }

    fn set_exit(&mut self, exit: Exit) {
        self.exit = Some(exit)
    }
}
