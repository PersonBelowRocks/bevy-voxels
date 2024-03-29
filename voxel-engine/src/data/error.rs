use std::path::PathBuf;

use super::{
    registries::error::TextureNotFound,
    resourcepath::error::{FromPathError, FromStrError},
    tile::Face,
    voxel::rotations::BlockModelFace,
};

#[derive(te::Error, Debug)]
#[error("TODO")]
pub struct TileDataConversionError;

#[derive(te::Error, Debug)]
pub enum TextureLoadingError {
    #[error("Error loading texture to registry. Path: '{0}'")]
    FileNotFound(PathBuf),
    #[error("Texture was either not square or not 2D")]
    InvalidTextureDimensions,
}

#[derive(te::Error, Debug)]
pub enum VariantFileLoaderError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("{0}")]
    ParseError(#[from] deser_hjson::Error),
    #[error("Requested variant was not found")]
    VariantNotFound,
    #[error("Invalid variant file name: '{0}'")]
    InvalidFileName(PathBuf),
    #[error("Error parsing file path to ResourcePath")]
    ResourcePathError(#[from] FromPathError),
}

#[derive(te::Error, Debug)]
pub enum FaceTextureDescriptorParseError {
    #[error("Couldn't parse resource path: {0}")]
    ResourcePathError(#[from] FromStrError),
    #[error("Couldn't parse texture rotation: {0}")]
    TexRotParseError(#[from] FaceTextureRotationParseError),
}

#[derive(te::Error, Debug)]
pub enum BlockModelDescriptorParseError {
    #[error(
        "Couldn't find face texture information for face '{face:?}' in direction '{direction:?}'"
    )]
    MissingFaceInDirection { direction: Face, face: Face },

    #[error("Block model didn't have texture information for face '{0:?}'")]
    MissingBlockModelFace(BlockModelFace),
}

#[derive(te::Error, Debug, dm::Constructor)]
#[error("Couldn't parse '{0}' into a face texture rotation")]
pub struct FaceTextureRotationParseError(String);

#[derive(te::Error, Debug, dm::Constructor)]
#[error("Couldn't parse '{0}' into a block model face")]
pub struct BlockModelFaceParseError(String);

#[derive(te::Error, Debug, dm::Constructor)]
#[error("Couldn't parse '{0}' into a face")]
pub struct FaceParseError(String);

#[derive(te::Error, Debug)]
pub enum RotatedTextureDescriptorParseError {
    #[error("{0}")]
    FaceTextureRotation(#[from] FaceTextureRotationParseError),
    #[error("{0}")]
    BlockModelFace(#[from] BlockModelFaceParseError),
    #[error("{0}")]
    Face(#[from] FaceParseError),
    #[error("Error parsing resource path: {0}")]
    ResourcePathError(#[from] FromStrError),
}

#[derive(te::Error, Debug)]
pub enum SubmodelFromDescriptorError {
    #[error("{0}")]
    TextureNotFound(#[from] TextureNotFound),
    #[error("Missing texture for face: {0:?}")]
    MissingFace(Face),
}

#[derive(te::Error, Debug)]
pub enum VoxelModelCreationError {
    #[error("{0}")]
    BlockModelError(#[from] SubmodelFromDescriptorError),
}

#[derive(Copy, Clone, te::Error, Debug)]
pub enum TextureAtlasesGetAssetError {
    #[error("Color texture atlas handle did not exist in world")]
    MissingColorHandle,
    #[error("Could not find color texture atlas in assets")]
    MissingColor,
    #[error("Normal texture atlas handle did not exist in world")]
    MissingNormalHandle,
    #[error("Could not find normal texture atlas in assets")]
    MissingNormal,
}
