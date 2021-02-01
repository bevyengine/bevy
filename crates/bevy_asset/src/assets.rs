use crate::{
    update_asset_storage_system, Asset, AssetLoader, AssetServer, Handle, HandleId, RefChange,
};
use bevy_app::{prelude::Events, AppBuilder};
use bevy_ecs::{FromResources, IntoSystem, ResMut};
use bevy_reflect::RegisterTypeBuilder;
use bevy_utils::HashMap;
use crossbeam_channel::Sender;
use std::fmt::Debug;

/// Events that happen on assets of type `T`
pub enum AssetEvent<T: Asset> {
    Created { handle: Handle<T> },
    Modified { handle: Handle<T> },
    Removed { handle: Handle<T> },
}

impl<T: Asset> Debug for AssetEvent<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetEvent::Created { handle } => f
                .debug_struct(&format!(
                    "AssetEvent<{}>::Created",
                    std::any::type_name::<T>()
                ))
                .field("handle", &handle.id)
                .finish(),
            AssetEvent::Modified { handle } => f
                .debug_struct(&format!(
                    "AssetEvent<{}>::Modified",
                    std::any::type_name::<T>()
                ))
                .field("handle", &handle.id)
                .finish(),
            AssetEvent::Removed { handle } => f
                .debug_struct(&format!(
                    "AssetEvent<{}>::Removed",
                    std::any::type_name::<T>()
                ))
                .field("handle", &handle.id)
                .finish(),
        }
    }
}

/// Stores Assets of a given type and tracks changes to them.
#[derive(Debug)]
pub struct Assets<T: Asset> {
    assets: HashMap<HandleId, T>,
    events: Events<AssetEvent<T>>,
    pub(crate) ref_change_sender: Sender<RefChange>,
}

impl<T: Asset> Assets<T> {
    pub(crate) fn new(ref_change_sender: Sender<RefChange>) -> Self {
        Assets {
            assets: HashMap::default(),
            events: Events::default(),
            ref_change_sender,
        }
    }

    pub fn add(&mut self, asset: T) -> Handle<T> {
        let id = HandleId::random::<T>();
        self.assets.insert(id, asset);
        self.events.send(AssetEvent::Created {
            handle: Handle::weak(id),
        });
        self.get_handle(id)
    }

    pub fn set<H: Into<HandleId>>(&mut self, handle: H, asset: T) -> Handle<T> {
        let id: HandleId = handle.into();
        if self.assets.insert(id, asset).is_some() {
            self.events.send(AssetEvent::Modified {
                handle: Handle::weak(id),
            });
        } else {
            self.events.send(AssetEvent::Created {
                handle: Handle::weak(id),
            });
        }

        self.get_handle(id)
    }

    pub fn set_untracked<H: Into<HandleId>>(&mut self, handle: H, asset: T) {
        let id: HandleId = handle.into();
        if self.assets.insert(id, asset).is_some() {
            self.events.send(AssetEvent::Modified {
                handle: Handle::weak(id),
            });
        } else {
            self.events.send(AssetEvent::Created {
                handle: Handle::weak(id),
            });
        }
    }

    pub fn get<H: Into<HandleId>>(&self, handle: H) -> Option<&T> {
        self.assets.get(&handle.into())
    }

    pub fn contains<H: Into<HandleId>>(&self, handle: H) -> bool {
        self.assets.contains_key(&handle.into())
    }

    pub fn get_mut<H: Into<HandleId>>(&mut self, handle: H) -> Option<&mut T> {
        let id: HandleId = handle.into();
        self.events.send(AssetEvent::Modified {
            handle: Handle::weak(id),
        });
        self.assets.get_mut(&id)
    }

    pub fn get_handle<H: Into<HandleId>>(&self, handle: H) -> Handle<T> {
        Handle::strong(handle.into(), self.ref_change_sender.clone())
    }

    pub fn get_or_insert_with<H: Into<HandleId>>(
        &mut self,
        handle: H,
        insert_fn: impl FnOnce() -> T,
    ) -> &mut T {
        let mut event = None;
        let id: HandleId = handle.into();
        let borrowed = self.assets.entry(id).or_insert_with(|| {
            event = Some(AssetEvent::Created {
                handle: Handle::weak(id),
            });
            insert_fn()
        });

        if let Some(event) = event {
            self.events.send(event);
        }
        borrowed
    }

    pub fn iter(&self) -> impl Iterator<Item = (HandleId, &T)> {
        self.assets.iter().map(|(k, v)| (*k, v))
    }

    pub fn ids(&self) -> impl Iterator<Item = HandleId> + '_ {
        self.assets.keys().cloned()
    }

    pub fn remove<H: Into<HandleId>>(&mut self, handle: H) -> Option<T> {
        let id: HandleId = handle.into();
        let asset = self.assets.remove(&id);
        if asset.is_some() {
            self.events.send(AssetEvent::Removed {
                handle: Handle::weak(id),
            });
        }
        asset
    }

    /// Clears the inner asset map, removing all key-value pairs.
    ///
    /// Keeps the allocated memory for reuse.
    pub fn clear(&mut self) {
        self.assets.clear()
    }

    /// Reserves capacity for at least additional more elements to be inserted into the assets.
    ///
    /// The collection may reserve more space to avoid frequent reallocations.
    pub fn reserve(&mut self, additional: usize) {
        self.assets.reserve(additional)
    }

    /// Shrinks the capacity of the asset map as much as possible.
    ///
    /// It will drop down as much as possible while maintaining the internal rules and possibly
    /// leaving some space in accordance with the resize policy.
    pub fn shrink_to_fit(&mut self) {
        self.assets.shrink_to_fit()
    }

    pub fn asset_event_system(
        mut events: ResMut<Events<AssetEvent<T>>>,
        mut assets: ResMut<Assets<T>>,
    ) {
        events.extend(assets.events.drain())
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

/// [AppBuilder] extension methods for adding new asset types
pub trait AddAsset {
    fn add_asset<T>(&mut self) -> &mut Self
    where
        T: Asset;
    fn init_asset_loader<T>(&mut self) -> &mut Self
    where
        T: AssetLoader + FromResources;
    fn add_asset_loader<T>(&mut self, loader: T) -> &mut Self
    where
        T: AssetLoader;
}

impl AddAsset for AppBuilder {
    fn add_asset<T>(&mut self) -> &mut Self
    where
        T: Asset,
    {
        let assets = {
            let asset_server = self.resources().get::<AssetServer>().unwrap();
            asset_server.register_asset_type::<T>()
        };

        self.add_resource(assets)
            .add_system_to_stage(
                super::stage::ASSET_EVENTS,
                Assets::<T>::asset_event_system.system(),
            )
            .add_system_to_stage(
                crate::stage::LOAD_ASSETS,
                update_asset_storage_system::<T>.system(),
            )
            .register_type::<Handle<T>>()
            .add_event::<AssetEvent<T>>()
    }

    fn init_asset_loader<T>(&mut self) -> &mut Self
    where
        T: AssetLoader + FromResources,
    {
        self.add_asset_loader(T::from_resources(self.resources()))
    }

    fn add_asset_loader<T>(&mut self, loader: T) -> &mut Self
    where
        T: AssetLoader,
    {
        self.resources()
            .get_mut::<AssetServer>()
            .expect("AssetServer does not exist. Consider adding it as a resource.")
            .add_loader(loader);
        self
    }
}
