use crate::{
    io::Writer, meta::Settings, transformer::TransformedAsset, Asset, AssetLoader,
    ErasedLoadedAsset, Handle, LoadedSubAsset, UntypedHandle,
};
use alloc::boxed::Box;
use atomicow::CowArc;
use bevy_platform::collections::HashMap;
use bevy_reflect::TypePath;
use bevy_tasks::{BoxedFuture, ConditionalSendFuture};
use core::{borrow::Borrow, hash::Hash, ops::Deref};
use serde::{Deserialize, Serialize};

/// Saves an [`Asset`] of a given [`AssetSaver::Asset`] type. [`AssetSaver::OutputLoader`] will then be used to load the saved asset
/// in the final deployed application. The saver should produce asset bytes in a format that [`AssetSaver::OutputLoader`] can read.
///
/// This trait is generally used in concert with [`AssetWriter`](crate::io::AssetWriter) to write assets as bytes.
///
/// For a version of this trait that can load assets, see [`AssetLoader`].
///
/// Note: This is currently only leveraged by the [`AssetProcessor`](crate::processor::AssetProcessor), and does not provide a
/// suitable interface for general purpose asset persistence. See [github issue #11216](https://github.com/bevyengine/bevy/issues/11216).
///
pub trait AssetSaver: TypePath + Send + Sync + 'static {
    /// The top level [`Asset`] saved by this [`AssetSaver`].
    type Asset: Asset;
    /// The settings type used by this [`AssetSaver`].
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// The type of [`AssetLoader`] used to load this [`Asset`]
    type OutputLoader: AssetLoader;
    /// The type of [error](`std::error::Error`) which could be encountered by this saver.
    type Error: Into<Box<dyn core::error::Error + Send + Sync + 'static>>;

    /// Saves the given runtime [`Asset`] by writing it to a byte format using `writer`. The passed in `settings` can influence how the
    /// `asset` is saved.
    fn save(
        &self,
        writer: &mut Writer,
        asset: SavedAsset<'_, Self::Asset>,
        settings: &Self::Settings,
    ) -> impl ConditionalSendFuture<
        Output = Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error>,
    >;
}

/// A type-erased dynamic variant of [`AssetSaver`] that allows callers to save assets without knowing the actual type of the [`AssetSaver`].
pub trait ErasedAssetSaver: Send + Sync + 'static {
    /// Saves the given runtime [`ErasedLoadedAsset`] by writing it to a byte format using `writer`. The passed in `settings` can influence how the
    /// `asset` is saved.
    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: &'a ErasedLoadedAsset,
        settings: &'a dyn Settings,
    ) -> BoxedFuture<'a, Result<(), Box<dyn core::error::Error + Send + Sync + 'static>>>;

    /// The type name of the [`AssetSaver`].
    fn type_name(&self) -> &'static str;
}

impl<S: AssetSaver> ErasedAssetSaver for S {
    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: &'a ErasedLoadedAsset,
        settings: &'a dyn Settings,
    ) -> BoxedFuture<'a, Result<(), Box<dyn core::error::Error + Send + Sync + 'static>>> {
        Box::pin(async move {
            let settings = settings
                .downcast_ref::<S::Settings>()
                .expect("AssetLoader settings should match the loader type");
            let saved_asset = SavedAsset::<S::Asset>::from_loaded(asset).unwrap();
            if let Err(err) = self.save(writer, saved_asset, settings).await {
                return Err(err.into());
            }
            Ok(())
        })
    }
    fn type_name(&self) -> &'static str {
        core::any::type_name::<S>()
    }
}

/// An [`Asset`] (and any "sub assets") intended to be saved.
pub struct SavedAsset<'a, A: Asset> {
    value: &'a A,
    subassets: &'a HashMap<CowArc<'static, str>, LoadedSubAsset>,
}

impl<'a, A: Asset> Deref for SavedAsset<'a, A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, A: Asset> SavedAsset<'a, A> {
    /// Creates a new [`SavedAsset`] from `asset` if its internal value matches `A`.
    pub fn from_loaded(asset: &'a ErasedLoadedAsset) -> Option<Self> {
        let value = asset.value.downcast_ref::<A>()?;
        Some(SavedAsset {
            value,
            subassets: &asset.subassets,
        })
    }

    /// Creates a new [`SavedAsset`] from the a [`TransformedAsset`]
    pub fn from_transformed(asset: &'a TransformedAsset<A>) -> Self {
        Self {
            value: &asset.value,
            subassets: &asset.subassets,
        }
    }

    /// Retrieves the value of this asset.
    #[inline]
    pub fn get(&self) -> &'a A {
        self.value
    }

    /// Returns the subasset, if it exists and matches this type.
    pub fn get_subasset<B: Asset, Q>(&self, subasset_name: &Q) -> Option<SavedAsset<'_, B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get(subasset_name)?;
        let value = subasset.asset.value.downcast_ref::<B>()?;
        Some(SavedAsset {
            value,
            subassets: &subasset.asset.subassets,
        })
    }

    /// Returns the type-erased subasset, if it exists and matches this type.
    pub fn get_erased_subasset<Q>(&self, subasset_name: &Q) -> Option<&ErasedLoadedAsset>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get(subasset_name)?;
        Some(&subasset.asset)
    }

    /// Returns the [`UntypedHandle`] of the subasset with the provided `subasset_name`, if it exists.
    pub fn get_untyped_handle<Q>(&self, subasset_name: &Q) -> Option<UntypedHandle>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get(subasset_name)?;
        Some(subasset.handle.clone())
    }

    /// Returns the [`Handle`] of the subasset with the provided `subasset_name`, if it exists and is an asset of type `B`
    pub fn get_handle<Q, B: Asset>(&self, subasset_name: &Q) -> Option<Handle<B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let subasset = self.subassets.get(subasset_name)?;
        if let Ok(handle) = subasset.handle.clone().try_typed::<B>() {
            return Some(handle);
        }
        None
    }

    /// Iterate over all subasset names in this loaded asset.
    pub fn iter_subasset_names(&self) -> impl Iterator<Item = &str> {
        self.subassets.keys().map(|s| &**s)
    }
}
