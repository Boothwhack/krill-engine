use std::collections::HashMap;
use std::iter::once;
use std::mem::size_of;
use std::ops::DerefMut;

use bytemuck::{cast_slice, cast_slice_mut, from_bytes_mut};
use nalgebra::{Matrix4, Point3, vector, Vector3};
use wgpu::RenderPassDescriptor;

use utils::{CompactList, Handle};

use crate::{BufferUsages, Color, DeviceContext, Frame, Material, MaybeRef, MutableHandle, SurfaceContext, TextureFormat};
use crate::geometry::{Geometry, VertexFormat};
use crate::material::{AttributeSemantics, MaterialDefinition, PipelineDefinition, PositionTransformation, UniformDefinition};
use crate::uniform::{Uniform, UniformInstance, UniformInstanceEntry};
use crate::vecbuf::VecBuf;

#[derive(Default)]
pub struct DeviceResources {
    pub(crate) materials: CompactList<Material>,
    pub(crate) buffers: CompactList<VecBuf>,
    pub(crate) geometries: CompactList<Geometry>,
    pub(crate) bind_group_layouts: CompactList<wgpu::BindGroupLayout>,
    pub(crate) uniforms: HashMap<String, Uniform>,
}

pub struct RenderApi {
    device: DeviceContext,
    resources: DeviceResources,
    surface: SurfaceContext,
}

impl RenderApi {
    pub fn new(device: DeviceContext, surface: SurfaceContext) -> Self {
        RenderApi {
            device,
            resources: Default::default(),
            surface,
        }
    }

    pub fn surface_format(&self) -> Option<TextureFormat> {
        self.surface.format()
    }

    pub fn surface_size(&self) -> Option<(u32, u32)> {
        self.surface.size()
    }

    pub fn configure_surface(&mut self, width: u32, height: u32) {
        self.surface.configure(&self.device, width, height);
    }

    pub fn request_frame(&self) -> Frame {
        self.surface.request_frame()
    }

    pub fn present_frame(&self, frame: Frame) {
        self.surface.present_frame(frame);
    }

    pub fn new_buffer(&mut self, capacity: usize, usage: BufferUsages) -> Handle<VecBuf> {
        let buffer = self.device.create_buffer(capacity, usage);
        self.resources.buffers.add(buffer)
    }

    pub fn get_buffer<'a>(&'a mut self, handle: impl Into<MaybeRef<'a, VecBuf>>) -> Option<MutableHandle<'a, VecBuf>> {
        match handle.into() {
            MaybeRef::Handle(handle) => self.resources.buffers.get_mut(handle)
                .map(|resource| MutableHandle {
                    context: &self.device,
                    resource,
                }),
            MaybeRef::Ref(resource) => Some(MutableHandle {
                context: &self.device,
                resource,
            })
        }
    }

    pub fn new_material(&mut self, material: MaterialDefinition, pipeline: PipelineDefinition) -> Handle<Material> {
        self.resources.materials.add(Material::new(&self.device, &self.resources, &self.surface, material, pipeline))
    }

    pub fn register_uniform(&mut self, name: &str, uniform: UniformDefinition) {
        let layout = self.device.create_uniform_bind_group_layout(name, &uniform);
        let layout = self.resources.bind_group_layouts.add(layout);

        self.resources.uniforms.insert(name.to_owned(), Uniform {
            layout,
            entries: uniform.entries,
        });
    }

    pub fn instantiate_uniform(&mut self, name: &str, values: Vec<Option<UniformInstanceEntry>>) -> UniformInstance {
        let uniform = &self.resources.uniforms[name];

        UniformInstance::new(&mut self.device, &self.resources, uniform, values)
    }

    pub fn new_empty_geometry(&mut self) -> Handle<Geometry> {
        self.new_geometry(vec![], VertexFormat::empty(), vec![])
    }

    pub fn new_geometry(&mut self, data: Vec<u8>, format: VertexFormat, indices: Vec<u16>) -> Handle<Geometry> {
        self.resources.geometries.add(
            Geometry::new(
                data,
                format,
                indices,
            )
        )
    }

    pub fn get_geometry<'a>(&'a mut self, handle: impl Into<MaybeRef<'a, Geometry>>) -> Option<MutableHandle<Geometry>> {
        match handle.into() {
            MaybeRef::Handle(handle) => self.resources.geometries.get_mut(handle)
                .map(|resource| MutableHandle {
                    context: &self.device,
                    resource,
                }),
            MaybeRef::Ref(resource) => Some(MutableHandle {
                context: &self.device,
                resource,
            })
        }
    }

    pub fn new_drawer(&mut self, frame: &Frame) -> Drawer {
        let target = frame.surface_texture.texture.create_view(&Default::default());
        let encoder = self.device.device.create_command_encoder(&Default::default());

        Drawer {
            context: &self.device,
            resources: &mut self.resources,
            encoder,
            target,
        }
    }
}

pub struct Drawer<'a> {
    context: &'a DeviceContext,
    resources: &'a mut DeviceResources,
    encoder: wgpu::CommandEncoder,
    target: wgpu::TextureView,
}

impl<'a> Drawer<'a> {
    pub fn submit_batch(&mut self, batch: Batch) {
        let material = self.resources.materials.get(batch.material)
            .unwrap();

        let mut index_counter = 0;
        let mut vertex_counter = 0;
        {
            let geometries: Vec<_> = batch.models.into_iter()
                .map(|model| {
                    (model.transform, self.resources.geometries.get(model.geometry).unwrap())
                })
                .collect();

            // sum required size of vertex data and index count
            let (indices, vertex_data_size) = geometries.iter().fold((0, 0), |(indices, vertex_data_size), (_, geometry)| {
                (indices + geometry.indices.len(), vertex_data_size + geometry.vertex_data.len())
            });

            let mut cache = material.cache.borrow_mut();
            let cache = cache.deref_mut();
            let mut vertex_buffer = MutableHandle::from_ref(self.context, &mut cache.vertex_buffer);
            let mut index_buffer = MutableHandle::from_ref(self.context, &mut cache.index_buffer);

            vertex_buffer.set_capacity_at_least(vertex_data_size, false);
            index_buffer.set_capacity_at_least(indices * size_of::<u16>(), false);


            for (transform, geometry) in geometries {
                let to_reserve = geometry.vertex_data.len() as isize - cache.staging_buffer.capacity() as isize;
                if to_reserve > 0 {
                    cache.staging_buffer.reserve(to_reserve as _);
                }

                // For now the vertex data is simply copied to the staging buffer and
                // transformations are only applied to position attributes using the transform
                // matrix. This will be replaced with a proper system to convert the geometry data
                // into the vertex format the material is expecting at a later time.
                let vertex_count = {
                    cache.staging_buffer.extend_from_slice(&geometry.vertex_data);
                    let vertices = cache.staging_buffer.chunks_exact_mut(geometry.vertex_format.vertex_size());
                    let vertex_count = vertices.len();
                    for vertex in vertices {
                        let mut offset = 0;
                        for attrib in geometry.vertex_format.attributes() {
                            let size = attrib.typ.size();
                            let attrib_data = &mut vertex[offset..offset + size];

                            match attrib.semantics {
                                AttributeSemantics::Position { transform: PositionTransformation::Model } => {
                                    let position: &mut Point3<f32> = from_bytes_mut(attrib_data);
                                    *position = transform.transform_point(position);
                                }
                                _ => {}
                            }

                            offset += size;
                        }
                    }
                    vertex_count
                };

                vertex_buffer.push(cache.staging_buffer.as_slice());
                cache.staging_buffer.clear();

                cache.staging_buffer.extend_from_slice(cast_slice(&geometry.indices));
                for index in cast_slice_mut::<_, u16>(&mut cache.staging_buffer) {
                    *index += vertex_counter;
                }
                vertex_counter += vertex_count as u16;
                index_buffer.push(cast_slice(&cache.staging_buffer));
                cache.staging_buffer.clear();

                index_counter += geometry.indices.len();
            }
        }

        let material_cache = material.cache.borrow();
        let uniform_caches: Vec<_> = batch.uniforms.into_iter().map(|uniform| {
            uniform.validate_bind_group(self.context, self.resources);
            uniform.cache()
        }).collect();

        let load = match batch.clear {
            None => wgpu::LoadOp::Load,
            Some(color) => wgpu::LoadOp::Clear(color.into()),
        };
        let mut render_pass = self.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Default::default(),
            color_attachments: &[Some(
                wgpu::RenderPassColorAttachment {
                    view: &self.target,
                    ops: wgpu::Operations {
                        store: true,
                        load,
                    },
                    resolve_target: None,
                },
            )],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&material.pipeline);
        render_pass.set_vertex_buffer(0, material_cache.vertex_buffer.entire_slice());
        render_pass.set_index_buffer(material_cache.index_buffer.entire_slice(), wgpu::IndexFormat::Uint16);
        for (i, uniform) in uniform_caches.iter().enumerate() {
            render_pass.set_bind_group(i as _, uniform.bind_group(), &[]);
        }

        log::trace!(
            target:"krill-render",
            "Drawing {} ({} bytes) vertices, {} ({} bytes) indices",
            vertex_counter, material_cache.vertex_buffer.len(),
            index_counter, material_cache.index_buffer.len(),
        );

        render_pass.draw_indexed(0..index_counter as _, 0, 0..1);
    }

    pub fn finish(self) {
        let buffer = self.encoder.finish();
        self.context.queue.submit(once(buffer));
    }
}

pub struct Model {
    pub geometry: Handle<Geometry>,
    pub transform: Matrix4<f32>,
}

impl Model {
    pub fn new(geometry: Handle<Geometry>, transform: Matrix4<f32>) -> Self {
        Model { geometry, transform }
    }
}

pub struct Batch<'a> {
    material: Handle<Material>,
    uniforms: Vec<&'a UniformInstance>,
    models: Vec<Model>,
    vertex_count: usize,
    clear: Option<Color>,
}

impl<'a> Batch<'a> {
    pub fn new(material: Handle<Material>, uniforms: Vec<&'a UniformInstance>) -> Self {
        Batch {
            material,
            uniforms,
            models: vec![],
            vertex_count: 0,
            clear: None,
        }
    }

    pub fn model(&mut self, model: Model) {
        self.models.push(model);
    }

    pub fn clear(&mut self, color: Color) {
        self.clear = Some(color);
    }
}
