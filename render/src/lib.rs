pub mod pipeline;

use std::collections::HashMap;
use std::iter::once;
use std::mem::{size_of};
use std::ops::Range;

use wgpu::{BufferDescriptor, ColorTargetState, FragmentState, include_wgsl, Label, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor, ShaderSource, TextureViewDescriptor, vertex_attr_array, VertexState};

pub use wgpu::BufferUsages;
use utils::{CompactList};
pub use utils::Handle;
use crate::pipeline::RenderPipelineAsset;
use crate::pipeline::serial::{TargetFormat, VertexFormatDefinition, VertexShaderStepMode};

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
    pipelines: CompactList<Pipeline>,
    buffers: CompactList<Buffer>,
}

pub struct WGPUContext {
    instance: wgpu::Instance,
}

impl WGPUContext {
    pub async fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();
        println!("Adapters:");
        for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
            println!("  {:?}", adapter.get_info());
        }

        Some(WGPUContext { instance })
    }

    pub async fn request_device(&self, surface: &SurfaceContext) -> Result<DeviceContext, wgpu::RequestDeviceError> {
        let adapter = self.instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: Some(&surface.surface),
            ..Default::default()
        }).await.expect("viable adapter");
        println!("Got adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
            None,
        ).await?;
        Ok(DeviceContext { adapter, device, queue, resources: Resources::default() })
    }

    pub fn create_surface<W>(&self, window: &W) -> SurfaceContext
        where W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle {
        let surface = unsafe { self.instance.create_surface(window) }.expect("surface");

        SurfaceContext {
            surface,
            surface_config: None,
        }
    }
}

pub struct VertexShader<'a> {
    pub source: &'a str,
}

pub struct DeviceContext {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    resources: Resources,
}

struct OwnedVertexBufferLayout {
    step_mode: wgpu::VertexStepMode,
    array_stride: u64,
    attributes: Vec<wgpu::VertexAttribute>,
}

impl OwnedVertexBufferLayout {
    fn to_ref(&self) -> wgpu::VertexBufferLayout {
        wgpu::VertexBufferLayout {
            step_mode: self.step_mode,
            array_stride: self.array_stride,
            attributes: self.attributes.as_slice(),
        }
    }
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

    pub fn get_buffer(&self, buffer: Handle<Buffer>) -> Option<&Buffer> {
        self.resources.buffers.get(buffer)
    }

    fn ensure_buffer_capacity(device: &wgpu::Device, buffer: &mut Buffer, size: usize) {
        if buffer.size < size {
            buffer.buffer = device.create_buffer(&BufferDescriptor {
                label: Label::default(),
                size: size as _,
                usage: buffer.usage,
                mapped_at_creation: false,
            });
            buffer.size = size;
        }
    }

    pub fn resize_buffer(&mut self, buffer: Handle<Buffer>, size: usize) {
        let buffer = self.resources.buffers.get_mut(buffer)
            .expect("cannot resize deleted buffer");

        Self::ensure_buffer_capacity(&self.device, buffer, size);
    }

    // TODO: Clean up!
    pub fn create_pipeline(&mut self, asset: RenderPipelineAsset) -> Handle<Pipeline> {
        let modules: HashMap<String, _> = asset.shader_modules
            .into_iter()
            .map(|(name, source)| {
                let shader =self.device.create_shader_module(ShaderModuleDescriptor {
                    label: Some(&name),
                    source: ShaderSource::Wgsl(source.into()),
                });
                (
                    name,
                    shader,
                )
            })
            .collect();

        let fragment_targets = asset.definition.fragment_shader
            .as_ref()
            .map(|frag| frag.targets
                .iter()
                .map(|target| Some(ColorTargetState {
                    format: match target.format {
                        TargetFormat::BGRA8UnormSRGB => wgpu::TextureFormat::Bgra8UnormSrgb,
                        TargetFormat::RGBA8UnormSRGB => wgpu::TextureFormat::Rgba8UnormSrgb,
                    },
                    blend: None,
                    write_mask: wgpu::ColorWrites::all(),
                }))
                .collect::<Vec<_>>()
            );
        let vertex_buffers = asset.definition.vertex_shader.buffers
            .iter()
            .map(|buffer| {
                let mut offset_accumulator: u64 = 0;
                OwnedVertexBufferLayout {
                    step_mode: match buffer.step_mode {
                        VertexShaderStepMode::Vertex => wgpu::VertexStepMode::Vertex,
                        VertexShaderStepMode::Instance => wgpu::VertexStepMode::Instance,
                    },
                    array_stride: buffer.stride(),
                    attributes: buffer.attributes.iter()
                        .enumerate()
                        .map(|(i, attr)| {
                            let format = match attr.format {
                                VertexFormatDefinition::Float32(1) => wgpu::VertexFormat::Float32,
                                VertexFormatDefinition::Float32(2) => wgpu::VertexFormat::Float32x2,
                                VertexFormatDefinition::Float32(3) => wgpu::VertexFormat::Float32x3,
                                VertexFormatDefinition::Float32(4) => wgpu::VertexFormat::Float32x4,

                                VertexFormatDefinition::Float64(1) => wgpu::VertexFormat::Float64,
                                VertexFormatDefinition::Float64(2) => wgpu::VertexFormat::Float64x2,
                                VertexFormatDefinition::Float64(3) => wgpu::VertexFormat::Float64x3,
                                VertexFormatDefinition::Float64(4) => wgpu::VertexFormat::Float64x4,

                                _ => panic!("vertex attribute definition should be validated when deserializing"),
                            };
                            let format_size = format.size();
                            let attr = wgpu::VertexAttribute {
                                format,
                                shader_location: i as _,
                                offset: offset_accumulator,
                            };
                            offset_accumulator += format_size;
                            attr
                        })
                        .collect(),
                }
            })
            .collect::<Vec<_>>();

        let pipeline = self.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Label::default(),

            fragment: asset.definition.fragment_shader.as_ref().map(|frag| {
                FragmentState {
                    module: &modules[&frag.shader_module],
                    entry_point: &frag.entrypoint,
                    targets: fragment_targets.as_ref().map(|vec| vec.as_slice()).unwrap_or(&[]),
                }
            }),
            vertex: VertexState {
                module: &modules[&asset.definition.vertex_shader.shader_module],
                entry_point: &asset.definition.vertex_shader.entrypoint,
                buffers: vertex_buffers.iter().map(|buffer| buffer.to_ref()).collect::<Vec<_>>().as_slice(),
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

    pub fn submit_buffer(&self, buffer: Handle<Buffer>, offset: usize, data: &[u8]) {
        let buffer = self.resources.buffers.get(buffer)
            .expect("cannot write to deleted buffer");
        self.queue.write_buffer(&buffer.buffer, offset as _, data);
    }
}

pub struct SurfaceContext {
    surface: wgpu::Surface,
    surface_config: Option<wgpu::SurfaceConfiguration>,
}

impl SurfaceContext {
    pub fn get_frame(&self) -> SurfaceFrame {
        SurfaceFrame {
            surface_texture: self.surface.get_current_texture()
                .expect("current surface texture for frame"),
        }
    }

    pub fn configure(&mut self, device: &DeviceContext, width: u32, height: u32) {
        let surface_config = self.surface.get_default_config(&device.adapter, width, height).expect("default surface configuration");
        println!("Surface capabilities: {:?}", self.surface.get_capabilities(&device.adapter));
        println!("Default surface configuration: {:?}", surface_config);
        // surface_config.format = wgpu::TextureFormat::Rgba8UnormSrgb;
        self.surface.configure(&device.device, &surface_config);
        self.surface_config = Some(surface_config);
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
    pub fn render_pass(&mut self, pass: RenderPass) {
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

        let pipeline = self.resources.pipelines.get(pass.pipeline)
            .expect("cannot draw with deleted pipeline");

        encoder_pass.set_pipeline(&pipeline.pipeline);

        for (idx, buffer) in pass.vertex_buffers.iter().enumerate() {
            if let Some(buffer) = buffer {
                let buffer = self.resources.buffers.get(*buffer)
                    .expect("cannot draw with deleted buffer");

                encoder_pass.set_vertex_buffer(idx as u32, buffer.buffer.slice(..));
            }
        }
        encoder_pass.draw(pass.vertices.clone(), 0..1);
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
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
