use std::collections::HashMap;
use std::mem::size_of;

use bytemuck::cast_slice;
use bytemuck_derive::{Pod, Zeroable};
use nalgebra::{Matrix4, point, Point3, RealField, Rotation3, vector};
use rand::{Rng, SeedableRng};
use rand::distributions::Standard;
use rand::rngs::StdRng;

use engine::render::{BufferUsages, Color, Handle, Model, RenderApi, VecBuf};
use engine::render::geometry::{Geometry, VertexFormat};
use engine::render::material::{AttributeDefinition, AttributeSemantics, AttributeType, Material, MaterialDefinition, PipelineDefinition, Shader, UniformDefinition, UniformEntryDefinition, UniformEntryTypeDefinition, UniformVisibility};
use engine::render::uniform::{UniformInstance, UniformInstanceEntry};

use crate::game::Transform;
use crate::text::Text;

#[derive(Default, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Point3<f32>,
    pub color: Color,
}

impl Vertex {
    pub fn new(position: Point3<f32>, color: Color) -> Self {
        Vertex { position, color }
    }
}

pub struct Graphics {
    pub material: Handle<Material>,
    pub camera_uniform: UniformInstance,
    pub camera_uniform_buffer: Handle<VecBuf>,
    pub ship_geometry: Handle<Geometry>,
    pub meteor_geometry: Handle<Geometry>,
    pub bullet_geometry: Handle<Geometry>,
    pub arrow_geometries: [Handle<Geometry>; 4],
    pub spacebar_geometry: Handle<Geometry>,
    pub text: Text,
}

impl Graphics {
    pub fn new(render: &mut RenderApi) -> Self {
        render.register_uniform("camera", UniformDefinition {
            entries: vec![UniformEntryDefinition {
                visibility: UniformVisibility::Vertex,
                typ: UniformEntryTypeDefinition::Buffer,
            }],
        });
        let camera_uniform_buffer = render.new_buffer(size_of::<Matrix4<f32>>(), BufferUsages::UNIFORM | BufferUsages::COPY_DST);
        let camera_uniform = render.instantiate_uniform("camera", vec![Some(UniformInstanceEntry::Buffer(camera_uniform_buffer.into()))]);

        let material = render.new_material(
            MaterialDefinition {
                attributes: vec![
                    AttributeDefinition {
                        name: None,
                        semantics: AttributeSemantics::Position { transform: Default::default() },
                        typ: AttributeType::Float32(3),
                    },
                    AttributeDefinition {
                        name: None,
                        semantics: AttributeSemantics::Color,
                        typ: AttributeType::Float32(4),
                    },
                ],
                uniforms: vec!["camera".to_owned()],
            },
            PipelineDefinition {
                shader_modules: vec![include_str!("assets/game.wgsl").to_owned()],
                vertex_shader: Shader { index: 0, entrypoint: "vs_main".to_owned() },
                fragment_shader: Shader { index: 0, entrypoint: "fs_main".to_owned() },
                attribute_locations: {
                    let mut attributes = HashMap::new();
                    attributes.insert("position".to_owned(), 0);
                    attributes.insert("color".to_owned(), 1);
                    attributes
                },
            },
        );

        let vertex_format = VertexFormat::from(vec![
            AttributeDefinition {
                name: Some("position".to_owned()),
                semantics: AttributeSemantics::Position { transform: Default::default() },
                typ: AttributeType::Float32(3),
            },
            AttributeDefinition {
                name: Some("color".to_owned()),
                semantics: AttributeSemantics::Color,
                typ: AttributeType::Float32(4),
            },
        ]);

        let ship_geometry = render.new_geometry(
            cast_slice(&SHIP_VERTICES).to_vec(),
            vertex_format.clone(),
            SHIP_INDICES.to_vec(),
        );
        let meteor_vertices = generate_meteor_geometry();
        let meteor_geometry = render.new_geometry(
            cast_slice(&meteor_vertices).to_vec(),
            vertex_format.clone(),
            generate_triangle_strip_indices(meteor_vertices.len()),
        );
        let bullet_geometry = render.new_geometry(
            cast_slice(&BULLET_VERTICES).to_vec(),
            vertex_format.clone(),
            BULLET_INDICES.to_vec(),
        );

        let arrow_geometries = [0.0, 1.0, 2.0, 3.0].map(|i| {
            render.new_geometry(
                cast_slice(&ARROW_VERTICES.map(|v| {
                    Vertex {
                        position: Rotation3::from_euler_angles(0.0, 0.0, (i * 90.0f32).to_radians()).transform_point(&v.position),
                        ..v
                    }
                })).to_vec(),
                vertex_format.clone(),
                generate_triangles_indices(ARROW_VERTICES.len() as _),
            )
        });

        let spacebar_geometry = render.new_geometry(
            cast_slice(&SPACEBAR_VERTICES).to_vec(),
            vertex_format.clone(),
            generate_triangles_indices(SPACEBAR_VERTICES.len() as _),
        );

        Graphics {
            material,
            camera_uniform,
            camera_uniform_buffer,
            ship_geometry,
            meteor_geometry,
            bullet_geometry,
            arrow_geometries,
            spacebar_geometry,
            text: Text::new(render, &vertex_format),
        }
    }

    pub fn draw_shape(&self, shape: &Shape, transform: &Transform, models: &mut Vec<Model>) {
        match shape {
            Shape::Ship => models.push(Model::new(self.ship_geometry, transform.to_matrix(), FOREGROUND_COLOR)),
            Shape::Meteor => models.push(Model::new(self.meteor_geometry, transform.to_matrix(), FOREGROUND_COLOR)),
            Shape::Bullet => models.push(Model::new(self.bullet_geometry, transform.to_matrix(), FOREGROUND_COLOR)),
        };
    }

    pub fn draw_text(&self, text: &str, transform: Matrix4<f32>, color: Color, models: &mut Vec<Model>) {
        let text = text
            .chars()
            .filter(|c| c.is_ascii())
            .flat_map(|c| c.to_uppercase());
        const LETTER_SPACING: f32 = 0.3;

        let mut offset = 0.0;
        for char in text {
            if let Some(character) = self.text.character(char) {
                let char_translation = Matrix4::new_translation(&vector!(
                    offset - character.bounds.0,
                    -1.0,
                    0.0
                ));

                offset += character.size() + LETTER_SPACING;
                models.push(Model::new(
                    character.data,
                    transform * char_translation,
                    color,
                ));
            }
        }
    }

    pub fn draw_arrow_keys(&self, transform: Matrix4<f32>, color: Color, models: &mut Vec<Model>) {
        models.push(Model::new(
            self.arrow_geometries[0],
            transform.prepend_translation(&vector!(0.0, 2.3, 0.0)),
            color,
        ));
        models.push(Model::new(
            self.arrow_geometries[1],
            transform.prepend_translation(&vector!(-2.3, 0.0, 0.0)),
            color,
        ));
        models.push(Model::new(
            self.arrow_geometries[2],
            transform.prepend_translation(&vector!(0.0, 0.0, 0.0)),
            color,
        ));
        models.push(Model::new(
            self.arrow_geometries[3],
            transform.prepend_translation(&vector!(2.3, 0.0, 0.0)),
            color,
        ));
    }

    pub fn draw_spacebar(&self, transform: Matrix4<f32>, color: Color, models: &mut Vec<Model>) {
        models.push(Model::new(
            self.spacebar_geometry,
            transform,
            color,
        ));
    }
}

pub fn generate_triangle_strip_indices(vertex_count: usize) -> Vec<u16> {
    if vertex_count > 2 {
        (0u16..(vertex_count as u16) - 2).flat_map(|i| i..i + 3).collect()
    } else {
        vec![]
    }
}

pub fn generate_triangles_indices(vertex_count: u16) -> Vec<u16> {
    (0..vertex_count).collect()
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Shape {
    Ship,
    Meteor,
    Bullet,
}

pub const FOREGROUND_COLOR: Color = Color::new(0.980392157, 0.921568627, 0.843137255, 1.0);
pub const BACKGROUND_COLOR: Color = Color::new(0.0, 0.011764706, 0.08627451, 1.0);

const SHIP_VERTICES: [Vertex; 4] = [
    Vertex { position: point!(-0.3, -0.3, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.0, -0.2, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.0, 0.3, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.3, -0.3, 0.0), color: Color::WHITE },
];
const SHIP_INDICES: [u16; 6] = [
    0, 1, 2,
    1, 2, 3,
];

const BULLET_VERTICES: [Vertex; 4] = [
    Vertex { position: point!(0.04, -0.08, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.04, 0.08, 0.0), color: Color::WHITE },
    Vertex { position: point!(-0.04, -0.08, 0.0), color: Color::WHITE },
    Vertex { position: point!(-0.04, 0.08, 0.0), color: Color::WHITE },
];
const BULLET_INDICES: [u16; 6] = [
    0, 1, 2,
    1, 2, 3,
];

const ARROW_HEIGHT: f32 = 0.3;
const ARROW_WIDTH: f32 = 0.3;
const ARROW_VERTICES: [Vertex; 36] = [
    Vertex { position: point!(-0.5, -ARROW_HEIGHT, 0.0), color: Color::WHITE },
    Vertex { position: point!(-0.5, -ARROW_HEIGHT + ARROW_WIDTH, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.0, ARROW_HEIGHT - ARROW_WIDTH, 0.0), color: Color::WHITE },
    Vertex { position: point!(-0.5, -ARROW_HEIGHT + ARROW_WIDTH, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.0, ARROW_HEIGHT - ARROW_WIDTH, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.0, ARROW_HEIGHT, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.0, ARROW_HEIGHT - ARROW_WIDTH, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.0, ARROW_HEIGHT, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.5, -ARROW_HEIGHT, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.0, ARROW_HEIGHT, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.5, -ARROW_HEIGHT, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.5, -ARROW_HEIGHT + ARROW_WIDTH, 0.0), color: Color::WHITE },
    Vertex { position: point!(-1.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-0.8, 0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(1.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-0.8, 0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(1.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.8, 0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(1.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.8, 0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(1.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.8, 0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(1.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(1.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(-1.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(0.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(-1.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-0.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(-1.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-0.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(-1.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-0.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(-1.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-0.8, 0.8, 0.0), color: Color::WHITE },
];
const SPACEBAR_VERTICES: [Vertex; 42] = [
    Vertex {position: point!(-0.6, 0.3, 0.0), color: Color::WHITE},
    Vertex {position: point!(-0.4, 0.3, 0.0), color: Color::WHITE},
    Vertex {position: point!(-0.6, -0.3, 0.0), color: Color::WHITE},
    Vertex {position: point!(-0.4, 0.3, 0.0), color: Color::WHITE},
    Vertex {position: point!(-0.6, -0.3, 0.0), color: Color::WHITE},
    Vertex {position: point!(-0.4, -0.1, 0.0), color: Color::WHITE},
    Vertex {position: point!(-0.6, -0.3, 0.0), color: Color::WHITE},
    Vertex {position: point!(-0.4, -0.1, 0.0), color: Color::WHITE},
    Vertex {position: point!(0.6, -0.3, 0.0), color: Color::WHITE},
    Vertex {position: point!(-0.4, -0.1, 0.0), color: Color::WHITE},
    Vertex {position: point!(0.6, -0.3, 0.0), color: Color::WHITE},
    Vertex {position: point!(0.4, -0.1, 0.0), color: Color::WHITE},
    Vertex {position: point!(0.6, -0.3, 0.0), color: Color::WHITE},
    Vertex {position: point!(0.4, -0.1, 0.0), color: Color::WHITE},
    Vertex {position: point!(0.6, 0.3, 0.0), color: Color::WHITE},
    Vertex {position: point!(0.4, -0.1, 0.0), color: Color::WHITE},
    Vertex {position: point!(0.6, 0.3, 0.0), color: Color::WHITE},
    Vertex {position: point!(0.4, 0.3, 0.0), color: Color::WHITE},

    Vertex { position: point!(-3.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-2.8, 0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(3.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-2.8, 0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(3.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(2.8, 0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(3.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(2.8, 0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(3.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(2.8, 0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(3.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(2.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(3.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(2.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(-3.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(2.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(-3.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-2.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(-3.0, -1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-2.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(-3.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-2.8, -0.8, 0.0), color: Color::WHITE },
    Vertex { position: point!(-3.0, 1.0, 0.0), color: Color::WHITE },
    Vertex { position: point!(-2.8, 0.8, 0.0), color: Color::WHITE },
];

fn generate_meteor_geometry() -> Vec<Vertex> {
    let radius = 0.5;
    let mut vertices: [Vertex; 10] = Default::default();

    let mut indices = vec![0];
    for i in 1..=vertices.len() / 2 {
        indices.push(i as u16);
        indices.push((vertices.len() - i) as u16);
    }

    let mut rng = StdRng::seed_from_u64(0).sample_iter::<f32, _>(Standard);

    let vertex_count = vertices.len();
    for (i, vertex) in vertices.iter_mut().enumerate() {
        let progress = (indices[i] as f32 / vertex_count as f32) * f32::pi() * 2.0;

        let random_x = rng.next().unwrap() * 0.09;
        let random_y = rng.next().unwrap() * 0.08;

        *vertex = Vertex {
            position: point!(
                    progress.sin() * radius + random_x,
                    progress.cos() * radius + random_y,
                    0.0
                ),
            color: Color::WHITE,
        };
    }

    vertices.to_vec()
}
