use wgpu::{DeviceDescriptor, RequestAdapterOptions};

use crate::genvec::{GenVec, Handle};

pub struct Pipeline {}

pub struct Buffer {
    buffer: wgpu::Buffer,
}

pub struct Renderer {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,

    pipelines: GenVec<Pipeline>,
    buffers: GenVec<Buffer>,
}

impl Renderer {
    async fn new<W>(window: &W, width: u32, height: u32) -> Renderer
        where W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle {
        let instance = wgpu::Instance::default();
        let adapter = instance.request_adapter(&RequestAdapterOptions::default())
            .await
            .expect("adapter");
        let (device, queue) = adapter.request_device(&DeviceDescriptor::default(), None)
            .await
            .expect("device");

        let surface = unsafe { instance.create_surface(window) }.expect("surface");
        let surface_config = surface.get_default_config(&adapter, width, height).expect("default surface configuration");
        surface.configure(&device, &surface_config);

        Renderer {
            instance,
            adapter,
            device,
            queue,
            surface,
            surface_config,
            pipelines: Default::default(),
            buffers: Default::default(),
        }
    }

    fn create_pipeline(&self, )
}

#[derive(Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

pub enum Target {
    ScreenTarget { clear: Option<Color> },
}

pub struct RenderPass {
    pub pipeline: Handle<Pipeline>,
    pub vertex_buffers: Vec<Handle<Buffer>>,
    pub targets: Vec<Target>,
}
