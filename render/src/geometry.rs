use crate::material::AttributeDefinition;

struct GeometryDefinition {
    vertex_data: Vec<u8>,
    indices: Option<Vec<u8>>,
    vertex_format: VertexFormatDefinition,
}

struct VertexFormatDefinition {
    attributes: Vec<AttributeDefinition>,
}

pub enum Indices {
    Uint16(Vec<u16>),
    Uint32(Vec<u32>),
}

impl From<Vec<u32>> for Indices {
    fn from(value: Vec<u32>) -> Self {
        Indices::Uint32(value)
    }
}

impl From<Vec<u16>> for Indices {
    fn from(value: Vec<u16>) -> Self {
        Indices::Uint16(value)
    }
}

pub struct Geometry {
    /// Raw vertex data. The application is responsible for making sure the geometry is formatted
    /// as the material expects it.
    pub(crate) data: Vec<u8>,
    pub(crate) format: GeometryFormat,
    pub(crate) indices: Vec<u16>,
}

impl Geometry {
    pub(crate) fn new(vertex_data: Vec<u8>, vertex_format: GeometryFormat, indices: Vec<u16>) -> Self {
        Geometry {
            data: vertex_data,
            format: vertex_format,
            indices,
        }
    }
}

#[derive(Clone)]
pub struct GeometryFormat(Vec<AttributeDefinition>);

impl From<Vec<AttributeDefinition>> for GeometryFormat {
    fn from(value: Vec<AttributeDefinition>) -> Self {
        GeometryFormat(value)
    }
}

impl GeometryFormat {
    pub fn empty() -> Self {
        GeometryFormat(vec![])
    }

    pub fn attributes(&self) -> &Vec<AttributeDefinition> {
        &self.0
    }

    pub fn vertex_size(&self) -> usize {
        self.attributes().iter().map(|a| a.typ.size()).sum()
    }
}
