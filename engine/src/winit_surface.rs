use std::error::Error;
use std::ops::Deref;
use async_trait::async_trait;
use frunk::hlist::{Plucker, Selector};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use render::{DeviceContext, SurfaceContext, WGPUContext};
use crate::process::ProcessBuilder;
use crate::resource::{ResourceList, ResourceListHas, Resources};

pub struct SurfaceConfigurationResource<S> {
    surface: S,
}

impl<S> SurfaceConfigurationResource<S> {
    pub fn surface(&self) -> &S {
        &self.surface
    }
}

pub type WindowSize = PhysicalSize<u32>;

pub trait HasWindow {
    type Window: HasRawWindowHandle + HasRawDisplayHandle;

    fn window(&self) -> &Self::Window;

    fn size(&self) -> WindowSize;
}

impl<S> Deref for SurfaceConfigurationResource<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.surface()
    }
}

pub struct WinitSurface {
    event_loop: EventLoop<()>,
    window: Window,
}

impl WinitSurface {
    pub fn window(&self) -> &Window {
        &self.window
    }
}

impl HasWindow for WinitSurface {
    type Window = Window;

    fn window(&self) -> &Self::Window {
        &self.window
    }

    fn size(&self) -> WindowSize {
        self.window.inner_size()
    }
}

pub type WinitSurfaceResource = SurfaceConfigurationResource<WinitSurface>;

pub trait WithWinitSurfaceExt<R: ResourceList> {
    fn with_winit_surface(self) -> ProcessBuilder<R::WithResource<WinitSurfaceResource>>;
}

impl<R: ResourceList + Send> WithWinitSurfaceExt<R> for ProcessBuilder<R> {
    fn with_winit_surface(self) -> ProcessBuilder<R::WithResource<WinitSurfaceResource>> {
        self.setup(|resources| {
            let event_loop = EventLoop::new();
            let window = WindowBuilder::new()
                .build(&event_loop).unwrap();

            resources.with_resource(SurfaceConfigurationResource {
                surface: WinitSurface { event_loop, window }
            })
        })
    }
}

pub enum SurfaceEvent {
    Resize {
        width: u32,
        height: u32,
    },
    Draw,
    CloseRequested,
}

pub enum SurfaceEventResult {
    Continue,
    Exit(Option<i32>),
    Err(Box<dyn Error>),
}

pub trait RunWinitSurfaceExt<R, I>
    where R: ResourceList,
          R::Resources: Plucker<WinitSurfaceResource, I>,
          <<R as ResourceList>::Resources as Plucker<SurfaceConfigurationResource<WinitSurface>, I>>::Remainder: 'static {
    fn run<F>(self, handler: F) -> !
        where F: FnMut(SurfaceEvent, &mut <R::Resources as Plucker<WinitSurfaceResource, I>>::Remainder) -> SurfaceEventResult + 'static;
}

impl<R, I> RunWinitSurfaceExt<R, I> for ProcessBuilder<R>
    where R: ResourceList,
          R::Resources: Plucker<WinitSurfaceResource, I>,
          <<R as ResourceList>::Resources as Plucker<SurfaceConfigurationResource<WinitSurface>, I>>::Remainder: 'static {
    fn run<F>(self, mut handler: F) -> !
        where F: FnMut(SurfaceEvent, &mut <R::Resources as Plucker<WinitSurfaceResource, I>>::Remainder) -> SurfaceEventResult + 'static {
        let resources = self.build().unpack();

        let (SurfaceConfigurationResource { surface }, mut resources): (WinitSurfaceResource, _) = resources.pluck();
        surface.event_loop.run(move |event, _, control_flow| {
            let result = match event {
                Event::RedrawRequested(window_id) if window_id == surface.window.id() => {
                    handler(SurfaceEvent::Draw, &mut resources)
                }
                Event::WindowEvent { event, window_id } if window_id == surface.window.id() => match event {
                    WindowEvent::Resized(PhysicalSize { width, height }) => {
                        handler(SurfaceEvent::Resize { width, height }, &mut resources)
                    }
                    WindowEvent::CloseRequested => {
                        handler(SurfaceEvent::CloseRequested, &mut resources)
                    }
                    _ => SurfaceEventResult::Continue,
                }
                _ => SurfaceEventResult::Continue,
            };
            match result {
                SurfaceEventResult::Continue => {},
                SurfaceEventResult::Exit(None) => control_flow.set_exit(),
                SurfaceEventResult::Exit(Some(code)) => control_flow.set_exit_with_code(code),
                SurfaceEventResult::Err(err) => panic!("error in surface event handler: {}", err),
            };
        })
    }
}

pub struct WGPURenderResource {
    wgpu_context: WGPUContext,
    surface_context: SurfaceContext,
    device_context: DeviceContext,
}

impl WGPURenderResource {
    pub fn wgpu_context(&self) -> &WGPUContext {
        &self.wgpu_context
    }

    pub fn get(&mut self) -> (&mut SurfaceContext, &mut DeviceContext) {
        (&mut self.surface_context, &mut self.device_context)
    }

    pub fn surface(&self) -> &SurfaceContext {
        &self.surface_context
    }

    pub fn surface_mut(&mut self) -> &mut SurfaceContext {
        &mut self.surface_context
    }

    pub fn device(&self) -> &DeviceContext {
        &self.device_context
    }

    pub fn device_mut(&mut self) -> &mut DeviceContext {
        &mut self.device_context
    }
}

#[async_trait(? Send)]
pub trait WGPURenderExt<R, I, W>
    where R: ResourceListHas<SurfaceConfigurationResource<W>, I>,
          W: HasWindow {
    async fn with_wgpu_render(self) -> ProcessBuilder<R::WithResource<WGPURenderResource>>;
}

#[async_trait(? Send)]
impl<R, I, W> WGPURenderExt<R, I, W> for ProcessBuilder<R>
    where R: ResourceList,
          R::Resources: Selector<SurfaceConfigurationResource<W>, I>,
          W: HasWindow, {
    async fn with_wgpu_render(self) -> ProcessBuilder<R::WithResource<WGPURenderResource>> {
        self.setup_async(|resources| async move {
            let window: &SurfaceConfigurationResource<W> = resources.get();

            let wgpu_context = WGPUContext::new().await.unwrap();
            let mut surface_context = wgpu_context.create_surface(window.surface.window());
            let device_context = wgpu_context.request_device(&surface_context).await.unwrap();

            let size = window.size();
            surface_context.configure(&device_context, size.width, size.height);

            resources.with_resource(WGPURenderResource {
                wgpu_context,
                surface_context,
                device_context,
            })
        }).await
    }
}
