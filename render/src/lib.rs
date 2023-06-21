use std::ops::Deref;

pub use wgpu::BufferUsages;

pub use color::Color;
pub use device_context::DeviceContext;
pub use maybe::*;
pub use render_api::{Batch, Model, RenderApi};
pub use surface_context::SurfaceContext;
pub use utils::Handle;
pub use vecbuf::VecBuf;
pub use wgpu_context::WGPUContext;

pub mod material;
pub mod geometry;
mod vecbuf;
mod color;
mod device_context;
mod surface_context;
mod render_api;
pub mod uniform;
mod maybe;
mod wgpu_context;
pub mod shader;

pub type TextureFormat = wgpu::TextureFormat;

pub struct Scene {}

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
