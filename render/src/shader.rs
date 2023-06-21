use std::collections::HashMap;

use crate::geometry::GeometryFormat;
use crate::material::AttributeDefinition;

pub struct ShaderDefinition {
    pub shader_modules: Vec<String>,
    pub vertex_shader: ShaderStage,
    pub fragment_shader: ShaderStage,
    pub attribute_locations: HashMap<String, u32>,
    pub uniforms: Vec<String>,
}

pub struct ShaderStage {
    pub module: usize,
    pub entrypoint: String,
}

pub trait Shader {
    type Input;

    type Format: VertexFormat;

    fn process_vertex<'a>(&self, input: &Self::Input, vertex: <Self::Format as VertexFormat>::Vertex<'a>);

    fn shader_definition(&self) -> ShaderDefinition;
}

pub trait VertexFormat {
    type Vertex<'a>: 'a;
    type Mapper: for<'a> VertexMapper<Vertex<'a>=Self::Vertex<'a>>;

    fn mapper_for_format(format: &GeometryFormat) -> Option<Self::Mapper>;

    fn describe() -> Vec<AttributeDefinition>;
}

pub trait VertexMapper {
    type Vertex<'a>: 'a;
    type Iterator<'a>: Iterator<Item=Self::Vertex<'a>>;

    fn vertices<'a>(&self, data: &'a mut [u8], format: &GeometryFormat) -> Self::Iterator<'a>;
}
