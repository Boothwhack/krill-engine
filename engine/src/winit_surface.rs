use crate::process::{ProcessBuilder};
use crate::surface::{RunnableSurface, SurfaceEvent, SurfaceEventResult, SurfaceResource};
use crate::wgpu_render::WGPUCompatible;
use never_say_never::Never;
use utils::hlist::{Concat, IntoShape};
use utils::{hlist, HList};
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub struct WinitSurface {
    event_loop: EventLoop<()>,
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

fn setup_winit_resource() -> SurfaceResource<WinitSurface> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    SurfaceResource::new(WinitSurface { event_loop, window })
}

pub trait WinitSetupExt<R, I>
    where
        R: IntoShape<(), I>,
        R::Remainder: Concat,
{
    type Output;

    fn setup_winit(self) -> Self::Output;
}

impl<R, I> WinitSetupExt<R, I> for ProcessBuilder<R>
    where
        R: IntoShape<(), I>,
        R::Remainder: Concat,
{
    type Output = ProcessBuilder<<R::Remainder as Concat>::Concatenated<HList!(SurfaceResource<WinitSurface>)>>;

    fn setup_winit(
        self,
    ) -> ProcessBuilder<<R::Remainder as Concat>::Concatenated<HList!(SurfaceResource<WinitSurface>)>>
    {
        self.setup(|_: HList!()| hlist!(setup_winit_resource()))
    }
}

impl RunnableSurface<'static> for WinitSurface {
    type Output = Never;

    fn run<F, R>(self, mut handler: F, mut resources: R) -> Self::Output
        where
            R: 'static,
            F: 'static + for<'a> FnMut(SurfaceEvent, &'a mut R) -> SurfaceEventResult,
    {
        self.event_loop.run(move |event, _, control_flow| {
            let result = match event {
                Event::RedrawRequested(window_id) if window_id == self.window.id() => {
                    handler(SurfaceEvent::Draw, &mut resources)
                }
                Event::RedrawEventsCleared => {
                    self.window.request_redraw();
                    SurfaceEventResult::Continue
                }
                Event::WindowEvent { event, window_id } if window_id == self.window.id() => {
                    match event {
                        WindowEvent::Resized(PhysicalSize { width, height }) => {
                            handler(
                                SurfaceEvent::Resize { width, height },
                                &mut resources,
                            )
                        }
                        WindowEvent::CloseRequested => {
                            handler(
                                SurfaceEvent::CloseRequested,
                                &mut resources,
                            )
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            handler(
                                SurfaceEvent::DeviceEvent(DeviceEvent::Key(input)),
                                &mut resources,
                            )
                        }
                        _ => SurfaceEventResult::Continue,
                    }
                }
                Event::DeviceEvent { event, .. } => {
                    handler(
                        SurfaceEvent::DeviceEvent(event),
                        &mut resources,
                    )
                }
                _ => SurfaceEventResult::Continue,
            };
            match result {
                SurfaceEventResult::Continue => {}
                SurfaceEventResult::Exit(None) => control_flow.set_exit(),
                SurfaceEventResult::Exit(Some(code)) => control_flow.set_exit_with_code(code),
                SurfaceEventResult::Err(err) => panic!("error in surface event handler: {}", err),
            };
        })
    }
}
