use std::{
    any::type_name,
    io::{Read, Write},
};

use bevy::math::Vec2;

use crate::util::FaceMap;

use super::{registry::VoxelTextureRegistry, tile::Transparency};

// TODO: error handling
pub trait VoxelData: Sized {
    fn write<W: Write>(&self, buf: &mut W);
    fn read<R: Read>(buf: &mut R) -> Option<Self>;
}

pub trait Voxel: Default {
    type Stored: VoxelData;

    fn label() -> &'static str {
        type_name::<Self>()
    }

    fn from_stored(storage: Self::Stored) -> Self;

    fn store(&self) -> Self::Stored;

    fn model(&self, textures: &VoxelTextureRegistry) -> Option<VoxelModel>;

    fn properties() -> VoxelProperties;
}

#[derive(Copy, Clone, Debug)]
pub struct SimpleStorage;

impl VoxelData for SimpleStorage {
    fn write<W: Write>(&self, _buf: &mut W) {
        panic!(
            "{} is only a marker type and shouldn't be attempted to be written to a buffer!",
            type_name::<Self>()
        );
    }

    fn read<R: Read>(_buf: &mut R) -> Option<Self> {
        panic!(
            "{} is only a marker type and shouldn't be attempted to be read from a buffer!",
            type_name::<Self>()
        );
    }
}

#[derive(Clone)]
pub struct VoxelProperties {
    pub transparency: Transparency,
}

#[derive(Default, Copy, Clone, Debug)]
pub enum FaceTextureRotation {
    #[default]
    Up,
    Down,
    Left,
    Right,
}

#[derive(Copy, Clone, Debug)]
pub struct FaceTexture {
    pub rotation: FaceTextureRotation,
    pub tex_pos: Vec2,
}

impl FaceTexture {
    pub fn new(tex_pos: Vec2) -> Self {
        Self {
            tex_pos,
            rotation: Default::default(),
        }
    }

    pub fn new_rotated(tex_pos: Vec2, rotation: FaceTextureRotation) -> Self {
        Self { tex_pos, rotation }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BlockModel {
    pub textures: FaceMap<FaceTexture>,
}

impl BlockModel {
    pub fn filled(tex_pos: Vec2) -> Self {
        Self {
            textures: FaceMap::filled(FaceTexture::new(tex_pos)),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum VoxelModel {
    Block(BlockModel),
}
