mod handle;
pub use handle::*;

use bevy_core::bytes::GetBytes;
use std::collections::HashMap;

pub struct AssetStorage<T> {
    assets: HashMap<HandleId, T>,
    names: HashMap<String, Handle<T>>,
}

impl<T> AssetStorage<T> {
    pub fn new() -> AssetStorage<T> {
        AssetStorage {
            assets: HashMap::new(),
            names: HashMap::new(),
        }
    }

    pub fn get_named(&mut self, name: &str) -> Option<Handle<T>> {
        self.names.get(name).map(|handle| *handle)
    }

    pub fn add(&mut self, asset: T) -> Handle<T> {
        let id = HandleId::new();
        self.assets.insert(id, asset);
        Handle::from_id(id)
    }

    pub fn add_with_handle(&mut self, handle: Handle<T>, asset: T) {
        self.assets.insert(handle.id, asset);
    }

    pub fn add_default(&mut self, asset: T) -> Handle<T> {
        self.assets.insert(DEFAULT_HANDLE_ID, asset);
        Handle::default()
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
}

impl<T> GetBytes for Handle<T> {
    fn get_bytes(&self) -> Vec<u8> {
        Vec::new()
    }

    fn get_bytes_ref(&self) -> Option<&[u8]> {
        None
    }
}
