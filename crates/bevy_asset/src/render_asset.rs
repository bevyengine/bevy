use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Asset;

bitflags::bitflags! {
    /// Defines where the asset will be used.
    ///
    /// If an asset is set to the `RENDER_WORLD` but not the `MAIN_WORLD`, the asset data (pixel data,
    /// mesh vertex data, etc) will be removed from the cpu-side asset once it's been extracted and prepared
    /// in the render world. The asset will remain in the assets collection, but with only metadata.
    ///
    /// Unloading the asset data saves on memory, as for most cases it is no longer necessary to keep
    /// it in RAM once it's been uploaded to the GPU's VRAM. However, this means you cannot access the
    /// asset data from the CPU (via the `Assets<T>` resource) once unloaded (without re-loading it).
    ///
    /// If you never need access to the asset from the CPU past the first frame it's loaded on,
    /// or only need very infrequent access, then set this to `RENDER_WORLD`. Otherwise, set this to
    /// `RENDER_WORLD | MAIN_WORLD`.
    ///
    /// If you have an asset that doesn't actually need to end up in the render world, like an Image
    /// that will be decoded into another Image asset, use `MAIN_WORLD` only.
    ///
    /// ## Platform-specific
    ///
    /// On Wasm, it is not possible for now to free reserved memory. To control memory usage, load assets
    /// in sequence and unload one before loading the next. See this
    /// [discussion about memory management](https://github.com/WebAssembly/design/issues/1397) for more
    /// details.
    #[repr(transparent)]
    #[derive(Serialize, Deserialize, Hash, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
    #[reflect(opaque)]
    #[reflect(Serialize, Deserialize, Hash, Clone, PartialEq, Debug)]
    pub struct RenderAssetUsages: u8 {
        /// The bit flag for the main world.
        const MAIN_WORLD = 1 << 0;
        /// The bit flag for the render world.
        const RENDER_WORLD = 1 << 1;
    }
}

impl Default for RenderAssetUsages {
    /// Returns the default render asset usage flags:
    /// `RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD`
    ///
    /// This default configuration ensures the asset persists in the main world, even after being prepared for rendering.
    ///
    /// If your asset does not change, consider using `RenderAssetUsages::RENDER_WORLD` exclusively. This will cause
    /// the asset to be unloaded from the main world once it has been prepared for rendering. If the asset does not need
    /// to reach the render world at all, use `RenderAssetUsages::MAIN_WORLD` exclusively.
    fn default() -> Self {
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
    }
}

/// Error returned when an asset due for extraction has already been extracted
#[derive(Debug, Error, Clone, Copy)]
pub enum AssetExtractionError {
    #[error("The asset has already been extracted")]
    AlreadyExtracted,
    #[error("The asset type does not support extraction. To clone the asset to the renderworld, use `RenderAssetUsages::default()`")]
    NoExtractionImplementation,
}

/// Error returned when an asset due for extraction has already been extracted
#[derive(Debug, Error, Clone, Copy)]
pub enum ExtractableAssetAccessError {
    #[error("The data has been extracted to the RenderWorld")]
    ExtractedToRenderWorld,
}

pub trait ExtractableAsset: Asset + Sized {
    type Data;

    /// Take `self` and previous gpu data, replace the data in place, then returns the asset.
    fn with_extractable_data(
        self,
        f: impl FnOnce(Self::Data) -> Self::Data,
    ) -> Result<Self, ExtractableAssetAccessError>;

    /// Access the extractable data.
    fn extractable_data_ref(&self) -> Result<&Self::Data, ExtractableAssetAccessError>;

    /// Mutably access the extractable data.
    fn extractable_data_mut(&mut self) -> Result<&mut Self::Data, ExtractableAssetAccessError>;

    /// Make a copy of the asset to be moved to the `RenderWorld` / gpu. Heavy internal data (pixels, vertex attributes)
    /// should be moved into the copy, leaving this asset with only metadata.
    /// An error may be returned to indicate that the asset has already been extracted, and should not
    /// have been modified on the CPU side (as it cannot be transferred to GPU again).
    /// The previous GPU asset is also provided, which can be used to check if the modification is valid.
    fn take_gpu_data(&mut self) -> Result<Self, AssetExtractionError>;
}
