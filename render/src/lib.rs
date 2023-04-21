pub mod bindgroup;
pub mod pipeline;

use std::collections::HashMap;
use std::fmt::Debug;
use std::iter::once;
use std::ops::{Deref, Range};
use std::rc::Rc;

use wgpu::{BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BufferBinding, BufferDescriptor, ColorTargetState, FragmentState, Label, LoadOp, Operations, PipelineLayoutDescriptor, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor, ShaderSource, TextureViewDescriptor, VertexState};

pub use wgpu::BufferUsages;
use utils::{CompactList};
pub use utils::Handle;
use crate::bindgroup::serial::{BindGroupLayoutAsset, BufferType, EntryType, Visibility};
use crate::pipeline::serial::{RenderPipelineAsset, TargetFormat, VertexFormatDefinition, VertexShaderStepMode};

pub type TextureFormat = wgpu::TextureFormat;

pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
}

pub struct Buffer {
    buffer: wgpu::Buffer,
    size: usize,
    usage: BufferUsages,
}

pub type BindGroupLayout = wgpu::BindGroupLayout;
pub type BindGroup = Rc<wgpu::BindGroup>;

#[derive(Default)]
struct Resources {
    pipelines: CompactList<Pipeline>,
    buffers: CompactList<Buffer>,
    bind_group_layouts: CompactList<BindGroupLayout>,
}

pub struct WGPUContext {
    instance: wgpu::Instance,
}

impl WGPUContext {
    // enumerate_adapters is not available in wasm environments
    #[cfg(not(target_family = "wasm"))]
    fn print_adapters(instance: &wgpu::Instance) {
        println!("Adapters:");
        for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
            println!("  {:?}", adapter.get_info());
        }
    }

    #[cfg(target_family = "wasm")]
    fn print_adapters(_: &wgpu::Instance) {}

    pub async fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();
        WGPUContext::print_adapters(&instance);

        log::info!("Got WGPU instance.");

        Some(WGPUContext { instance })
    }

    pub async fn request_device(&self, surface: &SurfaceContext) -> Result<DeviceContext, wgpu::RequestDeviceError> {
        let adapter = self.instance.request_adapter(&RequestAdapterOptions {
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
        Ok(DeviceContext { adapter, device, queue, resources: Resources::default() })
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

pub enum BindGroupBinding<'a> {
    Buffer(&'a Buffer)
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

    pub fn get_bind_group_layout(&self, bind_group_layout: Handle<BindGroupLayout>) -> Option<&BindGroupLayout> {
        self.resources.bind_group_layouts.get(bind_group_layout)
    }

    pub fn create_bind_group(&self, layout: Handle<BindGroupLayout>, entries: &[BindGroupBinding]) -> BindGroup {
        let entries: Vec<_> = entries.iter()
            .enumerate()
            .map(|(index, binding)| BindGroupEntry {
                binding: index as _,
                resource: match binding {
                    BindGroupBinding::Buffer(buffer) => buffer.buffer.as_entire_binding(),
                },
            })
            .collect();
        Rc::new(self.device.create_bind_group(&BindGroupDescriptor {
            label: Label::default(),
            layout: self.resources.bind_group_layouts.get(layout).unwrap(),
            entries: entries.as_slice(),
        }))
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

    pub fn create_bind_group_layout_from_asset(&mut self, asset: BindGroupLayoutAsset) -> Handle<BindGroupLayout> {
        let entries: Vec<_> = asset.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| BindGroupLayoutEntry {
                binding: index as _,
                visibility: match entry.visibility {
                    Visibility::Vertex => wgpu::ShaderStages::VERTEX,
                    Visibility::Fragment => wgpu::ShaderStages::FRAGMENT,
                    Visibility::VertexAndFragment => wgpu::ShaderStages::VERTEX_FRAGMENT,
                    Visibility::All => wgpu::ShaderStages::all(),
                },
                ty: match entry.ty {
                    EntryType::Buffer { ty: BufferType::Uniform } => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    }
                },
                count: None,
            })
            .collect();

        let bind_group_layout = self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Label::default(),
            entries: entries.as_slice(),
        });
        self.resources.bind_group_layouts.add(bind_group_layout)
    }

    pub fn create_pipeline_from_asset(
        &mut self,
        asset: RenderPipelineAsset,
        surface_format: Option<TextureFormat>,
        bind_group_layouts: HashMap<String, Handle<BindGroupLayout>>,
    ) -> Handle<Pipeline> {
        let modules: HashMap<_, _> = asset.shader_modules
            .into_iter()
            .map(|(name, source)| {
                let shader = self.device.create_shader_module(ShaderModuleDescriptor {
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
                        TargetFormat::BGRA8UnormSRGB => TextureFormat::Bgra8UnormSrgb,
                        TargetFormat::RGBA8UnormSRGB => TextureFormat::Rgba8UnormSrgb,
                        TargetFormat::Surface => surface_format.expect("surface format is not known"),
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
                let mut location_accumulator: u32 = 0;
                OwnedVertexBufferLayout {
                    step_mode: match buffer.step_mode {
                        VertexShaderStepMode::Vertex => wgpu::VertexStepMode::Vertex,
                        VertexShaderStepMode::Instance => wgpu::VertexStepMode::Instance,
                    },
                    array_stride: buffer.stride(),
                    attributes: buffer.attributes.iter()
                        .map(|attr| {
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
                                shader_location: location_accumulator,
                                offset: offset_accumulator,
                            };
                            location_accumulator += 1;
                            offset_accumulator += format_size;
                            attr
                        })
                        .collect(),
                }
            })
            .collect::<Vec<_>>();
        let bind_group_layouts: Vec<_> = asset.definition.bind_groups
            .iter()
            .map(|def| bind_group_layouts[&def.layout])
            .map(|handle| self.resources.bind_group_layouts.get(handle).unwrap())
            .collect();
        let layout = self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Label::default(),

            bind_group_layouts: bind_group_layouts.as_slice(),
            push_constant_ranges: &[],
        });

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

            layout: Some(&layout),
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

    pub fn format(&self) -> Option<TextureFormat> {
        self.surface_config.as_ref().map(|config| config.format)
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
        for (index, bind_group) in pass.bind_groups.iter().enumerate() {
            encoder_pass.set_bind_group(index as _, bind_group.deref(), &[]);
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
    pub bind_groups: Vec<BindGroup>,
}
