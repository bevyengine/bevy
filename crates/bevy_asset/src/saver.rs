use crate::AssetLoader;
use crate::{io::Writer, meta::Settings, Asset, ErasedLoadedAsset};
use bevy_utils::BoxedFuture;
use serde::{Deserialize, Serialize};

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
        asset: &'a Self::Asset,
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
            let asset = asset.get::<S::Asset>().unwrap();
            self.save(writer, asset, settings).await?;
            Ok(())
        })
    }
    fn type_name(&self) -> &'static str {
        std::any::type_name::<S>()
    }
}
