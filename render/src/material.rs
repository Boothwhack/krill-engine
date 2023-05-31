use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::mem::size_of;
use std::ops::DerefMut;
use std::str::FromStr;

use bytemuck::{cast_slice, cast_slice_mut, from_bytes_mut};
use nalgebra::Point3;
use serde::{Deserialize, Deserializer};
use thiserror::Error;
use wgpu::VertexFormat;

use utils::Handle;

use crate::{BufferUsages, DeviceContext, Model, MutableHandle, SurfaceContext, VecBuf};
use crate::render_api::DeviceResources;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MaterialDefinition {
    pub attributes: Vec<AttributeDefinition>,
    pub uniforms: Vec<String>,
}

#[derive(Deserialize, Clone, Hash, PartialOrd, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct AttributeDefinition {
    pub typ: AttributeType,
    pub semantics: AttributeSemantics,
    pub name: Option<String>,
}

#[derive(Debug, Copy, Clone, Hash, PartialOrd, PartialEq)]
pub enum AttributeType {
    Float32(u32),
    Float64(u32),
}

impl AttributeType {
    /// Returns the size of this type in bytes.
    pub fn size(&self) -> usize {
        (match self {
            AttributeType::Float32(count) => 4 * count,
            AttributeType::Float64(count) => 8 * count,
        }) as _
    }
}

impl<'de> Deserialize<'de> for AttributeType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let str = String::deserialize(deserializer)?;
        AttributeType::from_str(&str)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Error)]
pub enum InvalidVertexFormatString {
    #[error("invalid element count")]
    InvalidCount,
    #[error("unknown format")]
    UnknownFormat,
    #[error("element count out of range")]
    OutOfRange,
}

impl FromStr for AttributeType {
    type Err = InvalidVertexFormatString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use AttributeType::*;

        let parts: Vec<_> = s.splitn(2, "x").collect();

        let count = if parts.len() == 2 {
            u32::from_str(parts[1])
                .map_err(|_| InvalidVertexFormatString::InvalidCount)?
        } else {
            1
        };
        let (variant, count_range): (fn(u32) -> AttributeType, _) = match parts[0] {
            "f32" => (Float32, 1..=4),
            "f64" => (Float64, 1..=4),
            _ => return Err(InvalidVertexFormatString::UnknownFormat),
        };

        if count_range.contains(&count) {
            Ok(variant(count))
        } else {
            Err(InvalidVertexFormatString::OutOfRange)
        }
    }
}

impl Into<VertexFormat> for AttributeType {
    fn into(self) -> VertexFormat {
        match self {
            AttributeType::Float32(1) => VertexFormat::Float32,
            AttributeType::Float32(2) => VertexFormat::Float32x2,
            AttributeType::Float32(3) => VertexFormat::Float32x3,
            AttributeType::Float32(4) => VertexFormat::Float32x4,
            AttributeType::Float64(1) => VertexFormat::Float64,
            AttributeType::Float64(2) => VertexFormat::Float64x2,
            AttributeType::Float64(3) => VertexFormat::Float64x3,
            AttributeType::Float64(4) => VertexFormat::Float64x4,

            _ => panic!("invalid input type")
        }
    }
}

#[derive(Deserialize, Clone, Hash, PartialOrd, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum AttributeSemantics {
    Position {
        /// Defines how each vertex position will be transformed when geometry is submitted as part
        /// of a batch operation.
        transform: PositionTransformation,
    },
    Color,
}

impl AttributeSemantics {
    pub fn default_name(&self) -> &'static str {
        match self {
            AttributeSemantics::Position { .. } => "position",
            AttributeSemantics::Color => "color",
        }
    }
}

#[derive(Default, Deserialize, Clone, Hash, PartialOrd, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum PositionTransformation {
    /// Vertex positions will not be transformed.
    None,
    /// Vertex positions will be transformed with the transformation matrix submitted alongside the
    /// geometry.
    #[default]
    Model,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UniformDefinition {
    pub entries: Vec<UniformEntryDefinition>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UniformEntryDefinition {
    pub visibility: UniformVisibility,
    #[serde(flatten)]
    pub typ: UniformEntryTypeDefinition,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum UniformEntryTypeDefinition {
    Buffer,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UniformVisibility {
    Vertex,
    Fragment,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UniformSemantics {
    Projection,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UniformObjectFieldFormat {
    F32x4x4
}

pub struct PipelineDefinition {
    pub shader_modules: Vec<String>,
    pub vertex_shader: Shader,
    pub fragment_shader: Shader,
    pub attribute_locations: HashMap<String, wgpu::ShaderLocation>,
}

pub struct Shader {
    pub index: usize,
    pub entrypoint: String,
}

/// Represents a vertex format and render pipeline. Contains any temporary cache resources that are
/// used when rendering [Geometry] with this material.
pub struct Material {
    pipeline: wgpu::RenderPipeline,
    bind_groups: Vec<Handle<wgpu::BindGroupLayout>>,
    cache: RefCell<MaterialCache>,
}

pub struct Counter {
    pub vertices: u16,
    pub indices: u16,
}

impl Material {
    pub fn new(device: &DeviceContext, resources: &DeviceResources, surface: &SurfaceContext, definition: MaterialDefinition, pipeline: PipelineDefinition) -> Material {
        let bind_groups = definition.uniforms.iter()
            .map(|name| resources.uniforms.get(name).expect(&format!("uniform: {}", name)).layout)
            .collect();
        let pipeline = device.create_render_pipeline(resources, surface, definition, pipeline);
        Material {
            pipeline,
            bind_groups,
            cache: RefCell::new(MaterialCache {
                vertex_buffer: device.create_buffer(0, BufferUsages::VERTEX | BufferUsages::COPY_DST),
                index_buffer: device.create_buffer(0, BufferUsages::INDEX | BufferUsages::COPY_DST),
                staging_buffer: vec![],
            }),
        }
    }

    pub fn cache_models(&self, device: &DeviceContext, resources: &DeviceResources, models: &[Model]) -> Counter {
        let mut index_counter = 0;
        let mut vertex_counter = 0;
        {
            let geometries: Vec<_> = models.into_iter()
                .map(|model| {
                    (model.transform, resources.geometries.get(model.geometry).unwrap())
                })
                .collect();

            // sum required size of vertex data and index count
            let (indices, vertex_data_size) = geometries.iter().fold((0, 0), |(indices, vertex_data_size), (_, geometry)| {
                (indices + geometry.indices.len(), vertex_data_size + geometry.vertex_data.len())
            });

            let mut cache = self.cache();
            let cache = cache.deref_mut();
            let mut vertex_buffer = MutableHandle::from_ref(device, &mut cache.vertex_buffer);
            let mut index_buffer = MutableHandle::from_ref(device, &mut cache.index_buffer);

            // reserve required capacity
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

                vertex_buffer.push(cache.staging_buffer.as_slice());
                cache.staging_buffer.clear();

                cache.staging_buffer.extend_from_slice(cast_slice(&geometry.indices));
                if !cache.staging_buffer.is_empty() {
                    for index in cast_slice_mut::<_, u16>(&mut cache.staging_buffer) {
                        *index += vertex_counter;
                    }
                }
                vertex_counter += vertex_count as u16;
                index_buffer.push(cast_slice(&cache.staging_buffer));
                cache.staging_buffer.clear();

                index_counter += geometry.indices.len();
            }
        }

        Counter {
            indices: index_counter as _,
            vertices: vertex_counter,
        }
    }

    pub(crate) fn cache(&self) -> RefMut<MaterialCache> {
        self.cache.borrow_mut()
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }
}

pub(crate) struct MaterialCache {
    pub(crate) vertex_buffer: VecBuf,
    pub(crate) index_buffer: VecBuf,
    pub(crate) staging_buffer: Vec<u8>,
}
