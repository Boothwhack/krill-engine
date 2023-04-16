use async_trait::async_trait;
use frunk::hlist::{Plucker, Selector};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle};
use winit::event::Event;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use render::{DeviceContext, SurfaceContext, WGPUContext};
use crate::process::ProcessBuilder;
use crate::resource::{ResourceList, ResourceListHas, Resources};

pub struct SurfaceResource<S> {
    surface: S,
}

pub struct WinitSurface {
    event_loop: EventLoop<()>,
    window: Window,
}

unsafe impl HasRawWindowHandle for WinitSurface {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.window.raw_window_handle()
    }
}

unsafe impl HasRawDisplayHandle for WinitSurface {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.window.raw_display_handle()
    }
}

pub type WinitSurfaceResource = SurfaceResource<WinitSurface>;

pub trait WithWinitSurfaceExt<R: ResourceList> {
    fn with_winit_surface(self) -> ProcessBuilder<R::WithResource<WinitSurfaceResource>>;
}

impl<R: ResourceList + Send> WithWinitSurfaceExt<R> for ProcessBuilder<R> {
    fn with_winit_surface(self) -> ProcessBuilder<R::WithResource<WinitSurfaceResource>> {
        self.setup(|resources| {
            let event_loop = EventLoop::new();
            let window = WindowBuilder::new().build(&event_loop).unwrap();

            resources.with_resource(SurfaceResource {
                surface: WinitSurface { event_loop, window }
            })
        })
    }
}

pub enum SurfaceEvent {
    Draw,
    Close,
}

pub trait RunWinitSurfaceExt<R, I>
    where R: ResourceList,
          R::Resources: Plucker<WinitSurfaceResource, I>,
          <<R as ResourceList>::Resources as Plucker<SurfaceResource<WinitSurface>, I>>::Remainder: 'static {
    fn run<F>(self, handler: F) -> !
        where F: FnMut(SurfaceEvent, &mut <R::Resources as Plucker<WinitSurfaceResource, I>>::Remainder) + 'static;
}

impl<R, I> RunWinitSurfaceExt<R, I> for ProcessBuilder<R>
    where R: ResourceList,
          R::Resources: Plucker<WinitSurfaceResource, I>,
          <<R as ResourceList>::Resources as Plucker<SurfaceResource<WinitSurface>, I>>::Remainder: 'static {
    fn run<F>(self, mut handler: F) -> !
        where F: FnMut(SurfaceEvent, &mut <R::Resources as Plucker<WinitSurfaceResource, I>>::Remainder) + 'static {
        let resources = self.build().unpack();
        let (SurfaceResource { surface }, mut resources): (WinitSurfaceResource, _) = resources.pluck();
        surface.event_loop.run(move |event, _, control_flow| match event {
            Event::RedrawRequested(window_id) if window_id == surface.window.id() => {
                handler(SurfaceEvent::Draw, &mut resources);
            }
            _ => {}
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

    pub fn surface(&self) -> &SurfaceContext {
        &self.surface_context
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
    where R: ResourceListHas<SurfaceResource<W>, I>,
          W: HasRawWindowHandle + HasRawDisplayHandle {
    async fn with_wgpu_render(self) -> ProcessBuilder<R::WithResource<WGPURenderResource>>;
}

#[async_trait(? Send)]
impl<R, I, W> WGPURenderExt<R, I, W> for ProcessBuilder<R>
    where R: ResourceList,
          R::Resources: Selector<SurfaceResource<W>, I>,
          W: HasRawWindowHandle + HasRawDisplayHandle {
    async fn with_wgpu_render(self) -> ProcessBuilder<R::WithResource<WGPURenderResource>> {
        self.setup_async(|resources| async move {
            let window: &SurfaceResource<W> = resources.get();

            let wgpu_context = WGPUContext::new().await.unwrap();
            let mut surface_context = wgpu_context.create_surface(&window.surface);
            let device_context = wgpu_context.request_device(&surface_context).await.unwrap();

            surface_context.configure(&device_context, 800 , 600);

            resources.with_resource(WGPURenderResource {
                wgpu_context,
                surface_context,
                device_context,
            })
        }).await
    }
}
