mod gltf;

pub use self::gltf::load_gltf;

use std::{
    collections::HashMap,
    marker::PhantomData,
    ops::Drop,
    sync::{Arc, RwLock},
};

pub struct Handle<T> {
    pub id: Arc<RwLock<usize>>,
    marker: PhantomData<T>,
    free_indices: Arc<RwLock<Vec<usize>>>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Handle {
            id: self.id.clone(),
            free_indices: self.free_indices.clone(),
            marker: PhantomData,
        }
    }
}

impl<T> Drop for Handle<T> {
    fn drop(&mut self) {
        // TODO: Maybe this should be 1
        // TODO: Is this even necessary?
        if Arc::strong_count(&self.id) == 0 {
            let id = *self.id.read().unwrap();
            self.free_indices.write().unwrap().push(id);
        }
    }
}

pub trait Asset<D> {
    fn load(descriptor: D) -> Self;
}

pub struct AssetStorage<T, D>
where
    T: Asset<D>,
{
    assets: Vec<Option<T>>,
    free_indices: Arc<RwLock<Vec<usize>>>,
    names: HashMap<String, Arc<RwLock<usize>>>,
    marker: PhantomData<D>,
}

impl<T, D> AssetStorage<T, D>
where
    T: Asset<D>,
{
    pub fn new() -> AssetStorage<T, D> {
        AssetStorage {
            assets: Vec::new(),
            free_indices: Arc::new(RwLock::new(Vec::new())),
            names: HashMap::new(),
            marker: PhantomData,
        }
    }

    pub fn get_named(&self, name: &str) -> Option<Handle<T>> {
        match self.names.get(name) {
            Some(id) => Some(Handle {
                id: id.clone(),
                marker: PhantomData,
                free_indices: self.free_indices.clone(),
            }),
            None => None,
        }
    }

    pub fn add(&mut self, asset: T, name: &str) -> Handle<T> {
        match self.free_indices.write().unwrap().pop() {
            Some(id) => {
                self.assets[id as usize] = Some(asset);
                let handle = Arc::new(RwLock::new(id));
                self.names.insert(name.to_string(), handle.clone());
                Handle {
                    id: handle,
                    marker: PhantomData,
                    free_indices: self.free_indices.clone(),
                }
            }
            None => {
                self.assets.push(Some(asset));
                let id = self.assets.len() - 1;
                let handle = Arc::new(RwLock::new(id));
                self.names.insert(name.to_string(), handle.clone());
                Handle {
                    id: handle,
                    marker: PhantomData,
                    free_indices: self.free_indices.clone(),
                }
            }
        }
    }

    pub fn get(&mut self, id: usize) -> Option<&mut T> {
        if id >= self.assets.len() {
            None
        } else {
            if let Some(ref mut asset) = self.assets[id] {
                Some(asset)
            } else {
                None
            }
        }
    }
}
