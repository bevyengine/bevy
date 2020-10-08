use crate::{AssetServer, AssetVersion, Assets, Handle, LoadState};
use anyhow::Result;
use bevy_ecs::{Res, ResMut, Resource};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use fs::File;
use io::Read;
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Errors that occur while loading assets
#[derive(Error, Debug)]
pub enum AssetLoadError {
    #[error("Encountered an io error while loading asset.")]
    Io(#[from] io::Error),
    #[error("This asset's loader encountered an error while loading.")]
    LoaderError(#[from] anyhow::Error),
}

/// A loader for a given asset of type `T`
pub trait AssetLoader<T>: Send + Sync + 'static {
    fn from_bytes(&self, asset_path: &Path, bytes: Vec<u8>) -> Result<T, anyhow::Error>;
    fn extensions(&self) -> &[&str];
    fn load_from_file(&self, asset_path: &Path) -> Result<T, AssetLoadError> {
        let mut file = File::open(asset_path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let asset = self.from_bytes(asset_path, bytes)?;
        Ok(asset)
    }
}

/// The result of loading an asset of type `T`
#[derive(Debug)]
pub struct AssetResult<T: 'static> {
    pub result: Result<T, AssetLoadError>,
    pub handle: Handle<T>,
    pub path: PathBuf,
    pub version: AssetVersion,
}

/// A channel to send and receive [AssetResult]s
#[derive(Debug)]
pub struct AssetChannel<T: 'static> {
    pub sender: Sender<AssetResult<T>>,
    pub receiver: Receiver<AssetResult<T>>,
}

impl<T> AssetChannel<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        AssetChannel { sender, receiver }
    }
}

/// Reads [AssetResult]s from an [AssetChannel] and updates the [Assets] collection and [LoadState] accordingly
pub fn update_asset_storage_system<T: Resource>(
    asset_channel: Res<AssetChannel<T>>,
    asset_server: Res<AssetServer>,
    mut assets: ResMut<Assets<T>>,
) {
    loop {
        match asset_channel.receiver.try_recv() {
            Ok(result) => match result.result {
                Ok(asset) => {
                    assets.set(result.handle, asset);
                    asset_server
                        .set_load_state(result.handle.id, LoadState::Loaded(result.version));
                }
                Err(err) => {
                    asset_server
                        .set_load_state(result.handle.id, LoadState::Failed(result.version));
                    log::error!("Failed to load asset: {:?}", err);
                }
            },
            Err(TryRecvError::Empty) => {
                break;
            }
            Err(TryRecvError::Disconnected) => panic!("AssetChannel disconnected"),
        }
    }
}
