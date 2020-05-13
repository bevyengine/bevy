mod handle;
pub use handle::*;

use bevy_app::{stage, AppBuilder, Events};
use bevy_core::bytes::GetBytes;
use legion::prelude::*;
use std::collections::HashMap;

pub enum AssetEvent<T> {
    Created { handle: Handle<T> },
}

pub struct Assets<T> {
    assets: HashMap<HandleId, T>,
    names: HashMap<String, Handle<T>>,
    events: Events<AssetEvent<T>>,
}

impl<T> Default for Assets<T> {
    fn default() -> Self {
        Assets {
            assets: HashMap::default(),
            names: HashMap::default(),
            events: Events::default(),
        }
    }
}

impl<T> Assets<T> {
    pub fn get_named(&mut self, name: &str) -> Option<Handle<T>> {
        self.names.get(name).map(|handle| *handle)
    }

    pub fn add(&mut self, asset: T) -> Handle<T> {
        let id = HandleId::new();
        self.assets.insert(id, asset);
        let handle = Handle::from_id(id);
        self.events.send(AssetEvent::Created { handle });
        handle
    }

    pub fn add_with_handle(&mut self, handle: Handle<T>, asset: T) {
        self.assets.insert(handle.id, asset);
        self.events.send(AssetEvent::Created { handle });
    }

    pub fn add_default(&mut self, asset: T) -> Handle<T> {
        self.assets.insert(DEFAULT_HANDLE_ID, asset);
        let handle = Handle::default();
        self.events.send(AssetEvent::Created { handle });
        handle
    }

    pub fn set_name(&mut self, name: &str, handle: Handle<T>) {
        self.names.insert(name.to_string(), handle);
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

    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.assets.iter().map(|(k, v)| (Handle::from_id(*k), v))
    }

    pub fn asset_event_system(
        mut events: ResourceMut<Events<AssetEvent<T>>>,
        mut assets: ResourceMut<Assets<T>>,
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
}

impl AddAsset for AppBuilder {
    fn add_asset<T>(&mut self) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.init_resource::<Assets<T>>()
            .add_system_to_stage(
                stage::EVENT_UPDATE,
                Assets::<T>::asset_event_system.system(),
            )
            .add_event::<AssetEvent<T>>()
    }
}
