mod gltf;
mod mesh;
mod texture;

pub use self::gltf::load_gltf;
use std::hash::{Hash, Hasher};
pub use mesh::*;
pub use texture::*;

use std::{collections::HashMap, marker::PhantomData};

pub struct Handle<T> {
    pub id: usize,
    marker: PhantomData<T>,
}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Handle<T> {}

// TODO: somehow handle this gracefully in asset managers. or alternatively remove Default
impl<T> Default for Handle<T> {
    fn default() -> Self {
        Handle {
            id: std::usize::MAX,
            marker: PhantomData,
        }
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Handle {
            id: self.id.clone(),
            marker: PhantomData,
        }
    }
}

pub trait Asset<D> {
    fn load(descriptor: D) -> Self;
}

pub struct AssetStorage<T> {
    assets: HashMap<usize, T>,
    names: HashMap<String, usize>,
    current_index: usize,
}

impl<T> AssetStorage<T> {
    pub fn new() -> AssetStorage<T> {
        AssetStorage {
            assets: HashMap::new(),
            names: HashMap::new(),
            current_index: 0,
        }
    }

    pub fn get_named(&mut self, name: &str) -> Option<&mut T> {
        match self.names.get(name) {
            Some(id) => self.assets.get_mut(id),
            None => None,
        }
    }

    pub fn add(&mut self, asset: T) -> Handle<T> {
        let id = self.current_index;
        self.current_index += 1;
        self.assets.insert(id, asset);
        Handle {
            id,
            marker: PhantomData,
        }
    }

    pub fn add_named(&mut self, asset: T, name: &str) -> Handle<T> {
        let handle = self.add(asset);
        self.names.insert(name.to_string(), handle.id);
        handle
    }

    pub fn get_id(&self, id: usize) -> Option<&T> {
        self.assets.get(&id)
    }

    pub fn get_id_mut(&mut self, id: usize) -> Option<&mut T> {
        self.assets.get_mut(&id)
    }

    pub fn get(&self, handle: &Handle<T>) -> Option<&T> {
        self.assets.get(&handle.id)
    }

    pub fn get_mut(&mut self, handle: &Handle<T>) -> Option<&mut T> {
        self.assets.get_mut(&handle.id)
    }
}
