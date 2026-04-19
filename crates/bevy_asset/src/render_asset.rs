use core::marker::PhantomData;

use bevy_ecs::resource::Resource;
use bevy_platform::collections::HashMap;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};

use crate::{Asset, AssetId};

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

/// Represents a value that can be extracted, like [`Option`].
#[derive(Debug, Default, Clone, Copy, Reflect)]
pub enum Extractable<A> {
    #[default]
    Extracted,
    Data(A),
}

impl<A> Extractable<A> {
    pub fn as_option(self) -> Option<A> {
        match self {
            Extractable::Extracted => None,
            Extractable::Data(a) => Some(a),
        }
    }

    pub fn as_option_ref(&self) -> Option<&A> {
        match self {
            Extractable::Extracted => None,
            Extractable::Data(a) => Some(a),
        }
    }

    pub fn as_option_mut(&mut self) -> Option<&mut A> {
        match self {
            Extractable::Extracted => None,
            Extractable::Data(a) => Some(a),
        }
    }

    pub fn replace(&mut self, value: Self) -> Self {
        core::mem::replace(self, value)
    }

    pub fn take(&mut self) -> Self {
        core::mem::take(self)
    }
}

impl<A: Asset> From<A> for Extractable<A> {
    fn from(value: A) -> Self {
        Self::Data(value)
    }
}

/// Declares that this type is a retained asset of the source asset.
pub trait RetainedAsset: Send + Sync {
    type SourceAsset: Asset;
}

/// A special [`RetainedAsset`] that won't be stored in [`RetainedAssets`].
pub struct EmptyRetainedAsset<A: Asset>(PhantomData<A>);

impl<A: Asset> Default for EmptyRetainedAsset<A> {
    fn default() -> Self {
        Self(PhantomData::<A>)
    }
}

impl<A: Asset> RetainedAsset for EmptyRetainedAsset<A> {
    type SourceAsset = A;
}

/// Stores all ([`RetainedAsset`]) of extracted `RenderAsset` if they exist and are not [`EmptyRetainedAsset`].
#[derive(Resource)]
pub struct RetainedAssets<R: RetainedAsset>(HashMap<AssetId<R::SourceAsset>, R>);

impl<R: RetainedAsset> Default for RetainedAssets<R> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<R: RetainedAsset> RetainedAssets<R> {
    pub fn get(&self, id: impl Into<AssetId<R::SourceAsset>>) -> Option<&R> {
        self.0.get(&id.into())
    }

    pub fn insert(&mut self, id: impl Into<AssetId<R::SourceAsset>>, value: R) -> Option<R> {
        self.0.insert(id.into(), value)
    }

    pub fn remove(&mut self, id: impl Into<AssetId<R::SourceAsset>>) -> Option<R> {
        self.0.remove(&id.into())
    }

    pub fn iter(&self) -> impl Iterator<Item = (AssetId<R::SourceAsset>, &R)> {
        self.0.iter().map(|(k, v)| (*k, v))
    }
}
