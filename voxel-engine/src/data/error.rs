use std::path::PathBuf;

use super::resourcepath::error::FromPathError;

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

#[derive(Copy, Clone, te::Error, Debug, Default)]
#[error("Error parsing {}", stringify!(FaceTextureRotation))]
pub struct FaceTextureRotationParseError;

#[derive(Copy, Clone, te::Error, Debug, Default)]
#[error("Error parsing {}", stringify!(Face))]
pub struct FaceParseError;

#[derive(Clone, te::Error, Debug)]
#[error("Error parsing {0} as face texture descriptor")]
pub struct FaceTextureDescParseError(String);

impl FaceTextureDescParseError {
    pub fn new(s: &str) -> Self {
        Self(s.into())
    }
}

#[derive(Clone, te::Error, Debug)]
#[error("Error parsing {0} as submodel face texture descriptor")]
pub struct SubmodelFaceTextureDescParseError(String);

impl SubmodelFaceTextureDescParseError {
    pub fn new(s: &str) -> Self {
        Self(s.into())
    }
}
