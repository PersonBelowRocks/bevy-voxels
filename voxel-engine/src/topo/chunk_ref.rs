use std::{
    hash::BuildHasher,
    sync::{
        atomic::{AtomicBool, Ordering},
        Weak,
    },
};

use bevy::prelude::IVec3;

use crate::data::{
    tile::VoxelId,
    voxel::{BlockModel, VoxelModel},
};

use super::{
    access::{ChunkBounds, ReadAccess, WriteAccess},
    chunk::{Chunk, ChunkPos},
    error::{ChunkRefAccessError, ChunkVoxelAccessError},
    storage::containers::{
        data_storage::{SiccAccess, SiccReadAccess, SlccAccess, SlccReadAccess},
        dense::{SyncDenseContainerAccess, SyncDenseContainerReadAccess},
    },
};

#[derive(Clone)]
pub struct ChunkRef {
    pub(crate) chunk: Weak<Chunk>,
    pub(crate) changed: Weak<AtomicBool>,
    pub(crate) pos: ChunkPos,
}

impl ChunkRef {
    pub fn pos(&self) -> ChunkPos {
        self.pos
    }

    pub fn treat_as_changed(&self) -> Result<(), ChunkRefAccessError> {
        let changed = self
            .changed
            .upgrade()
            .ok_or(ChunkRefAccessError::Unloaded)?;
        changed.store(true, Ordering::SeqCst);
        Ok(())
    }

    #[allow(clippy::let_and_return)] // We need do to this little crime so the borrowchecker doesn't yell at us
    pub fn with_access<F, U>(&self, f: F) -> Result<U, ChunkRefAccessError>
    where
        F: for<'a> FnOnce(ChunkRefVxlAccess<'a, ahash::RandomState>) -> U,
    {
        let chunk = self.chunk.upgrade().ok_or(ChunkRefAccessError::Unloaded)?;
        self.treat_as_changed()?;

        let voxel_access = chunk.voxels.access();
        let model_access = chunk.models.access();

        let x = Ok(f(ChunkRefVxlAccess {
            voxels: voxel_access,
            models: model_access,
        }));
        x
    }

    #[allow(clippy::let_and_return)]
    pub fn with_read_access<F, U>(&self, f: F) -> Result<U, ChunkRefAccessError>
    where
        F: for<'a> FnOnce(ChunkRefVxlReadAccess<'a, ahash::RandomState>) -> U,
    {
        let chunk = self.chunk.upgrade().ok_or(ChunkRefAccessError::Unloaded)?;

        let voxel_access = chunk.voxels.read_access();
        let model_access = chunk.models.read_access();

        let x = Ok(f(ChunkRefVxlReadAccess {
            voxels: voxel_access,
            models: model_access,
        }));
        x
    }
}

pub struct ChunkRefVxlReadAccess<'a, S: BuildHasher> {
    voxels: SyncDenseContainerReadAccess<'a, VoxelId>,
    models: SiccReadAccess<'a, BlockModel, S>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChunkVoxelOutput {
    pub id: VoxelId,
    pub model: Option<BlockModel>,
}

#[derive(Copy, Clone, Debug)]
pub struct ChunkVoxelInput {
    pub id: VoxelId,
    pub model: Option<VoxelModel>,
}

pub struct ChunkRefVxlAccess<'a, S: BuildHasher> {
    voxels: SyncDenseContainerAccess<'a, VoxelId>,
    models: SiccAccess<'a, BlockModel, S>,
}

impl<'a, S: BuildHasher> WriteAccess for ChunkRefVxlAccess<'a, S> {
    type WriteErr = ChunkVoxelAccessError;
    type WriteType = ChunkVoxelInput;

    fn set(&mut self, pos: IVec3, data: Self::WriteType) -> Result<(), Self::WriteErr> {
        match data.model {
            None => {
                self.voxels.set(pos, data.id)?;
                self.models.set(pos, None)?;
            }
            Some(VoxelModel::Block(block_model)) => {
                self.voxels.set(pos, data.id)?;
                self.models.set(pos, Some(block_model))?;
            }
            _ => todo!(),
        }

        Ok(())
    }
}

impl<'a, S: BuildHasher> ReadAccess for ChunkRefVxlAccess<'a, S> {
    type ReadErr = ChunkVoxelAccessError;
    type ReadType = ChunkVoxelOutput;

    fn get(&self, pos: IVec3) -> Result<Self::ReadType, Self::ReadErr> {
        Ok(ChunkVoxelOutput {
            id: self.voxels.get(pos)?,
            model: self.models.get(pos)?,
        })
    }
}

impl<'a, S: BuildHasher> ChunkBounds for ChunkRefVxlAccess<'a, S> {}

impl<'a, S: BuildHasher> ReadAccess for ChunkRefVxlReadAccess<'a, S> {
    type ReadErr = ChunkVoxelAccessError;
    type ReadType = ChunkVoxelOutput;

    fn get(&self, pos: IVec3) -> Result<Self::ReadType, Self::ReadErr> {
        Ok(ChunkVoxelOutput {
            id: self.voxels.get(pos)?,
            model: self.models.get(pos)?,
        })
    }
}

impl<'a, S: BuildHasher> ChunkBounds for ChunkRefVxlReadAccess<'a, S> {}
