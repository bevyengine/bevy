use bevy::{asset::AssetLoader, prelude::*};
use ron::de::from_bytes;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
pub struct MyCustomData {
    pub num: i32,
}

#[derive(Deserialize)]
pub struct MySecondCustomData {
    pub is_set: bool,
}

// create a custom loader for data files
#[derive(Default)]
pub struct DataFileLoader {
    matching_extensions: Vec<&'static str>,
}

impl DataFileLoader {
    pub fn from_extensions(matching_extensions: Vec<&'static str>) -> Self {
        DataFileLoader {
            matching_extensions,
        }
    }
}

impl<TAsset> AssetLoader<TAsset> for DataFileLoader
where
    for<'de> TAsset: Deserialize<'de>,
{
    fn from_bytes(&self, _asset_path: &Path, bytes: Vec<u8>) -> Result<TAsset, anyhow::Error> {
        Ok(from_bytes::<TAsset>(bytes.as_slice())?)
    }

    fn extensions(&self) -> &[&str] {
        self.matching_extensions.as_slice()
    }
}

/// This example illustrates various ways to load assets
fn main() {
    App::build()
        .add_default_plugins()
        .add_asset::<MyCustomData>()
        .add_asset_loader_from_instance::<MyCustomData, DataFileLoader>(
            DataFileLoader::from_extensions(vec!["data1"]),
        )
        .add_asset::<MySecondCustomData>()
        .add_asset_loader_from_instance::<MySecondCustomData, DataFileLoader>(
            DataFileLoader::from_extensions(vec!["data2"]),
        )
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    asset_server: Res<AssetServer>,
    mut data1s: ResMut<Assets<MyCustomData>>,
    mut data2s: ResMut<Assets<MySecondCustomData>>,
) {
    let data1_handle = asset_server
        .load_sync(&mut data1s, "assets/data/test_data.data1")
        .unwrap();
    let data2_handle = asset_server
        .load_sync(&mut data2s, "assets/data/test_data.data2")
        .unwrap();

    let data1 = data1s.get(&data1_handle).unwrap();
    println!("Data 1 loaded with value {}", data1.num);

    let data2 = data2s.get(&data2_handle).unwrap();
    println!("Data 2 loaded with value {}", data2.is_set);
}
