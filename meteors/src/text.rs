use std::iter::once;

use nalgebra::{vector, Vector2};

use crate::text::gen::LineBuilder;

type Vec2 = Vector2<f32>;

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

fn intertwine(line1: impl IntoIterator<Item=Vec2>, line2: impl IntoIterator<Item=Vec2>) -> impl Iterator<Item=Vec2> {
    line1.into_iter().zip(line2).flat_map(|(a, b)| once(a).chain(once(b)))
}

const INNER_RADIUS: f32 = 0.2;
const OUTER_RADIUS: f32 = 0.4;

pub enum Topology {
    Triangles,
    TriangleStrip,
}

pub fn character_space() -> Character<(Topology, Vec<Vec2>)> {
    Character { data: (Topology::Triangles, vec![]), bounds: (0.0, 0.5) }
}

pub fn character_exclamation() -> Character<(Topology, Vec<Vec2>)> {
    let data = vec![
        Vec2::new(0.0, 1.0),
        Vec2::new(0.6, 1.0),
        Vec2::new(0.0, -0.2),
        Vec2::new(0.0, -0.2),
        Vec2::new(0.6, 1.0),
        Vec2::new(0.6, -0.2),
        Vec2::new(0.0, -0.4),
        Vec2::new(0.6, -0.4),
        Vec2::new(0.0, -1.0),
        Vec2::new(0.0, -1.0),
        Vec2::new(0.6, -0.4),
        Vec2::new(0.6, -1.0),
    ];

    Character {
        data: (Topology::Triangles, data),
        bounds: (0.0, 0.6),
    }
}

pub fn character_0() -> Character<(Topology, Vec<Vec2>)> {
    let line1 = LineBuilder::new()
        .rounded(vector!(0.8-OUTER_RADIUS, 1.0-OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.8-OUTER_RADIUS, -1.0+OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.8+OUTER_RADIUS, -1.0+OUTER_RADIUS), OUTER_RADIUS, 180.0, 90.0)
        .rounded(vector!(-0.8+OUTER_RADIUS, 1.0-OUTER_RADIUS), OUTER_RADIUS, 270.0, 90.0)
        .points([vector!(0.4, 1.0)]);
    let line2 = LineBuilder::new()
        .rounded(vector!(0.2-INNER_RADIUS, 0.8-INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(vector!(0.2-INNER_RADIUS, -0.8+INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .rounded(vector!(-0.2+INNER_RADIUS, -0.8+INNER_RADIUS), INNER_RADIUS, 180.0, 90.0)
        .rounded(vector!(-0.2+INNER_RADIUS, 0.8-INNER_RADIUS), INNER_RADIUS, 270.0, 90.0)
        .points([vector!(0.2, 0.8)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_1() -> Character<(Topology, Vec<Vec2>)> {
    Character::new(
        (Topology::TriangleStrip, vec![
            Vec2::new(0.3, -1.0),
            Vec2::new(0.3, 1.0),
            Vec2::new(-0.3, -1.0),
            Vec2::new(-0.3, 1.0),
            Vec2::new(-0.3, 0.8),
            Vec2::new(-0.3, 1.0),
            Vec2::new(-0.5, 0.8),
            Vec2::new(-0.5, 1.0),
        ]),
        (-0.5, 0.3),
    )
}

pub fn character_2() -> Character<(Topology, Vec<Vec2>)> {
    let inner_radius = 0.2;
    let outer_radius = 0.4;

    let line1 = LineBuilder::new()
        .points([Vec2::new(-0.8, 1.0)])
        .rounded(Vec2::new(0.8 - outer_radius, 1.0 - outer_radius), outer_radius, 0.0, 90.0)
        .rounded(Vec2::new(0.8 - outer_radius, -0.2 + outer_radius), outer_radius, 90.0, 90.0)
        .rounded(Vec2::new(-0.2 + inner_radius, -0.2 - inner_radius), inner_radius, 0.0, -90.0)
        .points([Vec2::new(-0.2, -0.8), Vec2::new(0.8, -0.8)]);
    let line2 = LineBuilder::new()
        .points([Vec2::new(-0.8, 1.0 - inner_radius)])
        .rounded(Vec2::new(0.2 - inner_radius, 0.8 - inner_radius), inner_radius, 0.0, 90.0)
        .rounded(Vec2::new(0.2 - inner_radius, inner_radius), inner_radius, 90.0, 90.0)
        .rounded(Vec2::new(-0.8 + outer_radius, -outer_radius), outer_radius, 0.0, -90.0)
        .points([Vec2::new(-0.8, -1.0), Vec2::new(0.8, -1.0)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_3() -> Character<(Topology, Vec<Vec2>)> {
    let outer_radius = 0.4;
    let left = -0.4;

    let line1 = LineBuilder::new()
        .points([Vec2::new(left, 1.0)])
        .rounded(Vec2::new(0.8 - outer_radius, 1.0 - outer_radius), outer_radius, 0.0, 90.0)
        .rounded(Vec2::new(0.8 - outer_radius, 0.4), outer_radius, 90.0, 90.0)
        .points([Vec2::new(left, 0.0)])
        .rounded(Vec2::new(0.8 - outer_radius, 0.2 - outer_radius), outer_radius, 0.0, 90.0)
        .rounded(Vec2::new(0.8 - outer_radius, -1.0 + outer_radius), outer_radius, 90.0, 90.0)
        .points([Vec2::new(left, -1.0)]);
    let line2 = LineBuilder::new()
        .points([Vec2::new(left, 0.8)])
        .line(Vec2::new(0.2, 0.8), Vec2::new(0.2, 0.2))
        .line(Vec2::new(0.2, 0.8), Vec2::new(0.2, 0.2))
        .points([Vec2::new(left, 0.2)])
        .line(Vec2::new(0.2, 0.0), Vec2::new(0.2, -0.8))
        .line(Vec2::new(0.2, 0.0), Vec2::new(0.2, -0.8))
        .points([Vec2::new(left, -0.8)]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (left, 0.8))
}

pub fn character_4() -> Character<(Topology, Vec<Vec2>)> {
    let thickness = 0.6;
    let data = vec![
        Vec2::new(0.2, -1.0),
        Vec2::new(0.8, -1.0),
        Vec2::new(0.2, 1.0 - thickness),
        Vec2::new(0.8, 1.0),
        Vec2::new(0.2, 1.0),
        Vec2::new(-0.8 + thickness, -0.5),
        Vec2::new(-0.8, -0.5),
        Vec2::new(-0.8, -0.7),
        Vec2::new(0.8, -0.5),
        Vec2::new(0.8, -0.7),
    ];
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_5() -> Character<(Topology, Vec<Vec2>)> {
    let inner_radius = 0.2;
    let outer_radius = 0.4;

    let line1 = LineBuilder::new()
        .points([Vec2::new(-0.8, -1.0)])
        .rounded(Vec2::new(0.8 - outer_radius, -1.0 + outer_radius), outer_radius, 180.0, -90.0)
        .rounded(Vec2::new(0.8 - outer_radius, -0.7 + outer_radius), outer_radius, 90.0, -90.0)
        .points([
            Vec2::new(-0.2, -0.7 + outer_radius * 2.0),
            Vec2::new(-0.2, 0.8),
            Vec2::new(0.8, 0.8),
        ]);
    let line2 = LineBuilder::new()
        .points([Vec2::new(-0.8, -0.8)])
        .rounded(Vec2::new(0.2 - inner_radius, -0.8 + inner_radius), inner_radius, 180.0, -90.0)
        .rounded(Vec2::new(0.2 - inner_radius, -0.5 + inner_radius), inner_radius, 90.0, -90.0)
        .points([
            Vec2::new(-0.8, -0.5 + inner_radius * 2.0),
            Vec2::new(-0.8, 1.0),
            Vec2::new(0.8, 1.0),
        ]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_6() -> Character<(Topology, Vec<Vec2>)> {
    let inner_radius = 0.2;
    let outer_radius = 0.4;

    let line1 = LineBuilder::new()
        .points([Vec2::new(0.8, 1.0)])
        .rounded(Vec2::new(-0.8 + outer_radius, 1.0 - outer_radius), outer_radius, 0.0, -90.0)
        .rounded(Vec2::new(-0.8 + outer_radius, -1.0 + outer_radius), outer_radius, -90.0, -90.0)
        .rounded(Vec2::new(0.8 - outer_radius, -1.0 + outer_radius), outer_radius, -180.0, -90.0)
        .rounded(Vec2::new(0.8 - outer_radius, 0.2 - outer_radius), outer_radius, -270.0, -90.0)
        .line(Vec2::new(0.8 - outer_radius, 0.2), Vec2::new(-0.2, 0.2));
    let line2 = LineBuilder::new()
        .points([Vec2::new(0.8, 0.8)])
        .rounded(Vec2::new(-0.2 + inner_radius, 0.8 - inner_radius), inner_radius, 0.0, -90.0)
        .rounded(Vec2::new(-0.2 + inner_radius, -0.8 + inner_radius), inner_radius, -90.0, -90.0)
        .rounded(Vec2::new(0.2 - inner_radius, -0.8 + inner_radius), inner_radius, -180.0, -90.0)
        .rounded(Vec2::new(0.2 - inner_radius, -inner_radius), inner_radius, -270.0, -90.0)
        .rounded(Vec2::new(-0.2 + inner_radius, -inner_radius), inner_radius, 0.0, -90.0);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_7() -> Character<(Topology, Vec<Vec2>)> {
    let data = vec![
        Vec2::new(-0.8, 1.0),
        Vec2::new(-0.8, 0.8),
        Vec2::new(0.8, 1.0),
        Vec2::new(0.0, 0.8),
        Vec2::new(0.0, -1.0),
        Vec2::new(-0.8, -1.0),
    ];
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_8() -> Character<(Topology, Vec<Vec2>)> {
    let inner_radius = 0.2;
    let outer_radius = 0.4;

    let line1 = LineBuilder::new()
        .points([Vec2::new(-0.4, 1.0)])
        .rounded(Vec2::new(0.8 - outer_radius, 1.0 - outer_radius), outer_radius, 0.0, 90.0)
        .rounded(Vec2::new(0.8 - outer_radius, outer_radius), outer_radius, 90.0, 90.0)
        .rounded(Vec2::new(0.0, -inner_radius), inner_radius, 0.0, -90.0)
        .rounded(Vec2::new(0.0, -0.8 + inner_radius), inner_radius, -90.0, -90.0)
        .rounded(Vec2::new(0.0, -0.8 + inner_radius), inner_radius, -180.0, -90.0)
        .rounded(Vec2::new(0.0, -inner_radius), inner_radius, -270.0, -90.0)
        .rounded(Vec2::new(-0.8 + outer_radius, outer_radius), outer_radius, 180.0, 90.0)
        .rounded(Vec2::new(-0.8 + outer_radius, 1.0 - outer_radius), outer_radius, 270.0, 90.0);
    let line2 = LineBuilder::new()
        .points([Vec2::new(0.0, 0.8)])
        .rounded(Vec2::new(0.0, 0.8 - inner_radius), inner_radius, 0.0, 90.0)
        .rounded(Vec2::new(0.0, 0.2 + inner_radius), inner_radius, 90.0, 90.0)
        .rounded(Vec2::new(-0.8 + outer_radius, -0.2), outer_radius, 0.0, -90.0)
        .rounded(Vec2::new(-0.8 + outer_radius, -1.0 + outer_radius), outer_radius, -90.0, -90.0)
        .rounded(Vec2::new(0.8 - outer_radius, -1.0 + outer_radius), outer_radius, -180.0, -90.0)
        .rounded(Vec2::new(0.8 - outer_radius, -0.2), outer_radius, -270.0, -90.0)
        .rounded(Vec2::new(0.0, 0.2 + inner_radius), inner_radius, 180.0, 90.0)
        .rounded(Vec2::new(0.0, 0.8 - inner_radius), inner_radius, 270.0, 90.0);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_9() -> Character<(Topology, Vec<Vec2>)> {
    let inner_radius = 0.2;
    let outer_radius = 0.4;

    let line1 = LineBuilder::new()
        .points([Vec2::new(-0.8, -1.0)])
        .rounded(Vec2::new(0.8 - outer_radius, -1.0 + outer_radius), outer_radius, 180.0, -90.0)
        .rounded(Vec2::new(0.8 - outer_radius, 1.0 - outer_radius), outer_radius, 90.0, -90.0)
        .rounded(Vec2::new(-0.8 + outer_radius, 1.0 - outer_radius), outer_radius, 0.0, -90.0)
        .rounded(Vec2::new(-0.8 + outer_radius, -0.2 + outer_radius), outer_radius, -90.0, -90.0)
        .line(Vec2::new(-0.8 + outer_radius, -0.2), Vec2::new(0.2, -0.2));
    let line2 = LineBuilder::new()
        .points([Vec2::new(-0.8, -0.8)])
        .rounded(Vec2::new(0.0, -0.8 + inner_radius), inner_radius, 180.0, -90.0)
        .rounded(Vec2::new(0.0, 0.8 - inner_radius), inner_radius, 90.0, -90.0)
        .rounded(Vec2::new(0.0, 0.8 - inner_radius), inner_radius, 0.0, -90.0)
        .rounded(Vec2::new(0.0, inner_radius), inner_radius, -90.0, -90.0)
        .rounded(Vec2::new(0.0, inner_radius), inner_radius, -180.0, -90.0);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_a() -> Character<(Topology, Vec<Vec2>)> {
    let inner_radius = 0.2;
    let outer_radius = 0.4;

    let line1 = LineBuilder::new()
        .points([Vec2::new(-0.8, -1.0)])
        .rounded(Vec2::new(-0.8 + outer_radius, 1.0 - outer_radius), outer_radius, 270.0, 90.0)
        .rounded(Vec2::new(0.8 - outer_radius, 1.0 - outer_radius), outer_radius, 0.0, 90.0)
        .points([
            Vec2::new(0.8, 0.0),
            Vec2::new(0.8, -0.1),
            Vec2::new(0.8, -0.1),
            Vec2::new(0.8, -0.2),
            Vec2::new(0.8, -1.0),
        ]);
    let line2 = LineBuilder::new()
        .points([Vec2::new(-0.2, -1.0)])
        .rounded(Vec2::new(-0.2 + inner_radius, 0.8 - inner_radius), inner_radius, 270.0, 90.0)
        .rounded(Vec2::new(0.2 - inner_radius, 0.8 - inner_radius), inner_radius, 0.0, 90.0)
        .points([
            Vec2::new(0.2, 0.0),
            Vec2::new(-0.2, 0.0),
            Vec2::new(-0.2, -0.2),
            Vec2::new(0.2, -0.2),
            Vec2::new(0.2, -1.0),
        ]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_b() -> Character<(Topology, Vec<Vec2>)> {
    let inner_radius = 0.2;
    let outer_radius = 0.4;

    let line1 = LineBuilder::new()
        .points([Vec2::new(-0.8, 1.0)])
        .rounded(Vec2::new(0.8 - outer_radius, 1.0 - outer_radius), outer_radius, 0.0, 90.0)
        .rounded(Vec2::new(0.8 - outer_radius, outer_radius), outer_radius, 90.0, 90.0)
        .points([Vec2::new(-0.2, 0.0)])
        .rounded(Vec2::new(0.8 - outer_radius, 0.2 - outer_radius), outer_radius, 0.0, 90.0)
        .rounded(Vec2::new(0.8 - outer_radius, -1.0 + outer_radius), outer_radius, 90.0, 90.0)
        .points([
            Vec2::new(-0.8, -1.0),
            Vec2::new(-0.8, 1.0),
        ]);
    let line2 = LineBuilder::new()
        .points([Vec2::new(-0.8, 0.8)])
        .rounded(Vec2::new(0.2 - inner_radius, 0.8 - inner_radius), inner_radius, 0.0, 90.0)
        .rounded(Vec2::new(0.2 - inner_radius, 0.2 + inner_radius), inner_radius, 90.0, 90.0)
        .points([Vec2::new(-0.2, 0.2)])
        .rounded(Vec2::new(0.2 - inner_radius, -inner_radius), inner_radius, 0.0, 90.0)
        .rounded(Vec2::new(0.2 - inner_radius, -0.8 + inner_radius), inner_radius, 90.0, 90.0)
        .points([
            Vec2::new(-0.2, -0.8),
            Vec2::new(-0.2, 0.8),
        ]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_c() -> Character<(Topology, Vec<Vec2>)> {
    let line1 = LineBuilder::new()
        .rounded(Vec2::new(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .rounded(Vec2::new(-0.8 + OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 180.0, 90.0)
        .rounded(Vec2::new(-0.8 + OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 270.0, 90.0)
        .rounded(Vec2::new(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0);
    let line2 = LineBuilder::new()
        .rounded(Vec2::new(0.2 - INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .rounded(Vec2::new(-0.2 + INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 180.0, 90.0)
        .rounded(Vec2::new(-0.2 + INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 270.0, 90.0)
        .rounded(Vec2::new(0.2 - INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}

pub fn character_d() -> Character<(Topology, Vec<Vec2>)> {
    let line1 = LineBuilder::new()
        .points([Vec2::new(-0.8, 1.0)])
        .rounded(Vec2::new(0.8 - OUTER_RADIUS, 1.0 - OUTER_RADIUS), OUTER_RADIUS, 0.0, 90.0)
        .rounded(Vec2::new(0.8 - OUTER_RADIUS, -1.0 + OUTER_RADIUS), OUTER_RADIUS, 90.0, 90.0)
        .points([
            Vec2::new(-0.8, -1.0),
            Vec2::new(-0.8, 1.0),
        ]);
    let line2 = LineBuilder::new()
        .points([Vec2::new(-0.8, 0.8)])
        .rounded(Vec2::new(0.2 - INNER_RADIUS, 0.8 - INNER_RADIUS), INNER_RADIUS, 0.0, 90.0)
        .rounded(Vec2::new(0.2 - INNER_RADIUS, -0.8 + INNER_RADIUS), INNER_RADIUS, 90.0, 90.0)
        .points([
            Vec2::new(-0.2, -0.8),
            Vec2::new(-0.2, 0.8),
        ]);

    let data = intertwine(line1, line2).collect();
    Character::new((Topology::TriangleStrip, data), (-0.8, 0.8))
}
