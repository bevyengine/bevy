use crate::AssetLoader;
use crate::{io::Writer, meta::Settings, Asset, ErasedLoadedAsset};
use bevy_utils::BoxedFuture;
use serde::{Deserialize, Serialize};

pub trait AssetSaver: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    type OutputLoader: AssetLoader;

    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: &'a Self::Asset,
        settings: &'a Self::Settings,
    ) -> BoxedFuture<'a, Result<<Self::OutputLoader as AssetLoader>::Settings, anyhow::Error>>;
}

pub trait ErasedAssetSaver: Send + Sync + 'static {
    fn process<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: &'a ErasedLoadedAsset,
        settings: &'a dyn Settings,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>>;
    fn type_name(&self) -> &'static str;
}

impl<S: AssetSaver> ErasedAssetSaver for S {
    fn process<'a>(
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
