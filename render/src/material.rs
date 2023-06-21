use std::cell::{RefCell, RefMut};
use std::ops::DerefMut;
use std::str::FromStr;
use bytemuck::cast_slice;

use serde::{Deserialize, Deserializer};
use thiserror::Error;

use utils::Handle;
use crate::render_api::DeviceResources;

use crate::{BufferUsages, DeviceContext, Model, MutableHandle, SurfaceContext, VecBuf};
use crate::shader::{Shader, VertexFormat, VertexMapper};

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

impl Into<wgpu::VertexFormat> for AttributeType {
    fn into(self) -> wgpu::VertexFormat {
        match self {
            AttributeType::Float32(1) => wgpu::VertexFormat::Float32,
            AttributeType::Float32(2) => wgpu::VertexFormat::Float32x2,
            AttributeType::Float32(3) => wgpu::VertexFormat::Float32x3,
            AttributeType::Float32(4) => wgpu::VertexFormat::Float32x4,
            AttributeType::Float64(1) => wgpu::VertexFormat::Float64,
            AttributeType::Float64(2) => wgpu::VertexFormat::Float64x2,
            AttributeType::Float64(3) => wgpu::VertexFormat::Float64x3,
            AttributeType::Float64(4) => wgpu::VertexFormat::Float64x4,

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

/// Represents a vertex format and render pipeline. Contains any temporary cache resources that are
/// used when rendering [Geometry] with this material.
pub struct Material<S: Shader> {
    shader: S,
    pipeline: wgpu::RenderPipeline,
    bind_groups: Vec<Handle<wgpu::BindGroupLayout>>,
    cache: RefCell<MaterialCache>,
}

pub struct Counter {
    pub vertices: u16,
    pub indices: u16,
}

impl<S: Shader> Material<S> {
    pub(crate) fn new(shader: S, device: &DeviceContext, resources: &DeviceResources, surface: &SurfaceContext) -> Self {
        let definition = shader.shader_definition();
        let bind_groups = definition.uniforms.iter()
            .map(|name| resources.uniforms.get(name).expect(&format!("uniform: {}", name)).layout)
            .collect();
        let pipeline = device.create_render_pipeline(resources, surface, definition, S::Format::describe());
        Material {
            pipeline,
            bind_groups,
            shader,
            cache: RefCell::new(MaterialCache::new(device)),
        }
    }

    pub fn cache_models(&self, device: &DeviceContext, resources: &DeviceResources, models: &[Model<S::Input>]) -> Counter {
        let mut index_counter = 0;
        let mut vertex_counter = 0;

        let mut cache = self.cache();
        let cache = cache.deref_mut();
        let mut vertex_buffer = MutableHandle::from_ref(device, &mut cache.vertex_buffer);
        let mut index_buffer = MutableHandle::from_ref(device, &mut cache.index_buffer);

        for model in models {
            let geometry = resources.geometries.get(model.geometry).unwrap();

            let vertex_offset = cache.vertex_staging_buffer.len();

            cache.vertex_staging_buffer.extend_from_slice(&geometry.data);
            cache.index_staging_buffer.extend_from_slice(&geometry.indices);

            // pass each vertex through the shader vertex mapper
            let vertex_count = geometry.data.len() / geometry.format.vertex_size();
            let mapper = S::Format::mapper_for_format(&geometry.format)
                .expect("shader is unable to handle geometry");
            for vertex in mapper.vertices(&mut cache.vertex_staging_buffer[vertex_offset..vertex_offset + geometry.data.len()], &geometry.format) {
                self.shader.process_vertex(&model.input, vertex);
            }

            // Update index offset
            let indices = &mut cache.index_staging_buffer[index_counter..index_counter + geometry.indices.len()];
            for index in indices.iter_mut() {
                *index += vertex_counter as u16;
            }

            vertex_counter += vertex_count;
            index_counter += geometry.indices.len();
        }

        vertex_buffer.upload(0, &cache.vertex_staging_buffer);
        index_buffer.upload(0, cast_slice(&cache.index_staging_buffer));
        cache.vertex_staging_buffer.clear();
        cache.index_staging_buffer.clear();

        Counter {
            indices: index_counter as _,
            vertices: vertex_counter as _,
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
    pub(crate) vertex_staging_buffer: Vec<u8>,
    pub(crate) index_staging_buffer: Vec<u16>,
}

impl MaterialCache {
    fn new(device: &DeviceContext) -> Self {
        MaterialCache {
            vertex_buffer: device.create_buffer(0, BufferUsages::COPY_DST | BufferUsages::VERTEX),
            index_buffer: device.create_buffer(0, BufferUsages::COPY_DST | BufferUsages::INDEX),
            vertex_staging_buffer: vec![],
            index_staging_buffer: vec![],
        }
    }
}
