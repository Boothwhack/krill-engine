use std::collections::HashMap;
use std::str::FromStr;
use serde::{Deserialize, Deserializer};
use thiserror::Error;
use wgpu::VertexFormat;

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
