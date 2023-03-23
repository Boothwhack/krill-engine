use std::iter::once;
use std::mem::size_of;
use std::ops::Range;
use wgpu::{BufferDescriptor, BufferUsages, include_wgsl, Label, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, ShaderModuleDescriptor, TextureViewDescriptor, vertex_attr_array, VertexBufferLayout};

use crate::genvec::{GenVec, Handle};

pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
}

pub struct Buffer {
    buffer: wgpu::Buffer,
    size: usize,
    usage: BufferUsages,
}

#[derive(Default)]
struct Resources {
    pipelines: GenVec<Pipeline>,
    buffers: GenVec<Buffer>,
}

pub struct WGPUContext {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
}

impl WGPUContext {
    pub async fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();
        println!("Adapters:");
        for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
            println!("  {:?}", adapter.get_info());
        }
        let adapter = instance.request_adapter(&RequestAdapterOptions::default())
            .await?;

        Some(WGPUContext { instance, adapter })
    }

    pub async fn request_device(&self) -> Result<DeviceContext, wgpu::RequestDeviceError> {
        let (device, queue) = self.adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
            None,
        ).await?;
        Ok(DeviceContext { device, queue, resources: Resources::default() })
    }

    pub fn create_surface<W>(&self, window: &W, device_context: &DeviceContext, width: u32, height: u32) -> SurfaceContext
        where W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle {
        let surface = unsafe { self.instance.create_surface(window) }.expect("surface");
        let mut surface_config = surface.get_default_config(&self.adapter, width, height).expect("default surface configuration");
        println!("Surface capabilities: {:?}", surface.get_capabilities(&self.adapter));
        println!("Default surface configuration: {:?}", surface_config);
        surface_config.format = wgpu::TextureFormat::Rgba8UnormSrgb;
        surface.configure(&device_context.device, &surface_config);
        SurfaceContext {
            surface,
            surface_config,
        }
    }
}

pub struct VertexShader<'a> {
    pub source: &'a str,
}

pub struct DeviceContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    resources: Resources,
}

impl DeviceContext {
    pub fn create_buffer(&mut self, size: usize, usage: BufferUsages) -> Handle<Buffer> {
        let buffer = self.device.create_buffer(&BufferDescriptor {
            label: Label::default(),
            usage,
            mapped_at_creation: false,
            size: size as _,
        });
        let buffer = Buffer {
            buffer,
            size,
            usage,
        };

        self.resources.buffers.add(buffer)
    }

    pub fn resize_buffer(&mut self, buffer: &Handle<Buffer>, size: usize) {
        let mut buffer = self.resources.buffers.get_mut(buffer)
            .expect("cannot resize deleted buffer");

        if buffer.size < size {
            buffer.buffer = self.device.create_buffer(&BufferDescriptor {
                label: Label::default(),
                size: size as _,
                usage: buffer.usage,
                mapped_at_creation: false,
            });
            buffer.size = size;
        }
    }

    pub fn create_pipeline(&mut self, vertex_shader: &VertexShader /* TODO: fragment_shader */) -> Handle<Pipeline> {
        let shader_source = self.device.create_shader_module(ShaderModuleDescriptor {
            label: Label::default(),
            source: wgpu::ShaderSource::Wgsl(vertex_shader.source.into()),
        });
        let fragment_shader_module = self.device.create_shader_module(include_wgsl!("frag_simple.wgsl"));

        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Label::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format:wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            vertex: wgpu::VertexState {
                module: &shader_source,
                entry_point: "vs_main",
                buffers: &[
                    VertexBufferLayout {
                        step_mode: wgpu::VertexStepMode::Vertex,
                        array_stride: (size_of::<f32>() * 2) as _,
                        attributes: &vertex_attr_array![0 => Float32x2],
                    }
                ],
            },
            layout: None,
            primitive: wgpu::PrimitiveState::default(),
            multiview: None,
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
        });
        self.resources.pipelines.add(Pipeline { pipeline })
    }

    pub fn command_encoder<'a>(&'a self, frame: &'a SurfaceFrame) -> CommandEncoderContext<'a> {
        CommandEncoderContext {
            encoder: self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
            resources: &self.resources,
            frame,
        }
    }

    pub fn submit_commands(&self, encoder: CommandEncoderContext) {
        let buffer = encoder.encoder.finish();
        self.queue.submit(once(buffer));
    }

    pub fn submit_buffer(&self, buffer: &Handle<Buffer>, offset: usize, data: &[u8]) {
        let buffer = self.resources.buffers.get(buffer)
            .expect("cannot write to deleted buffer");
        self.queue.write_buffer(&buffer.buffer, offset as _, data);
    }
}

pub struct SurfaceContext {
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
}

impl SurfaceContext {
    pub fn get_frame(&self) -> SurfaceFrame {
        SurfaceFrame {
            surface_texture: self.surface.get_current_texture()
                .expect("current surface texture for frame"),
        }
    }

    pub fn present(&self, frame: SurfaceFrame) {
        frame.surface_texture.present();
    }
}

pub struct SurfaceFrame {
    surface_texture: wgpu::SurfaceTexture,
}

pub struct CommandEncoderContext<'a> {
    resources: &'a Resources,
    encoder: wgpu::CommandEncoder,
    frame: &'a SurfaceFrame,
}

impl<'a> CommandEncoderContext<'a> {
    pub fn render_pass(&mut self, pass: &RenderPass) {
        let surface_view = self.frame.surface_texture.texture.create_view(&TextureViewDescriptor::default());
        let attachments = pass.targets.iter().map(|target| {
            match target {
                Target::None => None,
                Target::ScreenTarget { clear } => {
                    Some(RenderPassColorAttachment {
                        resolve_target: None,
                        view: &surface_view,
                        ops: Operations {
                            load: match clear {
                                None => LoadOp::Load,
                                Some(color) => LoadOp::Clear(color.into()),
                            },
                            store: true,
                        },
                    })
                }
            }
        }).collect::<Vec<_>>();
        let mut encoder_pass = self.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Label::default(),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
        });

        let pipeline = self.resources.pipelines.get(&pass.pipeline)
            .expect("cannot draw with deleted pipeline");

        encoder_pass.set_pipeline(&pipeline.pipeline);

        for (idx, buffer) in pass.vertex_buffers.iter().enumerate() {
            if let Some(buffer) = buffer {
                let buffer = self.resources.buffers.get(buffer)
                    .expect("cannot draw with deleted buffer");

                encoder_pass.set_vertex_buffer(idx as u32, buffer.buffer.slice(..));
            }
        }
        encoder_pass.draw(pass.vertices.clone(), 0..1);
    }
}

#[derive(Default, Copy, Clone)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color {r,g,b,a}
    }
}

impl Into<wgpu::Color> for &Color {
    fn into(self) -> wgpu::Color {
        wgpu::Color {
            a: self.a as f64,
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
        }
    }
}

pub enum Target {
    None,
    ScreenTarget { clear: Option<Color> },
}

pub struct RenderPass {
    pub pipeline: Handle<Pipeline>,
    pub vertex_buffers: Vec<Option<Handle<Buffer>>>,
    pub targets: Vec<Target>,
    pub vertices: Range<u32>,
}
