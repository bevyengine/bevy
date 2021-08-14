use crate::{
    path::AssetPath, AssetIo, AssetIoError, AssetMeta, AssetServer, Assets, Handle, HandleId,
    RefChangeChannel,
};
use anyhow::Result;
use bevy_ecs::{
    component::Component,
    system::{Res, ResMut},
};
use bevy_reflect::{TypeUuid, TypeUuidDynamic};
use bevy_tasks::TaskPool;
use bevy_utils::{BoxedFuture, HashMap};
use crossbeam_channel::{Receiver, Sender};
use downcast_rs::{impl_downcast, Downcast};
use std::path::Path;

/// A loader for an asset source
pub trait AssetLoader: Send + Sync + 'static {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>>;
    fn extensions(&self) -> &[&str];
}

pub trait Asset: TypeUuid + AssetDynamic {}

pub trait AssetDynamic: Downcast + TypeUuidDynamic + Send + Sync + 'static {}
impl_downcast!(AssetDynamic);

impl<T> Asset for T where T: TypeUuid + AssetDynamic + TypeUuidDynamic {}

impl<T> AssetDynamic for T where T: Send + Sync + 'static + TypeUuidDynamic {}

pub struct LoadedAsset<T: Asset> {
    pub(crate) value: Option<T>,
    pub(crate) dependencies: Vec<AssetPath<'static>>,
}

impl<T: Asset> LoadedAsset<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Some(value),
            dependencies: Vec::new(),
        }
    }

    pub fn with_dependency(mut self, asset_path: AssetPath) -> Self {
        self.dependencies.push(asset_path.to_owned());
        self
    }

    pub fn with_dependencies(mut self, asset_paths: Vec<AssetPath<'static>>) -> Self {
        self.dependencies.extend(asset_paths);
        self
    }
}

pub(crate) struct BoxedLoadedAsset {
    pub(crate) value: Option<Box<dyn AssetDynamic>>,
    pub(crate) dependencies: Vec<AssetPath<'static>>,
}

impl<T: Asset> From<LoadedAsset<T>> for BoxedLoadedAsset {
    fn from(asset: LoadedAsset<T>) -> Self {
        BoxedLoadedAsset {
            value: asset
                .value
                .map(|value| Box::new(value) as Box<dyn AssetDynamic>),
            dependencies: asset.dependencies,
        }
    }
}

pub struct LoadContext<'a> {
    pub(crate) ref_change_channel: &'a RefChangeChannel,
    pub(crate) asset_io: &'a dyn AssetIo,
    pub(crate) labeled_assets: HashMap<Option<String>, BoxedLoadedAsset>,
    pub(crate) path: &'a Path,
    pub(crate) version: usize,
    pub(crate) task_pool: &'a TaskPool,
}

impl<'a> LoadContext<'a> {
    pub(crate) fn new(
        path: &'a Path,
        ref_change_channel: &'a RefChangeChannel,
        asset_io: &'a dyn AssetIo,
        version: usize,
        task_pool: &'a TaskPool,
    ) -> Self {
        Self {
            ref_change_channel,
            asset_io,
            labeled_assets: Default::default(),
            version,
            path,
            task_pool,
        }
    }

    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn has_labeled_asset(&self, label: &str) -> bool {
        self.labeled_assets.contains_key(&Some(label.to_string()))
    }

    pub fn set_default_asset<T: Asset>(&mut self, asset: LoadedAsset<T>) {
        self.labeled_assets.insert(None, asset.into());
    }

    pub fn set_labeled_asset<T: Asset>(&mut self, label: &str, asset: LoadedAsset<T>) -> Handle<T> {
        assert!(!label.is_empty());
        self.labeled_assets
            .insert(Some(label.to_string()), asset.into());
        self.get_handle(AssetPath::new_ref(self.path(), Some(label)))
    }

    pub fn get_handle<I: Into<HandleId>, T: Asset>(&self, id: I) -> Handle<T> {
        Handle::strong(id.into(), self.ref_change_channel.sender.clone())
    }

    pub async fn read_asset_bytes<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>, AssetIoError> {
        self.asset_io.load_path(path.as_ref()).await
    }

    pub fn get_asset_metas(&self) -> Vec<AssetMeta> {
        let mut asset_metas = Vec::new();
        for (label, asset) in self.labeled_assets.iter() {
            asset_metas.push(AssetMeta {
                dependencies: asset.dependencies.clone(),
                label: label.clone(),
                type_uuid: asset.value.as_ref().unwrap().type_uuid(),
            });
        }
        asset_metas
    }

    pub fn task_pool(&self) -> &TaskPool {
        self.task_pool
    }
}

/// The result of loading an asset of type `T`
#[derive(Debug)]
pub struct AssetResult<T: Component> {
    pub asset: Box<T>,
    pub id: HandleId,
    pub version: usize,
}

/// A channel to send and receive [AssetResult]s
#[derive(Debug)]
pub struct AssetLifecycleChannel<T: Component> {
    pub sender: Sender<AssetLifecycleEvent<T>>,
    pub receiver: Receiver<AssetLifecycleEvent<T>>,
}

pub enum AssetLifecycleEvent<T: Component> {
    Create(AssetResult<T>),
    Free(HandleId),
}

pub trait AssetLifecycle: Downcast + Send + Sync + 'static {
    fn create_asset(&self, id: HandleId, asset: Box<dyn AssetDynamic>, version: usize);
    fn free_asset(&self, id: HandleId);
}
impl_downcast!(AssetLifecycle);

impl<T: AssetDynamic> AssetLifecycle for AssetLifecycleChannel<T> {
    fn create_asset(&self, id: HandleId, asset: Box<dyn AssetDynamic>, version: usize) {
        if let Ok(asset) = asset.downcast::<T>() {
            self.sender
                .send(AssetLifecycleEvent::Create(AssetResult {
                    asset,
                    id,
                    version,
                }))
                .unwrap()
        } else {
            panic!(
                "Failed to downcast asset to {}.",
                std::any::type_name::<T>()
            );
        }
    }

    fn free_asset(&self, id: HandleId) {
        self.sender.send(AssetLifecycleEvent::Free(id)).unwrap();
    }
}

impl<T: Component> Default for AssetLifecycleChannel<T> {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        AssetLifecycleChannel { sender, receiver }
    }
}

/// Updates the [Assets] collection according to the changes queued up by [AssetServer].
pub fn update_asset_storage_system<T: Asset + AssetDynamic>(
    asset_server: Res<AssetServer>,
    assets: ResMut<Assets<T>>,
) {
    asset_server.update_asset_storage(assets);
}
