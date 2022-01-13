use crate::{
    update_asset_storage_system, Asset, AssetLoader, AssetServer, AssetStage, Handle, HandleId,
    RefChange,
};
use bevy_app::{App, EventWriter, Events};
use bevy_ecs::{system::ResMut, world::FromWorld};
use bevy_utils::{HashMap, HashSet};
use crossbeam_channel::Sender;
use std::fmt::Debug;

#[derive(Debug)]
enum AssetEventType {
    Created,
    Modified,
    Removed,
}

/// Events that happen on assets of type `T`
///
/// Events sent via the [Assets] struct will always be sent with a _Weak_ handle
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
///
/// Each asset is mapped by a unique [`HandleId`], allowing any [`Handle`] with the same
/// [`HandleId`] to access it. These assets remain loaded for as long as a Strong handle to that
/// asset exists.
///
/// To store a reference to an asset without forcing it to stay loaded, you can use a Weak handle.
/// To make a Weak handle a Strong one, use [`Assets::get_handle`] or pass the `Assets` collection
/// into the handle's [`make_strong`](Handle::make_strong) method.
///
/// Remember, if there are no Strong handles for an asset (i.e. they have all been dropped), the
/// asset will unload. Make sure you always have a Strong handle when you want to keep an asset
/// loaded!
#[derive(Debug)]
pub struct Assets<T: Asset> {
    assets: HashMap<HandleId, T>,
    aliases: HashMap<HandleId, HashSet<HandleId>>,
    alias_to_handle: HashMap<HandleId, HandleId>,
    events: Events<AssetEvent<T>>,
    pub(crate) ref_change_sender: Sender<RefChange>,
}

impl<T: Asset> Assets<T> {
    pub(crate) fn new(ref_change_sender: Sender<RefChange>) -> Self {
        Assets {
            assets: HashMap::default(),
            aliases: HashMap::default(),
            alias_to_handle: HashMap::default(),
            events: Events::default(),
            ref_change_sender,
        }
    }

    /// Add an alias from a strong handle obtained from `AssetServer::load` to another handle,
    /// for example a `const` `HandleUntyped`.
    pub fn add_alias<H: Into<HandleId>, A: Into<HandleId>>(&mut self, handle: H, alias: A) {
        let handle_id = handle.into();
        let alias_handle_id = alias.into();
        self.aliases
            .entry(handle_id)
            .or_default()
            .insert(alias_handle_id);
        self.alias_to_handle.insert(alias_handle_id, handle_id);
    }

    /// Remove an alias from a strong handle obtained from `AssetServer::load` to another handle,
    /// for example a `const` `HandleUntyped`.
    pub fn remove_alias<H: Into<HandleId>, A: Into<HandleId>>(&mut self, handle: H, alias: A) {
        let handle_id = handle.into();
        let alias_handle_id = alias.into();
        self.aliases
            .entry(handle_id)
            .or_default()
            .remove(&alias_handle_id);
        self.alias_to_handle.remove(&alias_handle_id);
    }

    /// Get the strong handle obtained from `AssetServer::load` from an alias handle.
    pub fn get_handle_from_alias<A: Into<HandleId>>(&self, alias: A) -> Option<&HandleId> {
        self.alias_to_handle.get(&alias.into())
    }

    fn broadcast_event<H: Into<HandleId>>(&mut self, asset_event_type: AssetEventType, handle: H) {
        let make_event = match asset_event_type {
            AssetEventType::Created => |id| AssetEvent::Created {
                handle: Handle::weak(id),
            },
            AssetEventType::Modified => |id| AssetEvent::Modified {
                handle: Handle::weak(id),
            },
            AssetEventType::Removed => |id| AssetEvent::Removed {
                handle: Handle::weak(id),
            },
        };
        let handle_id = handle.into();
        self.events.send(make_event(handle_id));
        if let Some(aliases) = self.aliases.get(&handle_id) {
            for alias in aliases {
                self.events.send(make_event(*alias));
            }
        }
    }

    /// Adds an asset to the collection, returning a Strong handle to that asset.
    ///
    /// # Events
    /// * [`AssetEvent::Created`]
    pub fn add(&mut self, asset: T) -> Handle<T> {
        let id = HandleId::random::<T>();
        self.assets.insert(id, asset);
        self.broadcast_event(AssetEventType::Created, id);
        self.get_handle(id)
    }

    /// Add/modify the asset pointed to by the given handle.
    ///
    /// Unless there exists another Strong handle for this asset, it's advised to use the returned
    /// Strong handle. Not doing so may result in the unexpected release of the asset.
    ///
    /// See [`set_untracked`](Assets::set_untracked) for more info.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn set<H: Into<HandleId>>(&mut self, handle: H, asset: T) -> Handle<T> {
        let id: HandleId = handle.into();
        self.set_untracked(id, asset);
        self.get_handle(id)
    }

    /// Add/modify the asset pointed to by the given handle.
    ///
    /// If an asset already exists with the given [`HandleId`], it will be modified. Otherwise the
    /// new asset will be inserted.
    ///
    /// # Events
    /// * [`AssetEvent::Created`]: Sent if the asset did not yet exist with the given handle
    /// * [`AssetEvent::Modified`]: Sent if the asset with given handle already existed
    pub fn set_untracked<H: Into<HandleId>>(&mut self, handle: H, asset: T) {
        let id: HandleId = handle.into();
        if self.assets.insert(id, asset).is_some() {
            self.broadcast_event(AssetEventType::Modified, id);
        } else {
            self.broadcast_event(AssetEventType::Created, id);
        }
    }

    /// Get the asset for the given handle.
    ///
    /// This is the main method for accessing asset data from an [Assets] collection. If you need
    /// mutable access to the asset, use [`get_mut`](Assets::get_mut).
    pub fn get<H: Into<HandleId>>(&self, handle: H) -> Option<&T> {
        let handle_id = handle.into();
        self.assets.get(&handle_id).or_else(|| {
            self.get_handle_from_alias(handle_id)
                .and_then(|handle_id| self.assets.get(handle_id))
        })
    }

    /// Checks if an asset exists for the given handle
    pub fn contains<H: Into<HandleId>>(&self, handle: H) -> bool {
        self.assets.contains_key(&handle.into())
    }

    /// Get mutable access to the asset for the given handle.
    ///
    /// This is the main method for mutably accessing asset data from an [Assets] collection. If you
    /// do not need mutable access to the asset, you may also use [get](Assets::get).
    pub fn get_mut<H: Into<HandleId>>(&mut self, handle: H) -> Option<&mut T> {
        let id: HandleId = handle.into();
        self.broadcast_event(AssetEventType::Modified, id);
        self.assets.get_mut(&id)
    }

    /// Gets a _Strong_ handle pointing to the same asset as the given one
    pub fn get_handle<H: Into<HandleId>>(&self, handle: H) -> Handle<T> {
        Handle::strong(handle.into(), self.ref_change_sender.clone())
    }

    /// Get mutable access to an asset for the given handle, inserting a new value if none exists.
    ///
    /// # Events
    /// * [`AssetEvent::Created`]: Sent if the asset did not yet exist with the given handle
    pub fn get_or_insert_with<H: Into<HandleId>>(
        &mut self,
        handle: H,
        insert_fn: impl FnOnce() -> T,
    ) -> &mut T {
        let id: HandleId = handle.into();
        if !self.assets.contains_key(&id) {
            self.broadcast_event(AssetEventType::Created, id);
        }
        self.assets.entry(id).or_insert_with(insert_fn)
    }

    /// Get an iterator over all assets in the collection.
    pub fn iter(&self) -> impl Iterator<Item = (HandleId, &T)> {
        self.assets.iter().map(|(k, v)| (*k, v))
    }

    /// Get a mutable iterator over all assets in the collection.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (HandleId, &mut T)> {
        let mut keys = self.assets.keys().cloned().collect::<Vec<_>>();
        for id in keys.drain(..) {
            self.broadcast_event(AssetEventType::Modified, id);
        }
        self.assets.iter_mut().map(|(k, v)| (*k, v))
    }

    /// Get an iterator over all [`HandleId`]'s in the collection.
    pub fn ids(&self) -> impl Iterator<Item = HandleId> + '_ {
        self.assets.keys().cloned()
    }

    /// Remove an asset for the given handle.
    ///
    /// The asset is returned if it existed in the collection, otherwise `None`.
    ///
    /// # Events
    /// * [`AssetEvent::Removed`]
    pub fn remove<H: Into<HandleId>>(&mut self, handle: H) -> Option<T> {
        let id: HandleId = handle.into();
        let asset = self.assets.remove(&id);
        if asset.is_some() {
            self.broadcast_event(AssetEventType::Removed, id);
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
        mut events: EventWriter<AssetEvent<T>>,
        mut assets: ResMut<Assets<T>>,
    ) {
        // Check if the events are empty before calling `drain`.
        // As `drain` triggers change detection.
        if !assets.events.is_empty() {
            events.send_batch(assets.events.drain())
        }
    }

    /// Gets the number of assets in the collection
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// Returns true if there are no stored assets
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

/// [App] extension methods for adding new asset types
pub trait AddAsset {
    fn add_asset<T>(&mut self) -> &mut Self
    where
        T: Asset;
    fn init_asset_loader<T>(&mut self) -> &mut Self
    where
        T: AssetLoader + FromWorld;
    fn add_asset_loader<T>(&mut self, loader: T) -> &mut Self
    where
        T: AssetLoader;
}

impl AddAsset for App {
    /// Add an [`Asset`] to the [`App`].
    ///
    /// Adding the same [`Asset`] again after it has been added does nothing.
    fn add_asset<T>(&mut self) -> &mut Self
    where
        T: Asset,
    {
        if self.world.contains_resource::<Assets<T>>() {
            return self;
        }
        let assets = {
            let asset_server = self.world.get_resource::<AssetServer>().unwrap();
            asset_server.register_asset_type::<T>()
        };

        self.insert_resource(assets)
            .add_system_to_stage(AssetStage::AssetEvents, Assets::<T>::asset_event_system)
            .add_system_to_stage(AssetStage::LoadAssets, update_asset_storage_system::<T>)
            .register_type::<Handle<T>>()
            .add_event::<AssetEvent<T>>()
    }

    fn init_asset_loader<T>(&mut self) -> &mut Self
    where
        T: AssetLoader + FromWorld,
    {
        let result = T::from_world(&mut self.world);
        self.add_asset_loader(result)
    }

    fn add_asset_loader<T>(&mut self, loader: T) -> &mut Self
    where
        T: AssetLoader,
    {
        self.world
            .get_resource_mut::<AssetServer>()
            .expect("AssetServer does not exist. Consider adding it as a resource.")
            .add_loader(loader);
        self
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::App;

    use crate::{AddAsset, Assets};

    #[test]
    fn asset_overwriting() {
        #[derive(bevy_reflect::TypeUuid)]
        #[uuid = "44115972-f31b-46e5-be5c-2b9aece6a52f"]
        struct MyAsset;
        let mut app = App::new();
        app.add_plugin(bevy_core::CorePlugin)
            .add_plugin(crate::AssetPlugin);
        app.add_asset::<MyAsset>();
        let mut assets_before = app.world.get_resource_mut::<Assets<MyAsset>>().unwrap();
        let handle = assets_before.add(MyAsset);
        app.add_asset::<MyAsset>(); // Ensure this doesn't overwrite the Asset
        let assets_after = app.world.get_resource_mut::<Assets<MyAsset>>().unwrap();
        assert!(assets_after.get(handle).is_some())
    }
}
