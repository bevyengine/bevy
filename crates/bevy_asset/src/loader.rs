use crate::{AssetPath, Assets, Handle};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use fs::File;
use io::Read;
use legion::prelude::{Res, ResMut};
use std::{fs, io, path::Path};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AssetLoadError {
    #[error("Encountered an io error while loading asset.")]
    Io(#[from] io::Error),
    #[error("This asset's loader encountered an error while loading.")]
    LoaderError(#[from] anyhow::Error),
}

pub trait AssetLoader<T>: Send + Sync + 'static {
    fn from_bytes(&self, asset_path: &AssetPath, bytes: Vec<u8>) -> Result<T, anyhow::Error>;
    fn extensions(&self) -> &[&str];
    fn load_from_file(&self, asset_path: &AssetPath) -> Result<T, AssetLoadError> {
        let mut file = File::open(Path::new(asset_path.path.as_ref()))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let asset = self.from_bytes(asset_path, bytes)?;
        Ok(asset)
    }
}

pub struct AssetResult<T> {
    pub result: Result<T, AssetLoadError>,
    pub handle: Handle<T>,
    pub path: AssetPath,
}

pub struct AssetChannel<T> {
    pub sender: Sender<AssetResult<T>>,
    pub receiver: Receiver<AssetResult<T>>,
}

impl<T> AssetChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        AssetChannel { sender, receiver }
    }
}

pub fn update_asset_storage_system<T>(
    asset_channel: Res<AssetChannel<T>>,
    mut assets: ResMut<Assets<T>>,
) {
    loop {
        match asset_channel.receiver.try_recv() {
            Ok(result) => {
                let asset = result.result.unwrap();
                assets.add_with_handle(result.handle, asset);
                assets.set_path(result.handle, &result.path.path);
            }
            Err(TryRecvError::Empty) => {
                break;
            }
            Err(TryRecvError::Disconnected) => panic!("AssetChannel disconnected"),
        }
    }
}
