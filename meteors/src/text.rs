use std::iter::once;
use nalgebra::Vector2;

type Vec2 = Vector2<f32>;

pub struct Character<T> {
    pub data: T,
    pub bounds: (f32, f32),
}

impl<T> Character<T> {
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

fn intertwine(line1: impl Iterator<Item=Vec2>, line2: impl Iterator<Item=Vec2>) -> impl Iterator<Item=Vec2> {
    line1.zip(line2).flat_map(|(a, b)| once(a).chain(once(b)))
}

struct Curve {
    start: f32,
    range: f32,
    vertices: u32,

    iter: u32,
}

impl Iterator for Curve {
    type Item = Vec2;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter >= self.vertices {
            return None;
        }

        let f = self.iter as f32 / (self.vertices - 1) as f32;
        let t = self.start + self.range * f;
        self.iter += 1;
        let (sin, cos) = t.sin_cos();
        Some(Vec2::new(sin, cos))
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

fn line(from: Vec2, to: Vec2) -> impl Iterator<Item=Vec2> {
    let l = to - from;
    (0..SUBDIVISIONS).map(move |i| {
        let f = i as f32 / (SUBDIVISIONS - 1) as f32;
        from + l * f
    })
}

fn rounded(center: Vec2, radius: f32, start: f32, degrees: f32) -> impl Iterator<Item=Vec2> {
    Curve::new(start.to_radians(), degrees.to_radians(), SUBDIVISIONS)
        .map(move |p| p * radius + center)
}

pub fn character_0() -> Character<Vec<Vec2>> {
    let inner_bounds = Vec2::new(0.2, 0.8);
    let inner_radius = 0.2;

    let outer_bounds = Vec2::new(0.8, 1.0);
    let outer_radius = 0.4;

    fn rounded_box(bounds: Vec2, radius: f32) -> impl Iterator<Item=Vec2> {
        rounded(Vec2::new(bounds.x - radius, bounds.y - radius), radius, 0.0, 90.0)
            .chain(rounded(Vec2::new(bounds.x - radius, -(bounds.y - radius)), radius, 90.0, 90.0))
            .chain(rounded(Vec2::new(-(bounds.x - radius), -(bounds.y - radius)), radius, 180.0, 90.0))
            .chain(rounded(Vec2::new(-(bounds.x - radius), bounds.y - radius), radius, 270.0, 90.0))
    }

    let inner = rounded_box(inner_bounds, inner_radius);
    let outer = rounded_box(outer_bounds, outer_radius);

    let mut data: Vec<Vec2> = intertwine(inner, outer).collect();

    // close the loop
    data.push(data[0]);
    data.push(data[1]);
    Character {
        data,
        bounds: (-outer_bounds.x, outer_bounds.x),
    }
}

pub fn character_1() -> Character<Vec<Vec2>> {
    Character {
        data: vec![
            Vec2::new(0.3, -1.0),
            Vec2::new(0.3, 1.0),
            Vec2::new(-0.3, -1.0),
            Vec2::new(-0.3, 1.0),
            Vec2::new(-0.3, 0.8),
            Vec2::new(-0.3, 1.0),
            Vec2::new(-0.5, 0.8),
            Vec2::new(-0.5, 1.0),
        ],
        bounds: (-0.5, 0.3),
    }
}

const SUBDIVISIONS: u32 = 6;
const SLANT: f32 = 25.0;
const THIN_WIDTH: f32 = 0.2;
const WIDE_WIDTH: f32 = 0.6;

pub fn character_2() -> Character<Vec<Vec2>> {
    let inner_radius = THIN_WIDTH;

    let outer_bounds = Vec2::new(0.8, 1.0);
    let outer_radius = 0.4;

    let line1 = [
        Vec2::new(-outer_bounds.x, 1.0 - inner_radius),
    ].into_iter()
        .chain(rounded(Vec2::new(-inner_radius, 1.0 - inner_radius - inner_radius), inner_radius, 0.0, 90.0 + SLANT))
        .chain([
            Vec2::new(-outer_bounds.x, -outer_bounds.y),
            Vec2::new(outer_bounds.x, -outer_bounds.y),
        ]);

    let line2 = [
        Vec2::new(-outer_bounds.x, 1.0),
    ].into_iter()
        .chain(rounded(Vec2::new(outer_bounds.x - outer_radius, 1.0 - outer_radius), outer_radius, 0.0, 90.0 + SLANT))
        .chain([
            Vec2::new(-outer_bounds.x + 1.0, -outer_bounds.y + inner_radius),
            Vec2::new(outer_bounds.x, -outer_bounds.y + inner_radius),
        ]);

    let data = intertwine(line1, line2).collect();

    Character {
        data,
        bounds: (-outer_bounds.x, outer_bounds.x),
    }
}

pub fn character_3() -> Character<Vec<Vec2>> {
    let outer_radius = 0.4;
    let left = -0.4;

    let line1 = once(Vec2::new(left, 1.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, 1.0 - outer_radius), outer_radius, 0.0, 90.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, 0.4), outer_radius, 90.0, 90.0))
        .chain(once(Vec2::new(left, 0.0)))
        .chain(rounded(Vec2::new(0.8 - outer_radius, 0.2 - outer_radius), outer_radius, 0.0, 90.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, -1.0 + outer_radius), outer_radius, 90.0, 90.0))
        .chain(once(Vec2::new(left, -1.0)));
    let line2 = once(Vec2::new(left, 0.8))
        .chain(line(Vec2::new(0.2, 0.8), Vec2::new(0.2, 0.2)))
        .chain(line(Vec2::new(0.2, 0.8), Vec2::new(0.2, 0.2)))
        .chain(once(Vec2::new(left, 0.2)))
        .chain(line(Vec2::new(0.2, 0.0), Vec2::new(0.2, -0.8)))
        .chain(line(Vec2::new(0.2, 0.0), Vec2::new(0.2, -0.8)))
        .chain(once(Vec2::new(left, -0.8)));

    let data = intertwine(line1, line2).collect();
    Character {
        data,
        bounds: (left, 0.8),
    }
}

pub fn character_4() -> Character<Vec<Vec2>> {
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
    Character {
        data,
        bounds: (-0.8, 0.8),
    }
}

pub fn character_5() -> Character<Vec<Vec2>> {
    let inner_radius = 0.2;
    let outer_radius = 0.4;

    let line1 = once(Vec2::new(-0.8, -1.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, -1.0 + outer_radius), outer_radius, 180.0, -90.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, -0.7 + outer_radius), outer_radius, 90.0, -90.0))
        .chain([
            Vec2::new(-0.2, -0.7 + outer_radius * 2.0),
            Vec2::new(-0.2, 0.8),
            Vec2::new(0.8, 0.8),
        ]);
    let line2 = once(Vec2::new(-0.8, -0.8))
        .chain(rounded(Vec2::new(0.2 - inner_radius, -0.8 + inner_radius), inner_radius, 180.0, -90.0))
        .chain(rounded(Vec2::new(0.2 - inner_radius, -0.5 + inner_radius), inner_radius, 90.0, -90.0))
        .chain([
            Vec2::new(-0.8, -0.5 + inner_radius * 2.0),
            Vec2::new(-0.8, 1.0),
            Vec2::new(0.8, 1.0),
        ]);

    let data = intertwine(line1, line2).collect();
    Character {
        data,
        bounds: (-0.8, 0.8),
    }
}

pub fn character_6() -> Character<Vec<Vec2>> {
    let inner_radius = 0.2;
    let outer_radius = 0.4;

    let line1 = once(Vec2::new(0.8, 1.0))
        .chain(rounded(Vec2::new(-0.8 + outer_radius, 1.0 - outer_radius), outer_radius, 0.0, -90.0))
        .chain(rounded(Vec2::new(-0.8 + outer_radius, -1.0 + outer_radius), outer_radius, -90.0, -90.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, -1.0 + outer_radius), outer_radius, -180.0, -90.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, 0.2 - outer_radius), outer_radius, -270.0, -90.0))
        .chain(line(Vec2::new(0.8 - outer_radius, 0.2), Vec2::new(-0.2, 0.2)));
    let line2 = once(Vec2::new(0.8, 0.8))
        .chain(rounded(Vec2::new(-0.2 + inner_radius, 0.8 - inner_radius), inner_radius, 0.0, -90.0))
        .chain(rounded(Vec2::new(-0.2 + inner_radius, -0.8 + inner_radius), inner_radius, -90.0, -90.0))
        .chain(rounded(Vec2::new(0.2 - inner_radius, -0.8 + inner_radius), inner_radius, -180.0, -90.0))
        .chain(rounded(Vec2::new(0.2 - inner_radius, -inner_radius), inner_radius, -270.0, -90.0))
        .chain(rounded(Vec2::new(-0.2 + inner_radius, -inner_radius), inner_radius, 0.0, -90.0));

    let data = intertwine(line1, line2).collect();
    Character {
        data,
        bounds: (-0.8, 0.8),
    }
}

pub fn character_7() -> Character<Vec<Vec2>> {
    let data = vec![
        Vec2::new(-0.8, 1.0),
        Vec2::new(-0.8, 0.8),
        Vec2::new(0.8, 1.0),
        Vec2::new(0.0, 0.8),
        Vec2::new(0.0, -1.0),
        Vec2::new(-0.8, -1.0),
    ];
    Character {
        data,
        bounds: (-0.8, 0.8),
    }
}

pub fn character_8() -> Character<Vec<Vec2>> {
    let inner_radius = 0.2;
    let outer_radius = 0.4;

    let line1 = once(Vec2::new(-0.4, 1.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, 1.0 - outer_radius), outer_radius, 0.0, 90.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, outer_radius), outer_radius, 90.0, 90.0))
        .chain(rounded(Vec2::new(0.0, -inner_radius), inner_radius, 0.0, -90.0))
        .chain(rounded(Vec2::new(0.0, -0.8 + inner_radius), inner_radius, -90.0, -90.0))
        .chain(rounded(Vec2::new(0.0, -0.8 + inner_radius), inner_radius, -180.0, -90.0))
        .chain(rounded(Vec2::new(0.0, -inner_radius), inner_radius, -270.0, -90.0))
        .chain(rounded(Vec2::new(-0.8 + outer_radius, outer_radius), outer_radius, 180.0, 90.0))
        .chain(rounded(Vec2::new(-0.8 + outer_radius, 1.0 - outer_radius), outer_radius, 270.0, 90.0));
    let line2 = once(Vec2::new(0.0, 0.8))
        .chain(rounded(Vec2::new(0.0, 0.8 - inner_radius), inner_radius, 0.0, 90.0))
        .chain(rounded(Vec2::new(0.0, 0.2 + inner_radius), inner_radius, 90.0, 90.0))
        .chain(rounded(Vec2::new(-0.8 + outer_radius, -0.2), outer_radius, 0.0, -90.0))
        .chain(rounded(Vec2::new(-0.8 + outer_radius, -1.0 + outer_radius), outer_radius, -90.0, -90.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, -1.0 + outer_radius), outer_radius, -180.0, -90.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, -0.2), outer_radius, -270.0, -90.0))
        .chain(rounded(Vec2::new(0.0, 0.2 + inner_radius), inner_radius, 180.0, 90.0))
        .chain(rounded(Vec2::new(0.0, 0.8 - inner_radius), inner_radius, 270.0, 90.0));

    let data = intertwine(line1, line2).collect();
    Character {
        data,
        bounds: (-0.8, 0.8),
    }
}

pub fn character_9() -> Character<Vec<Vec2>> {
    let inner_radius = 0.2;
    let outer_radius = 0.4;

    let line1 = once(Vec2::new(-0.8, -1.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, -1.0 + outer_radius), outer_radius, 180.0, -90.0))
        .chain(rounded(Vec2::new(0.8 - outer_radius, 1.0 - outer_radius), outer_radius, 90.0, -90.0))
        .chain(rounded(Vec2::new(-0.8 + outer_radius, 1.0 - outer_radius), outer_radius, 0.0, -90.0))
        .chain(rounded(Vec2::new(-0.8 + outer_radius, -0.2 + outer_radius), outer_radius, -90.0, -90.0))
        .chain(line(Vec2::new(-0.8 + outer_radius, -0.2), Vec2::new(0.2, -0.2)));
    let line2 = once(Vec2::new(-0.8, -0.8))
        .chain(rounded(Vec2::new(0.0, -0.8 + inner_radius), inner_radius, 180.0, -90.0))
        .chain(rounded(Vec2::new(0.0, 0.8 - inner_radius), inner_radius, 90.0, -90.0))
        .chain(rounded(Vec2::new(0.0, 0.8 - inner_radius), inner_radius, 0.0, -90.0))
        .chain(rounded(Vec2::new(0.0, inner_radius), inner_radius, -90.0, -90.0))
        .chain(rounded(Vec2::new(0.0, inner_radius), inner_radius, -180.0, -90.0));

    let data = intertwine(line1, line2).collect();
    Character {
        data,
        bounds: (-0.8, 0.8),
    }
}
