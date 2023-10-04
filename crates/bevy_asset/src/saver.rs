use crate::{io::Writer, meta::Settings, Asset, ErasedLoadedAsset};
use crate::{AssetLoader, LabeledAsset};
use bevy_utils::{BoxedFuture, CowArc, HashMap};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// Saves an [`Asset`] of a given [`AssetSaver::Asset`] type. [`AssetSaver::OutputLoader`] will then be used to load the saved asset
/// in the final deployed application. The saver should produce asset bytes in a format that [`AssetSaver::OutputLoader`] can read.
pub trait AssetSaver: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    type OutputLoader: AssetLoader;

    /// Saves the given runtime [`Asset`] by writing it to a byte format using `writer`. The passed in `settings` can influence how the
    /// `asset` is saved.  
    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: SavedAsset<'a, Self::Asset>,
        settings: &'a Self::Settings,
    ) -> BoxedFuture<'a, Result<<Self::OutputLoader as AssetLoader>::Settings, anyhow::Error>>;
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
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>>;

    /// The type name of the [`AssetSaver`].
    fn type_name(&self) -> &'static str;
}

impl<S: AssetSaver> ErasedAssetSaver for S {
    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: &'a ErasedLoadedAsset,
        settings: &'a dyn Settings,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let settings = settings
                .downcast_ref::<S::Settings>()
                .expect("AssetLoader settings should match the loader type");
            let saved_asset = SavedAsset::<S::Asset>::from_loaded(asset).unwrap();
            self.save(writer, saved_asset, settings).await?;
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

    /// Retrieves the value of this asset.
    #[inline]
    pub fn get(&self) -> &'a A {
        self.value
    }

    /// Returns the labeled asset, if it exists and matches this type.
    pub fn get_labeled<B: Asset>(
        &self,
        label: impl Into<CowArc<'static, str>>,
    ) -> Option<SavedAsset<B>> {
        let labeled = self.labeled_assets.get(&label.into())?;
        let value = labeled.asset.value.downcast_ref::<B>()?;
        Some(SavedAsset {
            value,
            labeled_assets: &labeled.asset.labeled_assets,
        })
    }

    /// Returns the type-erased labeled asset, if it exists and matches this type.
    pub fn get_erased_labeled(
        &self,
        label: impl Into<CowArc<'static, str>>,
    ) -> Option<&ErasedLoadedAsset> {
        let labeled = self.labeled_assets.get(&label.into())?;
        Some(&labeled.asset)
    }

    /// Iterate over all labels for "labeled assets" in the loaded asset
    pub fn iter_labels(&self) -> impl Iterator<Item = &str> {
        self.labeled_assets.keys().map(|s| &**s)
    }
}
