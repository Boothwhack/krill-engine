use wgpu::{Adapter, Device, Queue, ShaderSource};
use crate::{BufferUsages, TextureFormat};
use crate::material::{MaterialDefinition, PipelineDefinition, UniformDefinition, UniformEntryTypeDefinition, UniformVisibility};
use crate::render_api::DeviceResources;
use crate::surface_context::SurfaceContext;
use crate::vecbuf::VecBuf;

pub struct DeviceContext {
    pub(crate) adapter: Adapter,
    pub(crate) device: Device,
    pub(crate) queue: Queue,
}

impl DeviceContext {
    pub(crate) fn new(adapter: Adapter, device: Device, queue: Queue) -> Self {
        DeviceContext {
            adapter,
            device,
            queue,
        }
    }

    pub(crate) fn create_buffer(&self, capacity: usize, usage: BufferUsages) -> VecBuf {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Default::default(),
            usage,
            mapped_at_creation: false,
            size: capacity as _,
        });

        VecBuf::new(buffer, capacity, usage)
    }

    pub(crate) fn create_uniform_bind_group_layout(&self, name: &str, uniform: &UniformDefinition) -> wgpu::BindGroupLayout {
        let entries: Vec<_> = uniform.entries.iter()
            .enumerate()
            .map(|(i, e)| {
                wgpu::BindGroupLayoutEntry {
                    binding: i as _,
                    count: None,
                    visibility: match e.visibility {
                        UniformVisibility::Vertex => wgpu::ShaderStages::VERTEX,
                        UniformVisibility::Fragment => wgpu::ShaderStages::FRAGMENT,
                    },
                    ty: match e.typ {
                        UniformEntryTypeDefinition::Buffer => wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        }
                    },
                }
            })
            .collect();
        self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("uniform:{}", name)),
            entries: entries.as_slice(),
        })
    }

    pub(crate) fn create_render_pipeline(&self,
                                         resources: &DeviceResources,
                                         surface: &SurfaceContext,
                                         material: MaterialDefinition,
                                         pipeline: PipelineDefinition) -> wgpu::RenderPipeline {
        let shader_modules: Vec<_> = pipeline.shader_modules.into_iter()
            .map(|s| self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Default::default(),
                source: ShaderSource::Wgsl(s.into()),
            }))
            .collect();

        let array_stride:usize = material.attributes.iter().map(|a| a.typ.size()).sum();
        let mut offset = 0;
        let attributes: Vec<_> = material.attributes.into_iter()
            .map(|a| {
                let shader_location = match a.name {
                    Some(name) => pipeline.attribute_locations[&name],
                    None => pipeline.attribute_locations[a.semantics.default_name()],
                };
                let attrib = wgpu::VertexAttribute {
                    format: a.typ.into(),
                    offset,
                    shader_location,
                };
                offset += a.typ.size() as wgpu::BufferAddress;
                attrib
            })
            .collect();

        let uniforms = material.uniforms.into_iter()
            .map(|u| &resources.uniforms[&u])
            .map(|u| resources.bind_group_layouts.get(u.layout))
            .collect::<Option<Vec<_>>>()
            .unwrap();
        let layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Default::default(),
            bind_group_layouts: uniforms.as_slice(),
            push_constant_ranges: &[],
        });
        self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Default::default(),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader_modules[pipeline.fragment_shader.index],
                entry_point: &pipeline.fragment_shader.entrypoint,
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: surface.format().unwrap_or(TextureFormat::Rgba8Unorm),
                        blend: None,
                        write_mask: Default::default(),
                    }),
                ],
            }),
            vertex: wgpu::VertexState {
                module: &shader_modules[pipeline.vertex_shader.index],
                entry_point: &pipeline.vertex_shader.entrypoint,
                buffers: &[
                    // Vertex buffer
                    wgpu::VertexBufferLayout {
                        attributes: attributes.as_slice(),
                        step_mode: wgpu::VertexStepMode::Vertex,
                        array_stride: array_stride as _,
                    },
                ],
            },
            layout: Some(&layout),
            multiview: None,
        })
    }
}
