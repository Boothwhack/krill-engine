use wgpu::{Adapter, Backends, Device, DeviceDescriptor, Instance, InstanceDescriptor, PowerPreference, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration, SurfaceError, SurfaceTexture, TextureUsages};

pub struct RenderApi {
    instance: Instance,
    surface: Surface,
    adapter: Adapter,
    device: Device,
    queue: Queue,
}

#[derive(Debug)]
pub enum FrameError {
    Suboptimal,
    SurfaceError(SurfaceError),
}

impl RenderApi {
    pub fn new<W>(window: &W) -> RenderApi
        where W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle {
        let instance = Instance::default();
        let surface = unsafe { instance.create_surface(window) }.unwrap();
        let adapter = futures::executor::block_on(instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        })).unwrap();
        let (device, queue) = futures::executor::block_on(adapter.request_device(&DeviceDescriptor::default(), None)).unwrap();

        RenderApi { instance, surface, adapter, device, queue }
    }

    pub fn configure_surface(&mut self, width: u32, height: u32) {
        let capabilities = self.surface.get_capabilities(&self.adapter);
        let format = capabilities
            .formats
            .iter()
            .find(|it| it.describe().srgb)
            .unwrap_or(&capabilities.formats[0])
            .clone();

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: Default::default(),
            view_formats: vec!(),
        };
        self.surface.configure(&self.device, &config);
    }

    pub fn begin_frame(&self) -> Result<Frame, FrameError> {
        let surface_texture = self.surface.get_current_texture().map_err(FrameError::SurfaceError)?;
        if surface_texture.suboptimal {
            return Err(FrameError::Suboptimal);
        }

        Ok(Frame {
            surface_texture,
            render_passes: vec!(),
        })
    }

    pub fn submit_frame(&mut self, frame: Frame) {
        frame.surface_texture.present();
    }
}

pub struct Frame {
    surface_texture: SurfaceTexture,
    render_passes: Vec<RenderPass>,
}

impl Frame {
    pub fn begin_render_pass(&self) -> RenderPass {
        RenderPass {
            commands: vec!(),
        }
    }

    pub fn submit_render_pass(&mut self, render_pass: RenderPass) {
        self.render_passes.push(render_pass);
    }
}

pub enum ColorAttachmentSource {
    SurfaceTexture,
}

pub struct ColorAttachment {
    color: wgpu::Color,
    source: ColorAttachmentSource,
}

pub struct RenderPass<'a> {
    color_attachments: &'a [Option<ColorAttachment>],
    commands: Vec<Command>,
}

enum Command {}
