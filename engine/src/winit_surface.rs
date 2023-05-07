use std::mem::swap;
use std::ops::ControlFlow;
use crate::process::{Process, ProcessBuilder};
use crate::surface::{Exit, RunnableSurface, SurfaceEvent, SurfaceResource};
use crate::wgpu_render::WGPUCompatible;
use never_say_never::Never;
use utils::hlist::{Concat, Has, IntoShape};
use utils::{hlist, HList};
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::platform::web::WindowExtWebSys;
use winit::window::{Window, WindowBuilder};
use crate::winit_surface::web::{CanvasEvent, Placement};

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
    SurfaceResource::new(WinitSurface { event_loop: event_loop.into(), window })
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

#[cfg(target_family = "wasm")]
mod web {
    use web_sys::{HtmlCanvasElement};
    use crate::events::Event;

    pub struct CanvasEvent {
        pub canvas: HtmlCanvasElement,
    }

    impl Event for CanvasEvent {
        type Output = Placement;
    }

    pub enum Placement {
        /// The canvas element will be placed into the DOM in the `<body>` tag.
        Default(HtmlCanvasElement),
        /// The canvas element has either been placed into the DOM by the application or does not
        /// need to be placed.
        DontPlace,
    }
}

#[cfg(target_family = "wasm")]
fn handle_canvas_on_web<R, I>(process: &mut Process<R>)
    where R: 'static + Has<SurfaceResource<WinitSurface>, I> {
    let canvas = process.get().window.canvas();

    match process.emit_event(CanvasEvent { canvas }) {
        Some(Placement::Default(canvas)) => {
            web_sys::window().unwrap()
                .document().unwrap()
                .body().unwrap()
                .append_child(&canvas)
                .expect("canvas to be placed");
        }
        _ => {}
    };
}

#[cfg(not(target_family = "wasm"))]
fn handle_canvas_on_web<R>(_process: &Process<R>) {}

impl RunnableSurface for WinitSurface {
    type Output = Never;

    fn run<R, I>(mut process: Process<R>) -> Self::Output
        where R: 'static + Has<SurfaceResource<WinitSurface>, I>
    {
        handle_canvas_on_web(&mut process);

        let event_loop = process.get_mut()
            .event_loop
            .detach()
            .expect("this is the only place that detaches, and never returns");
        let window = process.get().window.id();

        event_loop.run(move |event, _, control_flow| {
            let result = match event {
                Event::RedrawRequested(window_id) if window_id == window => {
                    process.emit_event(SurfaceEvent::Draw)
                }
                Event::RedrawEventsCleared => {
                    process.get().window.request_redraw();
                    None
                }
                Event::WindowEvent { event, window_id } if window_id == window => {
                    match event {
                        WindowEvent::Resized(PhysicalSize { width, height }) => {
                            process.emit_event(SurfaceEvent::Resize { width, height })
                        }
                        WindowEvent::CloseRequested => {
                            process.emit_event(SurfaceEvent::CloseRequested)
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            process.emit_event(SurfaceEvent::DeviceEvent(DeviceEvent::Key(input)))
                        }
                        _ => None,
                    }
                }
                Event::DeviceEvent { event, .. } => {
                    process.emit_event(SurfaceEvent::DeviceEvent(event))
                }
                _ => None,
            };
            match result {
                Some(ControlFlow::Break(Exit::Exit)) => control_flow.set_exit(),
                Some(ControlFlow::Break(Exit::Status(code))) => control_flow.set_exit_with_code(code),
                Some(ControlFlow::Break(Exit::Err(err))) => panic!("error in surface event handler: {}", err),

                _ => {}
            };
        })
    }
}
