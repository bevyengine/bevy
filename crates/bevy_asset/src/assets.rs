use crate::{
    update_asset_storage_system, AssetChannel, AssetLoader, AssetServer, ChannelAssetHandler,
    Handle, HandleId, DEFAULT_HANDLE_ID,
};
use bevy_app::{stage, AppBuilder, Events};
use bevy_core::bytes::GetBytes;
use legion::prelude::*;
use std::{path::{Path, PathBuf}, collections::HashMap};

pub enum AssetEvent<T> {
    Created { handle: Handle<T> },
    Modified { handle: Handle<T> },
}

pub struct Assets<T> {
    assets: HashMap<HandleId, T>,
    paths: HashMap<PathBuf, Handle<T>>,
    events: Events<AssetEvent<T>>,
}

impl<T> Default for Assets<T> {
    fn default() -> Self {
        Assets {
            assets: HashMap::default(),
            paths: HashMap::default(),
            events: Events::default(),
        }
    }
}

impl<T> Assets<T> {
    pub fn get_with_path<P: AsRef<Path>>(&mut self, path: P) -> Option<Handle<T>> {
        self.paths.get(path.as_ref()).map(|handle| *handle)
    }

    pub fn add(&mut self, asset: T) -> Handle<T> {
        let id = HandleId::new();
        self.assets.insert(id, asset);
        let handle = Handle::from_id(id);
        self.events.send(AssetEvent::Created { handle });
        handle
    }

    pub fn set(&mut self, handle: Handle<T>, asset: T) {
        let exists = self.assets.contains_key(&handle.id);
        self.assets.insert(handle.id, asset);

        if exists {
            self.events.send(AssetEvent::Modified { handle });
        } else {
            self.events.send(AssetEvent::Created { handle });
        }
    }

    pub fn add_default(&mut self, asset: T) -> Handle<T> {
        let exists = self.assets.contains_key(&DEFAULT_HANDLE_ID);
        self.assets.insert(DEFAULT_HANDLE_ID, asset);
        let handle = Handle::default();
        if exists {
            self.events.send(AssetEvent::Modified { handle });
        } else {
            self.events.send(AssetEvent::Created { handle });
        }
        handle
    }

    pub fn set_path<P: AsRef<Path>>(&mut self, handle: Handle<T>, path: P) {
        self.paths.insert(path.as_ref().to_owned(), handle);
    }

    pub fn get_id(&self, id: HandleId) -> Option<&T> {
        self.assets.get(&id)
    }

    pub fn get_id_mut(&mut self, id: HandleId) -> Option<&mut T> {
        self.assets.get_mut(&id)
    }

    pub fn get(&self, handle: &Handle<T>) -> Option<&T> {
        self.assets.get(&handle.id)
    }

    pub fn get_mut(&mut self, handle: &Handle<T>) -> Option<&mut T> {
        self.assets.get_mut(&handle.id)
    }

    pub fn get_or_insert_with(&mut self, handle: Handle<T>, insert_fn: impl FnOnce() -> T) -> &mut T {
        self.assets.entry(handle.id).or_insert_with(insert_fn)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.assets.iter().map(|(k, v)| (Handle::from_id(*k), v))
    }

    pub fn asset_event_system(
        mut events: ResMut<Events<AssetEvent<T>>>,
        mut assets: ResMut<Assets<T>>,
    ) {
        events.extend(assets.events.drain())
    }
}

impl<T> GetBytes for Handle<T> {
    fn get_bytes(&self) -> Vec<u8> {
        Vec::new()
    }

    fn get_bytes_ref(&self) -> Option<&[u8]> {
        None
    }
}

pub trait AddAsset {
    fn add_asset<T>(&mut self) -> &mut Self
    where
        T: Send + Sync + 'static;
    fn add_asset_loader<TLoader, TAsset>(&mut self, loader: TLoader) -> &mut Self
    where
        TLoader: AssetLoader<TAsset> + Clone,
        TAsset: Send + Sync + 'static;
}

impl AddAsset for AppBuilder {
    fn add_asset<T>(&mut self) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.init_resource::<Assets<T>>()
            .add_system_to_stage(stage::POST_UPDATE, Assets::<T>::asset_event_system.system())
            .add_event::<AssetEvent<T>>()
    }

    fn add_asset_loader<TLoader, TAsset>(&mut self, loader: TLoader) -> &mut Self
    where
        TLoader: AssetLoader<TAsset> + Clone,
        TAsset: Send + Sync + 'static,
    {
        {
            if !self.resources().contains::<AssetChannel<TAsset>>() {
                self.resources_mut().insert(AssetChannel::<TAsset>::new());
                self.add_system_to_stage(
                    crate::stage::LOAD_ASSETS,
                    update_asset_storage_system::<TAsset>.system(),
                );
            }
            let asset_channel = self
                .resources()
                .get::<AssetChannel<TAsset>>()
                .expect("AssetChannel should always exist at this point.");
            let mut asset_server = self
                .resources()
                .get_mut::<AssetServer>()
                .expect("AssetServer does not exist. Consider adding it as a resource.");
            asset_server.add_loader(loader.clone());
            let handler = ChannelAssetHandler::new(loader, asset_channel.sender.clone());
            asset_server.add_handler(handler);
        }
        self
    }
}
