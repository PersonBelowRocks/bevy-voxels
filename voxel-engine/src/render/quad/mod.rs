pub mod anon;
pub mod data;
pub mod error;
pub mod isometric;

use std::array;

use bevy::math::vec2;

use bevy::prelude::Vec2;
use bevy::prelude::Vec3;

use crate::data::registries::texture::TextureRegistry;
use crate::data::registries::RegistryId;
use crate::data::tile::Face;

use crate::data::texture::FaceTextureRotation;

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

#[rustfmt::skip]
pub mod consts {
    pub const ROTATION_MASK: u32 = 0b00000000_00000000_00000000_00000011;
    pub const FLIP_UV_X: u32     = 0b00000000_00000000_00000000_00000100;
    pub const FLIP_UV_Y: u32     = 0b00000000_00000000_00000000_00001000;
    pub const OCCLUSION: u32     = 0b00000000_00000000_00000000_00010000;
}

#[derive(Debug, Copy, Clone)]
pub struct QuadTextureData {
    pub texture: RegistryId<TextureRegistry>,
    pub rotation: FaceTextureRotation,
    pub flip_uv_x: bool,
    pub flip_uv_y: bool,
}

impl QuadTextureData {
    pub fn bitfield(self) -> u32 {
        let mut bits: u32 = 0;

        bits |= self.rotation.inner() as u32;

        if self.flip_uv_x {
            bits |= consts::FLIP_UV_X;
        }

        if self.flip_uv_y {
            bits |= consts::FLIP_UV_Y
        }

        bits
    }
}

#[derive(Debug, Copy, Clone)]
pub struct MeshableQuad {
    pub magnitude: f32,
    pub face: Face,
    pub quad: Quad,
    pub quad_tex: QuadTextureData,
}

impl MeshableQuad {
    #[rustfmt::skip]
    pub(crate) fn unswapped_uvs(self) -> [Vec2; 4] {
        let [hx, hy] = [self.quad.width(), self.quad.height()];
        let [lx, ly] = [0.0, 0.0];

        /*
            0---1
            |   |
            2---3
        */

        [
                vec2(lx, hy), vec2(hx, hy),
                vec2(lx, ly), vec2(hx, ly)
        ]
    }

    pub fn positions(self) -> [Vec3; 4] {
        self.quad.positions(self.face, self.magnitude)
    }

    pub fn add_to_mesh(self, idx: u32, mesh: &mut ChunkMeshAttributes) {
        let normal = self.face.normal().as_vec3();
        let positions = self.positions();

        /*
            0---1
            |   |
            2---3
        */

        let indices = match self.face {
            Face::Bottom | Face::East | Face::North => [0, 2, 1, 1, 2, 3],
            _ => [0, 1, 2, 1, 3, 2],
        }
        .map(|i| i + idx);

        let bitfields = [
            self.quad_tex.bitfield(),
            self.quad_tex.bitfield(),
            self.quad_tex.bitfield() | consts::OCCLUSION,
            self.quad_tex.bitfield(),
        ];

        mesh.indices.extend(indices);
        mesh.normals.extend([normal; 4]);
        mesh.positions.extend(positions);
        mesh.misc_data.extend(bitfields);
        mesh.textures
            .extend([self.quad_tex.texture.inner() as u32; 4]);
    }
}