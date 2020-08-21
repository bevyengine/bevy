use crate::{AssetServer, AssetVersion, Assets, Handle, HandleId, LoadState};
use anyhow::Result;
use bevy_ecs::{Res, ResMut, Resource};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use fs::File;
use std::{
    any::Any,
    fs,
    io::{self, Read},
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

/// A loader for a given asset type.
pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Send + Sync + 'static;

    fn extensions(&self) -> &[&str];
    fn from_bytes(&self, asset_path: &Path, bytes: Vec<u8>) -> Result<Self::Asset, anyhow::Error>;
    fn load_from_file(&self, asset_path: &Path) -> Result<Self::Asset, AssetLoadError> {
        let mut file = File::open(asset_path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let asset = self.from_bytes(asset_path, bytes)?;
        Ok(asset)
    }
}

pub(crate) struct ChannelAssetLoader<A: 'static> {
    pub(crate) sender: Sender<AssetResult<A>>,
    pub(crate) loader: Box<dyn AssetLoader<Asset = A>>,
}

pub(crate) trait UntypedLoader: Send + Sync + Any {
    fn load_from_file(&self, data: LoadData);
    fn cast_to_any(&self) -> &dyn Any;
}

impl dyn UntypedLoader {
    pub fn downcast_loader<A: 'static>(&self) -> Option<&dyn AssetLoader<Asset = A>> {
        self.cast_to_any()
            .downcast_ref::<ChannelAssetLoader<A>>()
            .map(|cal| &*cal.loader)
    }
}

impl<A> UntypedLoader for ChannelAssetLoader<A>
where
    A: Send + Sync + 'static,
{
    fn load_from_file(&self, data: LoadData) {
        let result = self.loader.load_from_file(&data.path);
        let msg = AssetResult {
            result,
            handle: Handle::from_id(data.handle_id),
            path: data.path,
            version: data.version,
        };
        self.sender
            .send(msg)
            .expect("loaded asset should have been sent");
    }

    fn cast_to_any(&self) -> &dyn Any {
        self
    }
}

pub(crate) struct LoadData {
    pub(crate) path: PathBuf,
    pub(crate) handle_id: HandleId,
    pub(crate) version: AssetVersion,
}

/// The result of loading an asset of type `T`
pub(crate) struct AssetResult<T: 'static> {
    pub result: Result<T, AssetLoadError>,
    pub handle: Handle<T>,
    pub path: PathBuf,
    pub version: AssetVersion,
}

/// A channel to send and receive [AssetResult]s
pub(crate) struct AssetChannel<T: 'static> {
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
pub(crate) fn update_asset_storage_system<T: Resource>(
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
