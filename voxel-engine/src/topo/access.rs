use bevy::prelude::*;
use std::error::Error;

use super::bounding_box::BoundingBox;
use super::chunk::Chunk;
use super::chunk_ref::ChunkVoxelOutput;

pub trait ChunkAccess: ReadAccess<ReadType = ChunkVoxelOutput> + ChunkBounds {}
impl<T> ChunkAccess for T where T: ReadAccess<ReadType = ChunkVoxelOutput> + ChunkBounds {}

pub trait ReadAccess {
    type ReadType;
    // TODO: create custom access error trait that lets a caller check if the access errored due to an out of bounds position
    type ReadErr: Error + 'static;

    fn get(&self, pos: IVec3) -> Result<Self::ReadType, Self::ReadErr>;
}

pub trait WriteAccess {
    type WriteType;
    type WriteErr: Error + 'static;

    fn set(&mut self, pos: IVec3, data: Self::WriteType) -> Result<(), Self::WriteErr>;
}

pub trait HasBounds {
    fn bounds(&self) -> BoundingBox;
}

pub trait ChunkBounds: HasBounds {}

impl<T: ChunkBounds> HasBounds for T {
    fn bounds(&self) -> BoundingBox {
        Chunk::BOUNDING_BOX
    }
}

pub trait GeneralAccess:
    ReadAccess<ReadType = Self::DataType> + WriteAccess<WriteType = Self::DataType> + HasBounds
{
    type DataType;
}

impl<V, DT> GeneralAccess for V
where
    V: WriteAccess<WriteType = DT> + ReadAccess<ReadType = DT> + HasBounds,
{
    type DataType = DT;
}
