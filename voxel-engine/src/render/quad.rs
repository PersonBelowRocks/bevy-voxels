use std::array;

use bevy::math::vec2;
use bevy::prelude::Mesh;
use bevy::prelude::Vec2;
use bevy::prelude::Vec3;

use crate::data::tile::Face;
use crate::util;

use super::mesh_builder::ChunkMeshAttributes;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Quad {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Quad {
    fn from_min_max(min: Vec2, max: Vec2) -> Self {
        let [width, height] = (max - min).abs().to_array();

        Self {
            x: min.x,
            y: min.y,
            width,
            height,
        }
    }

    pub fn from_points(p1: Vec2, p2: Vec2) -> Self {
        Self::from_min_max(p1.min(p2), p1.max(p2))
    }

    pub fn min(self) -> Vec2 {
        [self.x, self.y].into()
    }

    pub fn max(self) -> Vec2 {
        [self.x + self.width, self.y + self.height].into()
    }

    // TODO: use AxisMagnitude instead of a Vec3 here
    pub fn positions(self, face: Face, mag: f32) -> [Vec3; 4] {
        let non_rotated: [Vec2; 4] = {
            let min = self.min();
            let max = self.max();

            /*
            0---1
            |   |
            2---3
             */

            [
                [min.x, max.y],
                [max.x, max.y],
                [min.x, min.y],
                [max.x, min.y],
            ]
            .map(Into::into)
        };

        array::from_fn(|i| {
            let v = non_rotated[i];

            match face {
                Face::Top => [v.x, mag + 1.0, v.y],
                Face::Bottom => [v.x, mag, v.y],
                Face::North => [mag + 1.0, v.x, v.y],
                Face::East => [v.x, v.y, mag + 1.0],
                Face::South => [mag, v.x, v.y],
                Face::West => [v.x, v.y, mag],
            }
            .into()
        })
    }

    #[rustfmt::skip]
    pub fn uvs(self) -> [Vec2; 4] {
        let span = (self.max() - self.min()).abs();

        [
            [0.0, span.y],
            [span.x, span.y],
            [0.0, 0.0],
            [span.x, 0.0]
        ].map(Into::into)
    }

    pub fn width(self) -> f32 {
        self.width
    }

    pub fn height(self) -> f32 {
        self.height
    }

    pub fn heighten(mut self, amount: f32) -> Self {
        self.height += amount;
        assert!(self.height >= 0.0);
        self
    }

    pub fn widen(mut self, amount: f32) -> Self {
        self.width += amount;
        assert!(self.width >= 0.0);
        self
    }

    pub fn heighten_until<F>(self, step: f32, ceil: u32, mut f: F) -> Self
    where
        F: FnMut(u32) -> bool,
    {
        let mut n = 0;
        while !f(n) && n < ceil {
            n += 1;
        }

        self.heighten((n as f32) * step)
    }

    pub fn widen_until<F>(self, step: f32, ceil: u32, mut f: F) -> Self
    where
        F: FnMut(u32) -> bool,
    {
        let mut n: u32 = 0;
        while !f(n) && n < ceil {
            n += 1;
        }

        self.widen((n as f32) * step)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PositionedQuad {
    pub magnitude: f32,
    pub face: Face,
    pub quad: Quad,
}

impl PositionedQuad {
    pub fn positions(self) -> [Vec3; 4] {
        self.quad.positions(self.face, self.magnitude)
    }

    pub fn add_to_mesh(self, idx: u32, mesh: &mut ChunkMeshAttributes) {
        let normal = self.face.normal().as_vec3();
        let positions = self.positions();

        let uv_max = vec2(self.quad.width(), self.quad.height());

        let [hx, hy] = uv_max.to_array();
        let [lx, ly] = [0.0, 0.0];

        let raw_uvs = [vec2(lx, hy), vec2(hx, hy), vec2(lx, ly), vec2(hx, ly)];

        /*
            0---1
            |   |
            2---3
        */

        // let uvs = match self.face {
        //     Face::Top | Face::Bottom | Face::East => raw_uv.into_iter().rev(),
        //     Face::West => raw_uv.into_iter(),

        //     Face::North | Face::South => util::circular_shift(raw_uv, 2)
        // };

        let uvs = raw_uvs;

        let indices = [0, 1, 2, 1, 3, 2].map(|i| i + idx);

        mesh.indices.extend(indices);
        mesh.normals.extend([normal; 4]);
        mesh.positions.extend(positions);
        mesh.uvs.extend(uvs);
        mesh.textures.extend([vec2(0.0, 0.0); 4]);
    }
}
