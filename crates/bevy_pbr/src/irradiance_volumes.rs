//! Irradiance volumes.

use bevy_asset::{io::Reader, Asset, AssetLoader, LoadContext};
use bevy_math::{IVec3, Vec4};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_transform::prelude::Transform;
use bevy_utils::BoxedFuture;
use thiserror::Error;

#[cfg(feature = "serialize")]
use bincode;
#[cfg(feature = "serialize")]
use serde;
#[cfg(feature = "serialize")]
use std::io;

pub static IRRADIANCE_VOLUME_EXTENSION: &str = "voxelgi.bincode";

/// The component that defines an irradiance volume.
#[derive(Clone, Default, Reflect, Debug, TypeUuid, Asset)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[uuid = "692f12fb-b566-4c28-bf63-e0bc5ee4df87"]
pub struct IrradianceVolume {
    /// Transforms a canonical voxel cube with corners at (0, 0, 0) and (1, 1,
    /// 1) to world space.  In other words, this matrix specifies a
    /// transformation from a cube whose side length is 1 and centered at (0.5,
    /// 0.5, 0.5), representing a *single* voxel (not the entire voxel grid), to
    /// the scene's world space.
    pub transform: Transform,

    /// The size of the voxel grid, in voxels.
    pub resolution: IVec3,

    /// The voxel grid data, stored as 32-bit floating point RGBA.
    /// TODO(pcwalton): Switch to RGB9e5.
    pub data: Vec<Vec4>,
}

#[derive(Default)]
pub struct IrradianceVolumeAssetLoader;

#[derive(Error, Debug)]
pub enum IrradianceVolumeAssetLoadingError {
    #[error("`bevy_pbr` was not compiled with the `serialize` feature")]
    SerdeFeatureNotPresent,
    #[error("a serialization error occurred: {0}")]
    #[cfg(feature = "serialize")]
    Bincode(#[from] bincode::Error),
    #[error("an I/O error occurred: {0}")]
    Io(#[from] io::Error),
}

impl AssetLoader for IrradianceVolumeAssetLoader {
    type Asset = IrradianceVolume;
    type Settings = ();
    type Error = IrradianceVolumeAssetLoadingError;

    #[cfg(feature = "serialize")]
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _: &'a Self::Settings,
        _: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        use bevy_asset::AsyncReadExt;

        Box::pin(async move {
            let mut buffer = vec![];
            reader.read_to_end(&mut buffer).await?;
            Ok(bincode::deserialize(&buffer)?)
        })
    }

    #[cfg(not(feature = "serialize"))]
    fn load<'a>(
        &'a self,
        _: &'a mut Reader,
        _: &'a Self::Settings,
        _: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async { Err(IrradianceVolumeAssetLoadingError::SerdeFeatureNotPresent) })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: [&str; 1] = [IRRADIANCE_VOLUME_EXTENSION];
        &EXTENSIONS
    }
}
