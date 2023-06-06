use bytemuck::cast_slice;
use bytemuck_derive::{Pod, Zeroable};
use nalgebra::{point, Point3, RealField};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rand::distributions::Standard;

use engine::render::{Color, Handle, RenderApi};
use engine::render::geometry::{Geometry, VertexFormat};

#[derive(Default, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Point3<f32>,
    pub color: Color,
}

pub struct Graphics {
    pub ship_geometry: Handle<Geometry>,
    pub meteor_geometry: Handle<Geometry>,
    pub bullet_geometry: Handle<Geometry>,
}

impl Graphics {
    pub fn new(render: &mut RenderApi, vertex_format: &VertexFormat) -> Self {
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

        Graphics {
            ship_geometry,
            meteor_geometry,
            bullet_geometry,
        }
    }

    pub fn get_geometry(&self, shape: &Shape) -> Handle<Geometry> {
        match shape {
            Shape::Ship => self.ship_geometry,
            Shape::Meteor => self.meteor_geometry,
            Shape::Bullet => self.bullet_geometry,
        }
    }
}

pub fn generate_triangle_strip_indices(vertex_count: usize) -> Vec<u16> {
    if vertex_count > 2 {
        (0u16..(vertex_count as u16) - 2).flat_map(|i| i..i + 3).collect()
    } else {
        vec![]
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Shape {
    Ship,
    Meteor,
    Bullet,
}

pub const DEFAULT_COLOR: Color = Color::new(0.980392157, 0.921568627, 0.843137255, 1.0);

const SHIP_VERTICES: [Vertex; 4] = [
    Vertex { position: point!(-0.3, -0.3, 0.0), color: DEFAULT_COLOR },
    Vertex { position: point!(0.0, -0.2, 0.0), color: DEFAULT_COLOR },
    Vertex { position: point!(0.0, 0.3, 0.0), color: DEFAULT_COLOR },
    Vertex { position: point!(0.3, -0.3, 0.0), color: DEFAULT_COLOR },
];
const SHIP_INDICES: [u16; 6] = [
    0, 1, 2,
    1, 2, 3,
];

const BULLET_VERTICES: [Vertex; 4] = [
    Vertex { position: point!(0.04, -0.08, 0.0), color: DEFAULT_COLOR },
    Vertex { position: point!(0.04, 0.08, 0.0), color: DEFAULT_COLOR },
    Vertex { position: point!(-0.04, -0.08, 0.0), color: DEFAULT_COLOR },
    Vertex { position: point!(-0.04, 0.08, 0.0), color: DEFAULT_COLOR },
];
const BULLET_INDICES: [u16; 6] = [
    0, 1, 2,
    1, 2, 3,
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
            color: DEFAULT_COLOR,
        };
    }

    vertices.to_vec()
}
