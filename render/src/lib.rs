use std::ops::Deref;

pub use wgpu::BufferUsages;

pub use color::Color;
pub use device_context::DeviceContext;
pub use render_api::{Batch, Model, RenderApi};
pub use surface_context::SurfaceContext;
pub use utils::Handle;
pub use vecbuf::VecBuf;

pub mod bindgroup;
pub mod pipeline;
pub mod material;
pub mod geometry;
mod vecbuf;
mod color;
mod device_context;
mod surface_context;
mod render_api;
pub mod uniform;

pub type TextureFormat = wgpu::TextureFormat;

pub enum MaybeOwned<T> {
    Handle(Handle<T>),
    Owned(T),
}

pub enum MaybeRef<'a, T> {
    Handle(Handle<T>),
    Ref(&'a mut T),
}

impl<T> From<Handle<T>> for MaybeOwned<T> {
    fn from(value: Handle<T>) -> Self {
        MaybeOwned::Handle(value)
    }
}

impl<T> From<T> for MaybeOwned<T> {
    fn from(value: T) -> Self {
        MaybeOwned::Owned(value)
    }
}

impl<'a, T> From<Handle<T>> for MaybeRef<'a, T> {
    fn from(value: Handle<T>) -> Self {
        MaybeRef::Handle(value)
    }
}

impl<'a, T> From<&'a mut T> for MaybeRef<'a, T> {
    fn from(value: &'a mut T) -> Self {
        MaybeRef::Ref(value)
    }
}

pub struct Scene {}

pub struct WGPUContext {
    instance: wgpu::Instance,
}

impl WGPUContext {
    // enumerate_adapters is not available in wasm environments
    #[cfg(not(target_family = "wasm"))]
    fn log_adapters(instance: &wgpu::Instance) {
        log::info!("Adapters:");
        for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
            log::info!("  {:?}", adapter.get_info());
        }
    }

    pub async fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();

        #[cfg(not(target_family = "wasm"))]
        WGPUContext::log_adapters(&instance);

        log::info!("Got WGPU instance.");

        Some(WGPUContext { instance })
    }

    pub async fn request_device(&self, surface: &SurfaceContext) -> Result<DeviceContext, wgpu::RequestDeviceError> {
        let adapter = self.instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface.surface),
            ..Default::default()
        }).await.expect("viable adapter");
        log::info!("Got adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                limits: wgpu::Limits::downlevel_webgl2_defaults(),

                ..Default::default()
            },
            None,
        ).await?;
        Ok(DeviceContext::new(adapter, device, queue))
    }

    pub fn create_surface<W>(&self, window: &W) -> SurfaceContext
        where W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle {
        log::info!("Creating surface...");
        let surface = unsafe { self.instance.create_surface(window) }.expect("surface");

        SurfaceContext {
            surface,
            surface_config: None,
        }
    }
}

pub struct MutableHandle<'a, T> {
    pub(crate) resource: &'a mut T,
    pub(crate) context: &'a DeviceContext,
}

impl<'a, T> MutableHandle<'a, T> {
    pub fn from_ref(context: &'a DeviceContext, resource: &'a mut T) -> Self {
        MutableHandle { context, resource }
    }
}

impl<'a, T> Deref for MutableHandle<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.resource
    }
}

pub struct Frame {
    surface_texture: wgpu::SurfaceTexture,
}

pub enum Target {
    None,
    ScreenTarget { clear: Option<Color> },
}
