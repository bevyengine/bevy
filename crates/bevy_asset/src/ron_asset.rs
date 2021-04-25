use crate::{Asset, AssetLoader, AssetServer, BoxedFuture, Handle, LoadContext, LoadedAsset};
use bevy_log::warn;
use bevy_reflect::TypeUuid;
use serde::Deserialize;

#[derive(Debug, Deserialize, TypeUuid)]
#[uuid = "2150da22-9881-41e6-89db-62777c6dcbee"]
pub struct RonAsset(ron::Value);

#[derive(Default)]
pub struct RonAssetLoader;

impl AssetLoader for RonAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let ron_value = ron::de::from_bytes::<ron::Value>(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(RonAsset(ron_value)));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

pub trait RonAssetDeserializer {
    fn deserialize<T: for<'de> Deserialize<'de> + Asset>(&self, path: &str) -> Handle<T>;
}

impl RonAssetDeserializer for AssetServer {
    fn deserialize<T: for<'de> Deserialize<'de> + Asset>(&self, path: &str) -> Handle<T> {
        let ron_handle: Handle<RonAsset> = self.load(path);
        let path = path.to_string();
        self.create_from(ron_handle, move |ron| {
            match ron.0.clone().into_rust::<T>() {
                Ok(deserialized) => Some(deserialized),
                Err(e) => {
                    warn!("error deserializing \"{}\": {}", path, e);
                    None
                }
            }
        })
    }
}
