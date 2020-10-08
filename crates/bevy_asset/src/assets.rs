use crate::{
    update_asset_storage_system, AssetChannel, AssetLoader, AssetServer, ChannelAssetHandler,
    Handle, HandleId,
};
use bevy_app::{prelude::Events, AppBuilder};
use bevy_ecs::{FromResources, IntoQuerySystem, ResMut, Resource};
use bevy_type_registry::RegisterType;
use bevy_utils::HashMap;

/// Events that happen on assets of type `T`
#[derive(Debug)]
pub enum AssetEvent<T: Resource> {
    Created { handle: Handle<T> },
    Modified { handle: Handle<T> },
    Removed { handle: Handle<T> },
}

/// Stores Assets of a given type and tracks changes to them.
#[derive(Debug)]
pub struct Assets<T: Resource> {
    assets: HashMap<Handle<T>, T>,
    events: Events<AssetEvent<T>>,
}

impl<T: Resource> Default for Assets<T> {
    fn default() -> Self {
        Assets {
            assets: HashMap::default(),
            events: Events::default(),
        }
    }
}

impl<T: Resource> Assets<T> {
    pub fn add(&mut self, asset: T) -> Handle<T> {
        let handle = Handle::new();
        self.assets.insert(handle, asset);
        self.events.send(AssetEvent::Created { handle });
        handle
    }

    pub fn set(&mut self, handle: Handle<T>, asset: T) {
        let exists = self.assets.contains_key(&handle);
        self.assets.insert(handle, asset);

        if exists {
            self.events.send(AssetEvent::Modified { handle });
        } else {
            self.events.send(AssetEvent::Created { handle });
        }
    }

    pub fn add_default(&mut self, asset: T) -> Handle<T> {
        let handle = Handle::default();
        let exists = self.assets.contains_key(&handle);
        self.assets.insert(handle, asset);
        if exists {
            self.events.send(AssetEvent::Modified { handle });
        } else {
            self.events.send(AssetEvent::Created { handle });
        }
        handle
    }

    pub fn get_with_id(&self, id: HandleId) -> Option<&T> {
        self.get(&Handle::from_id(id))
    }

    pub fn get_with_id_mut(&mut self, id: HandleId) -> Option<&mut T> {
        self.get_mut(&Handle::from_id(id))
    }

    pub fn get(&self, handle: &Handle<T>) -> Option<&T> {
        self.assets.get(&handle)
    }

    pub fn get_mut(&mut self, handle: &Handle<T>) -> Option<&mut T> {
        self.events.send(AssetEvent::Modified { handle: *handle });
        self.assets.get_mut(&handle)
    }

    pub fn get_or_insert_with(
        &mut self,
        handle: Handle<T>,
        insert_fn: impl FnOnce() -> T,
    ) -> &mut T {
        let mut event = None;
        let borrowed = self.assets.entry(handle).or_insert_with(|| {
            event = Some(AssetEvent::Created { handle });
            insert_fn()
        });

        if let Some(event) = event {
            self.events.send(event);
        }
        borrowed
    }

    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.assets.iter().map(|(k, v)| (*k, v))
    }

    pub fn remove(&mut self, handle: &Handle<T>) -> Option<T> {
        self.assets.remove(&handle)
    }

    pub fn asset_event_system(
        mut events: ResMut<Events<AssetEvent<T>>>,
        mut assets: ResMut<Assets<T>>,
    ) {
        events.extend(assets.events.drain())
    }
}

/// [AppBuilder] extension methods for adding new asset types
pub trait AddAsset {
    fn add_asset<T>(&mut self) -> &mut Self
    where
        T: Send + Sync + 'static;
    fn add_asset_loader<TAsset, TLoader>(&mut self) -> &mut Self
    where
        TLoader: AssetLoader<TAsset> + FromResources,
        TAsset: Send + Sync + 'static;
    fn add_asset_loader_from_instance<TAsset, TLoader>(&mut self, instance: TLoader) -> &mut Self
    where
        TLoader: AssetLoader<TAsset> + FromResources,
        TAsset: Send + Sync + 'static;
}

impl AddAsset for AppBuilder {
    fn add_asset<T>(&mut self) -> &mut Self
    where
        T: Resource,
    {
        self.init_resource::<Assets<T>>()
            .register_component::<Handle<T>>()
            .add_system_to_stage(
                super::stage::ASSET_EVENTS,
                Assets::<T>::asset_event_system.system(),
            )
            .add_event::<AssetEvent<T>>()
    }

    fn add_asset_loader_from_instance<TAsset, TLoader>(&mut self, instance: TLoader) -> &mut Self
    where
        TLoader: AssetLoader<TAsset> + FromResources,
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
            asset_server.add_loader(instance);
            let handler = ChannelAssetHandler::new(
                TLoader::from_resources(self.resources()),
                asset_channel.sender.clone(),
            );
            asset_server.add_handler(handler);
        }
        self
    }

    fn add_asset_loader<TAsset, TLoader>(&mut self) -> &mut Self
    where
        TLoader: AssetLoader<TAsset> + FromResources,
        TAsset: Send + Sync + 'static,
    {
        self.add_asset_loader_from_instance::<TAsset, TLoader>(TLoader::from_resources(
            self.resources(),
        ))
    }
}
