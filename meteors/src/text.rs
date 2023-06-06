use std::iter::once;
use bytemuck::cast_slice;

use nalgebra::{point, vector, Vector2};
use engine::render::geometry::{Geometry, VertexFormat};
use engine::render::{Handle, RenderApi};
use crate::graphics::{DEFAULT_COLOR, generate_triangle_strip_indices, Vertex};

use crate::text::gen::LineBuilder;

pub struct Text {
    characters: [Option<Character<Handle<Geometry>>>; 59],
}

impl Text {
    pub fn new(render: &mut RenderApi, vertex_format: &VertexFormat) -> Self {
        Text {
            characters: [
                // start at ASCII char 32 (space)
                Some(character_space()),
                Some(character_exclamation()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(character_0()),
                Some(character_1()),
                Some(character_2()),
                Some(character_3()),
                Some(character_4()),
                Some(character_5()),
                Some(character_6()),
                Some(character_7()),
                Some(character_8()),
                Some(character_9()),
                Some(character_colon()),
                None,
                None,
                None,
                None,
                None,
                None,
                Some(character_a()),
                Some(character_b()),
                Some(character_c()),
                Some(character_d()),
                Some(character_e()),
                Some(character_f()),
                Some(character_g()),
                Some(character_h()),
                Some(character_i()),
                Some(character_j()),
                Some(character_k()),
                Some(character_l()),
                Some(character_m()),
                Some(character_n()),
                Some(character_o()),
                Some(character_p()),
                Some(character_q()),
                Some(character_r()),
                Some(character_s()),
                Some(character_t()),
                Some(character_u()),
                Some(character_v()),
                Some(character_w()),
                Some(character_x()),
                Some(character_y()),
                Some(character_z()),
            ].map(|character|
                character.map(|char| char.map(|(topology, vertices)| {
                    let vertices: Vec<_> = vertices.into_iter().map(|v| {
                        Vertex { position: point![v.x, v.y, 0.0], color: DEFAULT_COLOR }
                    }).collect();
                    let indices = match topology {
                        Topology::Triangles => (0..vertices.len() as u16).collect(),
                        Topology::TriangleStrip => generate_triangle_strip_indices(vertices.len()),
                    };
                    render.new_geometry(
                        cast_slice(&vertices).to_vec(),
                        vertex_format.clone(),
                        indices,
                    )
                }))
            )
        }
    }

    pub fn character(&self, character: char) -> Option<&Character<Handle<Geometry>>> {
        let char_code = (character as usize) - 32;
        self.characters.get(char_code)?.as_ref()
    }
}

mod gen {
    use std::iter::empty;

    use nalgebra::{vector, Vector2};

    const SUBDIVISIONS: u32 = 6;

    pub struct LineBuilder<I> {
        iter: I,
    }

    impl LineBuilder<std::iter::Empty<Vector2<f32>>> {
        pub fn new() -> Self {
            LineBuilder {
                iter: empty(),
            }
        }
    }

    impl<I> LineBuilder<I>
        where I: Iterator<Item=Vector2<f32>> {
        pub fn rounded(self, center: Vector2<f32>, radius: f32, start: f32, degrees: f32) -> LineBuilder<impl Iterator<Item=Vector2<f32>>> {
            LineBuilder {
                iter: self.iter.chain(
                    Curve::new(start.to_radians(), degrees.to_radians(), SUBDIVISIONS)
                        .map(move |p| p * radius + center)
                )
            }
        }

        pub fn points<const N: usize>(self, points: [Vector2<f32>; N]) -> LineBuilder<impl Iterator<Item=Vector2<f32>>> {
            LineBuilder {
                iter: self.iter.chain(points),
            }
        }

        pub fn line(self, from: Vector2<f32>, to: Vector2<f32>) -> LineBuilder<impl Iterator<Item=Vector2<f32>>> {
            let l = to - from;
            LineBuilder {
                iter: self.iter.chain((0..SUBDIVISIONS).map(move |i| {
                    let f = i as f32 / (SUBDIVISIONS - 1) as f32;
                    from + l * f
                }))
            }
        }
    }

    impl<I> IntoIterator for LineBuilder<I>
        where I: Iterator<Item=Vector2<f32>> {
        type Item = Vector2<f32>;
        type IntoIter = I;

        fn into_iter(self) -> Self::IntoIter {
            self.iter
        }
    }

    struct Curve {
        start: f32,
        range: f32,
        vertices: u32,

        iter: u32,
    }

    impl Iterator for Curve {
        type Item = Vector2<f32>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.iter >= self.vertices {
                return None;
            }

            let f = self.iter as f32 / (self.vertices - 1) as f32;
            let t = self.start + self.range * f;
            self.iter += 1;
            let (sin, cos) = t.sin_cos();
            Some(vector!(sin, cos))
        }
    }

    impl ExactSizeIterator for Curve {
        fn len(&self) -> usize {
            self.vertices as _
        }
    }

    impl Curve {
        fn new(start: f32, range: f32, vertices: u32) -> Self {
            Curve {
                start,
                range,
                vertices,
                iter: 0,
            }
        }
    }
}

pub struct Character<T> {
    pub data: T,
    pub bounds: (f32, f32),
}

impl<T> Character<T> {
    pub fn new(data: T, bounds: (f32, f32)) -> Self {
        Character { data, bounds }
    }

    pub fn map<R, F>(self, f: F) -> Character<R>
        where F: FnOnce(T) -> R {
        Character {
            data: f(self.data),
            bounds: self.bounds,
        }
    }

    pub fn size(&self) -> f32 {
        self.bounds.1 - self.bounds.0
    }
}

fn intertwine<T>(line1: impl IntoIterator<Item=T>, line2: impl IntoIterator<Item=T>) -> impl Iterator<Item=T> {
    line1.into_iter().zip(line2).flat_map(|(a, b)| once(a).chain(once(b)))
}

const INNER_RADIUS: f32 = 0.2;
const OUTER_RADIUS: f32 = 0.4;

pub enum Topology {
    Triangles,
    TriangleStrip,
}

type StandardCharacter = Character<(Topology, Vec<Vector2<f32>>)>;

pub fn character_space() -> StandardCharacter {
    Character { data: (Topology::Triangles, vec![]), bounds: (0.0, 0.5) }
}

pub fn character_exclamation() -> StandardCharacter {
    let data = vec![
        vector!(0.0, 1.0),
        vector!(0.6, 1.0),
        vector!(0.0, -0.2),
        vector!(0.0, -0.2),
        vector!(0.6, 1.0),
        vector!(0.6, -0.2),
        vector!(0.0, -0.4),
        vector!(0.6, -0.4),
        vector!(0.0, -1.0),
        vector!(0.0, -1.0),
        vector!(0.6, -0.4),
        vector!(0.6, -1.0),
    ];

    Character {
        data: (Topology::Triangles, data),
        bounds: (0.0, 0.6),
    }
}

pub fn character_0() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 180.0, 90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 270.0, 90.0)
        .points([vector!(0.4, 1.0)]);
    let line2 = LineBuilder::new()
        .rounded(vector!(0.2 - INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.2 + INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 180.0, 90.0)
        .rounded(vector!(-0.2 + INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 270.0, 90.0)
        .points([vector!(0.2, 0.8)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_1() -> StandardCharacter {
    Character::new(
        (Topology::TriangleStrip, vec![
            vector!(0.3, -1.0),
            vector!(0.3, 1.0),
            vector!(-0.3, -1.0),
            vector!(-0.3, 1.0),
            vector!(-0.3, 0.8),
            vector!(-0.3, 1.0),
            vector!(-0.5, 0.8),
            vector!(-0.5, 1.0),
        ]),
        (-0.5, 0.3),
    )
}

pub fn character_2() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(-0.8, 1.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, -0.2 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.2 + INNER_RADIUS, -0.2 - INNER_RADIUS), INNER_RADIUS, 0.0, -90.0)
        .points([vector!(-0.2, -0.8), vector!(0.8, -0.8)]);
    let line2 = LineBuilder::new()
        .points([vector!(-0.8, 1.0 - INNER_RADIUS)])
        .rounded(vector!(0.2 - INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, -OUTER_RADIUS), OUTER_RADIUS, 0.0, -90.0)
        .points([vector!(-0.8, -1.0), vector!(0.8, -1.0)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_3() -> StandardCharacter {
    let left = -0.4;

    let line1 = LineBuilder::new()
        .points([vector!(left, 1.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, 0.4), OUTER_RADIUS, 90.0, 90.0)
        .points([vector!(left, 0.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, 0.2 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .points([vector!(left, -1.0)]);
    let line2 = LineBuilder::new()
        .points([vector!(left, 0.8)])
        .line(vector!(0.2, 0.8), vector!(0.2, 0.2))
        .line(vector!(0.2, 0.8), vector!(0.2, 0.2))
        .points([vector!(left, 0.2)])
        .line(vector!(0.2, 0.0), vector!(0.2, -0.8))
        .line(vector!(0.2, 0.0), vector!(0.2, -0.8))
        .points([vector!(left, -0.8)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (left, 0.8))
}

pub fn character_4() -> StandardCharacter {
    let thickness = 0.6;
    let data = vec![
        vector!(0.2, -1.0),
        vector!(0.8, -1.0),
        vector!(0.2, 1.0 - thickness),
        vector!(0.8, 1.0),
        vector!(0.2, 1.0),
        vector!(-0.8 + thickness, -0.5),
        vector!(-0.8, -0.5),
        vector!(-0.8, -0.7),
        vector!(0.8, -0.5),
        vector!(0.8, -0.7),
    ];
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_5() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(-0.8, -1.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 180.0, -90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, -0.7 + OUTER_RADIUS), OUTER_RADIUS, 90.0, -90.0)
        .points([
            vector!(-0.2, -0.7 + OUTER_RADIUS * 2.0),
            vector!(-0.2, 0.8),
            vector!(0.8, 0.8),
        ]);
    let line2 = LineBuilder::new()
        .points([vector!(-0.8, -0.8)])
        .rounded(vector!(0.2 - INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 180.0, -90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, -0.5 + INNER_RADIUS), INNER_RADIUS, 90.0, -90.0)
        .points([
            vector!(-0.8, -0.5 + INNER_RADIUS * 2.0),
            vector!(-0.8, 1.0),
            vector!(0.8, 1.0),
        ]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_6() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(0.8, 1.0)])
        .rounded(vector!(-0.8 + OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, -90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, -90.0, -90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, -180.0, -90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, 0.2 - OUTER_RADIUS), OUTER_RADIUS, -270.0, -90.0)
        .line(vector!(0.8 - OUTER_RADIUS, 0.2), vector!(-0.2, 0.2));
    let line2 = LineBuilder::new()
        .points([vector!(0.8, 0.8)])
        .rounded(vector!(-0.2 + INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, -90.0)
        .rounded(vector!(-0.2 + INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, -90.0, -90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, -180.0, -90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, -INNER_RADIUS), INNER_RADIUS, -270.0, -90.0)
        .rounded(vector!(-0.2 + INNER_RADIUS, -INNER_RADIUS), INNER_RADIUS, 0.0, -90.0);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_7() -> StandardCharacter {
    let data = vec![
        vector!(-0.8, 1.0),
        vector!(-0.8, 0.8),
        vector!(0.8, 1.0),
        vector!(0.0, 0.8),
        vector!(0.0, -1.0),
        vector!(-0.8, -1.0),
    ];
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_8() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(-0.4, 1.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .rounded(vector!(0.0, -INNER_RADIUS), INNER_RADIUS, 0.0, -90.0)
        .rounded(vector!(0.0, -0.8 + INNER_RADIUS), INNER_RADIUS, -90.0, -90.0)
        .rounded(vector!(0.0, -0.8 + INNER_RADIUS), INNER_RADIUS, -180.0, -90.0)
        .rounded(vector!(0.0, -INNER_RADIUS), INNER_RADIUS, -270.0, -90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, OUTER_RADIUS), OUTER_RADIUS, 180.0, 90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 270.0, 90.0);
    let line2 = LineBuilder::new()
        .points([vector!(0.0, 0.8)])
        .rounded(vector!(0.0, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.0, 0.2 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, -0.2), OUTER_RADIUS, 0.0, -90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, -90.0, -90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, -180.0, -90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, -0.2), OUTER_RADIUS, -270.0, -90.0)
        .rounded(vector!(0.0, 0.2 + INNER_RADIUS), INNER_RADIUS, 180.0, 90.0)
        .rounded(vector!(0.0, 0.8 - INNER_RADIUS), INNER_RADIUS, 270.0, 90.0);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_9() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(-0.8, -1.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 180.0, -90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 90.0, -90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, -90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, -0.2 + OUTER_RADIUS), OUTER_RADIUS, -90.0, -90.0)
        .line(vector!(-0.8 + OUTER_RADIUS, -0.2), vector!(0.2, -0.2));
    let line2 = LineBuilder::new()
        .points([vector!(-0.8, -0.8)])
        .rounded(vector!(0.0, -0.8 + INNER_RADIUS), INNER_RADIUS, 180.0, -90.0)
        .rounded(vector!(0.0, 0.8 - INNER_RADIUS), INNER_RADIUS, 90.0, -90.0)
        .rounded(vector!(0.0, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, -90.0)
        .rounded(vector!(0.0, INNER_RADIUS), INNER_RADIUS, -90.0, -90.0)
        .rounded(vector!(0.0, INNER_RADIUS), INNER_RADIUS, -180.0, -90.0);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_colon() -> StandardCharacter {
    let data = vec![
        vector!(0.0, 0.6),
        vector!(0.4, 0.6),
        vector!(0.0, 0.2),
        vector!(0.4, 0.6),
        vector!(0.0, 0.2),
        vector!(0.4, 0.2),
        vector!(0.0, -0.2),
        vector!(0.4, -0.2),
        vector!(0.0, -0.6),
        vector!(0.4, -0.2),
        vector!(0.0, -0.6),
        vector!(0.4, -0.6),
    ];
    Character::new((Topology::Triangles, data), (0.0, 0.4))
}

pub fn character_a() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(-0.8, -1.0)])
        .rounded(vector!(-0.8 + OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 270.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .points([
            vector!(0.8, 0.0),
            vector!(0.8, -0.1),
            vector!(0.8, -0.1),
            vector!(0.8, -0.2),
            vector!(0.8, -1.0),
        ]);
    let line2 = LineBuilder::new()
        .points([vector!(-0.2, -1.0)])
        .rounded(vector!(-0.2 + INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 270.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .points([
            vector!(0.2, 0.0),
            vector!(-0.2, 0.0),
            vector!(-0.2, -0.2),
            vector!(0.2, -0.2),
            vector!(0.2, -1.0),
        ]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_b() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(-0.8, 1.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .points([vector!(-0.2, 0.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, 0.2 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .points([
            vector!(-0.8, -1.0),
            vector!(-0.8, 1.0),
        ]);
    let line2 = LineBuilder::new()
        .points([vector!(-0.8, 0.8)])
        .rounded(vector!(0.2 - INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, 0.2 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .points([vector!(-0.2, 0.2)])
        .rounded(vector!(0.2 - INNER_RADIUS, -INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .points([
            vector!(-0.2, -0.8),
            vector!(-0.2, 0.8),
        ]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_c() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 180.0, 90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 270.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0);
    let line2 = LineBuilder::new()
        .rounded(vector!(0.2 - INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.2 + INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 180.0, 90.0)
        .rounded(vector!(-0.2 + INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 270.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_d() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(-0.8, 1.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .points([
            vector!(-0.8, -1.0),
            vector!(-0.8, 1.0),
        ]);
    let line2 = LineBuilder::new()
        .points([vector!(-0.8, 0.8)])
        .rounded(vector!(0.2 - INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .points([
            vector!(-0.2, -0.8),
            vector!(-0.2, 0.8),
        ]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_e() -> StandardCharacter {
    let right = 1.4;

    let line1 = [
        vector!(right, 1.0),
        vector!(0.0, 1.0),
        vector!(0.0, 0.2),
        vector!(0.0, 0.1),
        vector!(0.0, 0.1),
        vector!(0.0, 0.0),
        vector!(0.0, -1.0),
        vector!(right, -1.0),
    ];
    let line2 = [
        vector!(right, 0.8),
        vector!(0.6, 0.8),
        vector!(0.6, 0.2),
        vector!(right, 0.2),
        vector!(right, 0.0),
        vector!(0.6, 0.0),
        vector!(0.6, -0.8),
        vector!(right, -0.8),
    ];

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (0.0, right))
}

pub fn character_f() -> StandardCharacter {
    let right = 1.4;

    let line1 = [
        vector!(right, 1.0),
        vector!(0.0, 1.0),
        vector!(0.0, 0.2),
        vector!(0.0, 0.1),
        vector!(0.0, 0.1),
        vector!(0.0, 0.0),
        vector!(0.0, -1.0),
    ];
    let line2 = [
        vector!(right, 0.8),
        vector!(0.6, 0.8),
        vector!(0.6, 0.2),
        vector!(right, 0.2),
        vector!(right, 0.0),
        vector!(0.6, 0.0),
        vector!(0.6, -1.0),
    ];

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (0.0, right))
}

pub fn character_g() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(0.8, 0.2)])
        .line(vector!(0.8, -0.6), vector!(0.8, -1.0))
        .rounded(vector!(-0.8+OUTER_RADIUS, -1.0+OUTER_RADIUS), OUTER_RADIUS, 180.0, 90.0)
        .rounded(vector!(-0.8+OUTER_RADIUS, 1.0-OUTER_RADIUS), OUTER_RADIUS, 270.0, 90.0)
        .points([vector!(0.8,1.0)]);
    let line2 = LineBuilder::new()
        .points([vector!(0.2,0.2)])
        .rounded(vector!(0.2-INNER_RADIUS, -0.8+INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.2+INNER_RADIUS, -0.8+INNER_RADIUS), INNER_RADIUS, 180.0, 90.0)
        .rounded(vector!(-0.2+INNER_RADIUS, 0.8-INNER_RADIUS), INNER_RADIUS, 270.0, 90.0)
        .points([vector!(0.8, 0.8)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_h() -> StandardCharacter {
    let data = vec![
        vector!(-0.8, 1.0),
        vector!(-0.8, -1.0),
        vector!(-0.2, 1.0),
        vector!(-0.2, -1.0),
        vector!(-0.2, 0.0),
        vector!(-0.2, -0.2),
        vector!(0.2, 0.0),
        vector!(0.2, -0.2),
        vector!(0.2, 1.0),
        vector!(0.2, -1.0),
        vector!(0.8, 1.0),
        vector!(0.8, -1.0),
    ];

    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_i() -> StandardCharacter {
    let data = vec![
        vector!(-0.5, 1.0),
        vector!(0.5, 1.0),
        vector!(-0.5, 0.8),
        vector!(0.5, 0.8),
        vector!(-0.3, 0.8),
        vector!(0.3, 0.8),
        vector!(-0.3, -0.8),
        vector!(0.3, -0.8),
        vector!(-0.5, -0.8),
        vector!(0.5, -0.8),
        vector!(-0.5, -1.0),
        vector!(0.5, -1.0),
    ];

    Character::new((Topology::TriangleStrip, data), (-0.5, 0.5))
}

pub fn character_j() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(0.6, 1.0)])
        .rounded(vector!(0.6-OUTER_RADIUS, -1.0+OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .points([vector!(-0.2, -1.0)]);
    let line2 = LineBuilder::new()
        .points([vector!(0.0,1.0)])
        .rounded(vector!(0.0-INNER_RADIUS, -0.8+INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .points([vector!(-0.2, -0.8)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.2, 0.6))
}

pub fn character_k() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(0.8, 1.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, -0.1 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .points([
            vector!(-0.2, -0.2),
            vector!(-0.2, -1.0),
            vector!(-0.8, -1.0),
            vector!(-0.2, -0.0),
        ])
        .rounded(vector!(0.8-OUTER_RADIUS, -0.1-OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .points([vector!(0.8, -1.0)]);
    let line2 = LineBuilder::new()
        .points([vector!(0.2, 1.0)])
        .rounded(vector!(0.2 - INNER_RADIUS, INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .points([
            vector!(-0.2, 0.0),
            vector!(-0.2, 1.0),
            vector!(-0.8, 1.0),
            vector!(-0.2, -0.2),
        ])
        .rounded(vector!(0.2 - INNER_RADIUS, -0.2 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .points([vector!(0.2, -1.0)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_l() -> StandardCharacter {
    let right = 1.4;

    let data = vec![
        vector!(0.0, 1.0),
        vector!(0.6, 1.0),
        vector!(0.0, -1.0),
        vector!(0.6, -0.8),
        vector!(right, -1.0),
        vector!(right, -0.8),
    ];
    Character::new((Topology::TriangleStrip, data), (0.0, right))
}

pub fn character_m() -> StandardCharacter {
    let line_height = 0.8;
    let line_offset = -0.2;

    let data = vec![
        vector!(-0.8, -1.0),
        vector!(-0.2, -1.0),
        vector!(-0.8, 1.0),
        vector!(-0.2, 1.0 - line_height),
        vector!(-0.2, 1.0),
        vector!(0.0, line_offset),
        vector!(0.0, line_offset + line_height),
        vector!(0.2, 1.0 - line_height),
        vector!(0.2, 1.0),
        vector!(0.8, 1.0),
        vector!(0.2, -1.0),
        vector!(0.8, -1.0),
    ];
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_n() -> StandardCharacter {
    let line_height = -0.1;

    let data = vec![
        vector!(-0.8, -1.0),
        vector!(-0.2, -1.0),
        vector!(-0.8, 1.0),
        vector!(-0.2, line_height),
        vector!(-0.2, 1.0),
        vector!(0.2, -1.0),
        vector!(0.2, -line_height),
        vector!(0.8, -1.0),
        vector!(0.2, 1.0),
        vector!(0.8, 1.0),
    ];
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_o() -> StandardCharacter {
    // reuse 0
    character_0()
}

pub fn character_p() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([
            vector!(-0.8, -1.0),
            vector!(-0.8, 1.0),
        ])
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .points([vector!(-0.2, 0.0)]);
    let line2 = LineBuilder::new()
        .points([
            vector!(-0.2, -1.0),
            vector!(-0.2, 0.8),
        ])
        .rounded(vector!(0.2 - INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, 0.2 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .points([vector!(-0.2, 0.2)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_q() -> StandardCharacter {
    let slant = 0.2;
    let line_height = 0.4;

    let line1 = LineBuilder::new()
        .rounded(vector!(-0.8 + OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 180.0, 90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 270.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .points([
            vector!(0.3, -1.0),
            vector!(0.3 + slant, -1.0 - line_height),
            vector!(-0.3 + slant, -1.0 - line_height),
            vector!(-0.3, -1.0),
            vector!(-0.8 + OUTER_RADIUS, -1.0)
        ]);
    let line2 = LineBuilder::new()
        .rounded(vector!(-0.2 + INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 180.0, 90.0)
        .rounded(vector!(-0.2 + INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 270.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .points([
            vector!(0.3 - slant * (line_height / 2.0), -0.8),
            vector!(0.3 - slant * (line_height / 2.0), -0.8),
            vector!(-0.3 - slant * (line_height / 2.0), -0.8),
            vector!(-0.3 - slant * (line_height / 2.0), -0.8),
            vector!(-0.2, -0.8),
        ]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_r() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([
            vector!(-0.8, -1.0),
            vector!(-0.8, 1.0),
        ])
        .rounded(vector!(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .points([vector!(-0.2, 0.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, 0.2 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .points([vector!(0.8, -1.0)]);
    let line2 = LineBuilder::new()
        .points([
            vector!(-0.2, -1.0),
            vector!(-0.2, 0.8),
        ])
        .rounded(vector!(0.2 - INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, 0.2 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .points([vector!(-0.2, 0.2)])
        .rounded(vector!(0.2 - INNER_RADIUS, -INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .points([vector!(0.2, -1.0)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_s() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(0.8, 1.0)])
        .rounded(vector!(-0.8 + OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, -90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, OUTER_RADIUS), OUTER_RADIUS, 270.0, -90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, -INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.2 - INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .points([vector!(-0.8, -0.8)]);
    let line2 = LineBuilder::new()
        .points([vector!(0.8, 0.8)])
        .rounded(vector!(-0.2 + INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, -90.0)
        .rounded(vector!(-0.2 + INNER_RADIUS, 0.2 + INNER_RADIUS), INNER_RADIUS, 270.0, -90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, 0.2 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .points([vector!(-0.8, -1.0)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_t() -> StandardCharacter {
    let size = 0.8;
    let data = vec![
        vector!(-0.3, -1.0),
        vector!(0.3, -1.0),
        vector!(-0.3, 0.8),
        vector!(0.3, 0.8),
        vector!(-size, 0.8),
        vector!(size, 0.8),
        vector!(-size, 1.0),
        vector!(size, 1.0),
    ];
    Character::new((Topology::TriangleStrip, data), (-size, size))
}

pub fn character_u() -> StandardCharacter {
    let line1 = LineBuilder::new()
        .points([vector!(0.8, 1.0)])
        .rounded(vector!(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.8 + OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 180.0, 90.0)
        .points([vector!(-0.8, 1.0)]);
    let line2 = LineBuilder::new()
        .points([vector!(0.2, 1.0)])
        .rounded(vector!(0.2 - INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.2 + INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 180.0, 90.0)
        .points([vector!(-0.2, 1.0)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_v() -> StandardCharacter {
    let width = 1.4;
    let weight = 0.5;

    let data = vec![
        vector!(0.0, 1.0),
        vector!(weight, 1.0),
        vector!((width - weight) / 2.0, -1.0),
        vector!((width + weight) / 2.0, -1.0),
        vector!(width - weight, 1.0),
        vector!(width, 1.0),
    ];
    Character::new((Topology::TriangleStrip, data), (0.0, width))
}

pub fn character_w() -> StandardCharacter {
    let width = 2.2;
    let weight = 0.5;

    let data = vec![
        vector!(0.0, 1.0),
        vector!(weight, 1.0),
        vector!(width / 3.0 - weight / 2.0, -1.0),
        vector!(width / 3.0 + weight / 2.0, -1.0),
        vector!((width - weight) / 2.0, 1.0),
        vector!((width + weight) / 2.0, 1.0),
        vector!(width / 1.5 - weight / 2.0, -1.0),
        vector!(width / 1.5 + weight / 2.0, -1.0),
        vector!(width - weight, 1.0),
        vector!(width, 1.0),
    ];
    Character::new((Topology::TriangleStrip, data), (0.0, width))
}

pub fn character_x() -> StandardCharacter {
    let width = 1.6;
    let weight = 0.6;

    let data = vec![
        vector!(0.0, 1.0),
        vector!(weight, 1.0),
        vector!(width - weight, -1.0),
        vector!(weight, 1.0),
        vector!(width - weight, -1.0),
        vector!(width, -1.0),
        vector!(width - weight, 1.0),
        vector!(width, 1.0),
        vector!(0.0, -1.0),
        vector!(width, 1.0),
        vector!(0.0, -1.0),
        vector!(weight, -1.0),
    ];
    Character::new((Topology::Triangles, data), (0.0, width))
}

pub fn character_y() -> StandardCharacter {
    let middle = -0.2;

    let data = vec![
        vector!(-0.8, 1.0),
        vector!(-0.2, 1.0),
        vector!(-0.3, middle),
        vector!(-0.2, 1.0),
        vector!(-0.3, middle),
        vector!(0.3, middle),
        vector!(0.2, 1.0),
        vector!(0.8, 1.0),
        vector!(-0.3, middle),
        vector!(0.8, 1.0),
        vector!(-0.3, middle),
        vector!(0.3, middle),
        vector!(-0.3, middle),
        vector!(0.3, middle),
        vector!(-0.3, -1.0),
        vector!(0.3, middle),
        vector!(-0.3, -1.0),
        vector!(0.3, -1.0),
    ];
    Character::new((Topology::Triangles, data), (-0.8, 0.8))
}

pub fn character_z() -> StandardCharacter {
    let width = 0.7;

    let line1 = [
        vector!(-0.8, 1.0),
        vector!(0.8, 1.0),
        vector!(0.8, 0.8),
        vector!(-0.8 + width, -0.8),
        vector!(-0.8 + width, -0.8),
        vector!(0.8, -0.8),
    ];
    let line2 = [
        vector!(-0.8, 0.8),
        vector!(0.8 - width, 0.8),
        vector!(0.8 - width, 0.8),
        vector!(-0.8, -0.8),
        vector!(-0.8, -1.0),
        vector!(0.8, -1.0),
    ];

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}
