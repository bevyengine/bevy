use anyhow::Result;
use bevy_app::{AppBuilder, Plugin};
use bevy_asset::{AddAsset, AssetLoader};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    marker::{PhantomData, Send, Sync},
    path::Path,
};
use toml::de::from_slice;

/// a struct wrapping a user defined configuration file
pub struct Config<T>(T)
where
    T: DeserializeOwned + Serialize;

impl<T> Config<T>
where
    T: Serialize + DeserializeOwned,
{
    /// return a reference to the configuration data
    pub fn get_config(&self) -> &T
    where
        T: Serialize + DeserializeOwned,
    {
        &self.0
    }
}

/// bevy asset loader for configuration files
#[derive(Default)]
pub struct ConfigLoader;

impl<T> AssetLoader<Config<T>> for ConfigLoader
where
    T: DeserializeOwned + Serialize,
{
    fn from_bytes(&self, _asset_path: &Path, bytes: Vec<u8>) -> Result<Config<T>, anyhow::Error> {
        let config: T = from_slice(&bytes)?;
        Ok(Config(config))
    }

    fn extensions(&self) -> &[&str] {
        &["toml"]
    }
}

/// bevy configuration plugin
#[derive(Default)]
pub struct ConfigPlugin<T>(PhantomData<T>)
where
    T: Serialize + DeserializeOwned;

impl<T: 'static> Plugin for ConfigPlugin<T>
where
    T: DeserializeOwned + Serialize + Send + Sync,
{
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<Config<T>>()
            .add_asset_loader::<Config<T>, ConfigLoader>();
    }
}
