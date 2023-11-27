use std::any::type_name;

use bevy::{
    asset::{AssetPath, LoadedFolder, UntypedAssetIdConversionError},
    prelude::*,
    sprite::TextureAtlasBuilderError,
};

use crate::data::{registries::texture::TextureRegistryLoader, textures::VoxelTextureFolder};

use super::registries::{texture::TextureRegistry, Registries};

#[derive(te::Error, Debug)]
pub enum TextureRegistryError {
    #[error("Could not get path for handle {0:?}")]
    CannotMakePath(UntypedHandle),
    #[error("World does not contain resource '{}'", type_name::<VoxelTextureFolder>())]
    VoxelTextureFolderNotFound,
    #[error("Voxel texture folder asset is not loaded")]
    VoxelTextureFolderNotLoaded,
    #[error("Atlas builder error: {0}")]
    AtlasBuilderError(#[from] TextureAtlasBuilderError),
    #[error("Unexpected asset ID type: {0}")]
    UnexpectedAssetIdType(#[from] UntypedAssetIdConversionError),
    #[error("{0:?} was not a square image")]
    InvalidImageDimensions(AssetPath<'static>),
    #[error("Texture does not exist: {0:?}")]
    TextureDoesntExist(AssetPath<'static>),
}

fn create_texture_registry(
    folders: Res<Assets<LoadedFolder>>,
    mut images: ResMut<Assets<Image>>,
    texture_folder: Res<VoxelTextureFolder>,
) -> Result<TextureRegistry, TextureRegistryError> {
    // rust-analyzer can't infer this type for some reason so we have to explicitly state it
    let folder: &LoadedFolder = folders
        .get(&texture_folder.0)
        .ok_or(TextureRegistryError::VoxelTextureFolderNotLoaded)?;

    let mut registry_loader = TextureRegistryLoader::new();
    for handle in folder.handles.iter() {
        let Some(path) = handle.path() else {
            return Err(TextureRegistryError::CannotMakePath(handle.clone()));
        };

        let id = handle.id().try_typed::<Image>()?;
        let texture = images
            .get(id)
            .ok_or(TextureRegistryError::TextureDoesntExist(path.clone()))?;

        if texture.height() != texture.width() {
            return Err(TextureRegistryError::InvalidImageDimensions(path.clone()));
        }

        info!("Loaded texture at path: '{}'", path);
        registry_loader.register(path.to_string(), id, texture);
    }

    Ok(registry_loader.build_registry(images.as_mut())?)
}

pub fn build_registries(world: &mut World) {
    let sysid = world.register_system(create_texture_registry);
    let result: Result<TextureRegistry, TextureRegistryError> = world.run_system(sysid).unwrap();

    world.insert_resource(Registries::new());
    let registries = world.get_resource::<Registries>().unwrap();

    let textures = match result {
        Ok(textures) => textures,
        Err(error) => {
            error!("Error when building texture registry: '{:?}'", error);
            panic!("Cannot build registries");
        }
    };

    registries.add_registry(textures);
}
