use crate::transformer::TransformedAsset;
use crate::{io::Writer, meta::Settings, Asset, ErasedLoadedAsset};
use crate::{AssetLoader, Handle, LabeledAsset, UntypedHandle};
use bevy_utils::{BoxedFuture, ConditionalSendFuture, CowArc, HashMap};
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, hash::Hash, ops::Deref};

/// Saves an [`Asset`] of a given [`AssetSaver::Asset`] type. [`AssetSaver::OutputLoader`] will then be used to load the saved asset
/// in the final deployed application. The saver should produce asset bytes in a format that [`AssetSaver::OutputLoader`] can read.
pub trait AssetSaver: Send + Sync + 'static {
    /// The top level [`Asset`] saved by this [`AssetSaver`].
    type Asset: Asset;
    /// The settings type used by this [`AssetSaver`].
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    /// The type of [`AssetLoader`] used to load this [`Asset`]
    type OutputLoader: AssetLoader;
    /// The type of [error](`std::error::Error`) which could be encountered by this saver.
    type Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>;

    /// Saves the given runtime [`Asset`] by writing it to a byte format using `writer`. The passed in `settings` can influence how the
    /// `asset` is saved.  
    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: SavedAsset<'a, Self::Asset>,
        settings: &'a Self::Settings,
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
    ) -> BoxedFuture<'a, Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>;

    /// The type name of the [`AssetSaver`].
    fn type_name(&self) -> &'static str;
}

impl<S: AssetSaver> ErasedAssetSaver for S {
    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: &'a ErasedLoadedAsset,
        settings: &'a dyn Settings,
    ) -> BoxedFuture<'a, Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>> {
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
        std::any::type_name::<S>()
    }
}

/// An [`Asset`] (and any labeled "sub assets") intended to be saved.
pub struct SavedAsset<'a, A: Asset> {
    value: &'a A,
    labeled_assets: &'a HashMap<CowArc<'static, str>, LabeledAsset>,
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
            labeled_assets: &asset.labeled_assets,
        })
    }

    /// Creates a new [`SavedAsset`] from the a [`TransformedAsset`]
    pub fn from_transformed(asset: &'a TransformedAsset<A>) -> Self {
        Self {
            value: &asset.value,
            labeled_assets: &asset.labeled_assets,
        }
    }

    /// Retrieves the value of this asset.
    #[inline]
    pub fn get(&self) -> &'a A {
        self.value
    }

    /// Returns the labeled asset, if it exists and matches this type.
    pub fn get_labeled<B: Asset, Q>(&self, label: &Q) -> Option<SavedAsset<B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get(label)?;
        let value = labeled.asset.value.downcast_ref::<B>()?;
        Some(SavedAsset {
            value,
            labeled_assets: &labeled.asset.labeled_assets,
        })
    }

    /// Returns the type-erased labeled asset, if it exists and matches this type.
    pub fn get_erased_labeled<Q>(&self, label: &Q) -> Option<&ErasedLoadedAsset>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get(label)?;
        Some(&labeled.asset)
    }

    /// Returns the [`UntypedHandle`] of the labeled asset with the provided 'label', if it exists.
    pub fn get_untyped_handle<Q>(&self, label: &Q) -> Option<UntypedHandle>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get(label)?;
        Some(labeled.handle.clone())
    }

    /// Returns the [`Handle`] of the labeled asset with the provided 'label', if it exists and is an asset of type `B`
    pub fn get_handle<Q, B: Asset>(&self, label: &Q) -> Option<Handle<B>>
    where
        CowArc<'static, str>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let labeled = self.labeled_assets.get(label)?;
        if let Ok(handle) = labeled.handle.clone().try_typed::<B>() {
            return Some(handle);
        }
        None
    }

    /// Iterate over all labels for "labeled assets" in the loaded asset
    pub fn iter_labels(&self) -> impl Iterator<Item = &str> {
        self.labeled_assets.keys().map(|s| &**s)
    }
}
