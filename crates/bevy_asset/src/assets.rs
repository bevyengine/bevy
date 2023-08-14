use crate::{
    update_asset_storage_system, Asset, AssetEvents, AssetLoader, AssetServer, Handle, HandleId,
    LoadAssets, RefChange, ReflectAsset, ReflectHandle,
};
use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_reflect::{FromReflect, GetTypeRegistration, Reflect};
use bevy_utils::HashMap;
use crossbeam_channel::Sender;
use std::fmt::Debug;

/// Events that involve assets of type `T`.
///
/// Events sent via the [`Assets`] struct will always be sent with a _Weak_ handle, because the
/// asset may not exist by the time the event is handled.
#[derive(Event)]
pub enum AssetEvent<T: Asset> {
    #[allow(missing_docs)]
    Created { handle: Handle<T> },
    #[allow(missing_docs)]
    Modified { handle: Handle<T> },
    #[allow(missing_docs)]
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
                .field("handle", &handle.id())
                .finish(),
            AssetEvent::Modified { handle } => f
                .debug_struct(&format!(
                    "AssetEvent<{}>::Modified",
                    std::any::type_name::<T>()
                ))
                .field("handle", &handle.id())
                .finish(),
            AssetEvent::Removed { handle } => f
                .debug_struct(&format!(
                    "AssetEvent<{}>::Removed",
                    std::any::type_name::<T>()
                ))
                .field("handle", &handle.id())
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
#[derive(Debug, Resource)]
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

    /// Adds an asset to the collection, returning a Strong handle to that asset.
    ///
    /// # Events
    ///
    /// * [`AssetEvent::Created`]
    pub fn add(&mut self, asset: T) -> Handle<T> {
        let id = HandleId::random::<T>();
        self.assets.insert(id, asset);
        self.events.send(AssetEvent::Created {
            handle: Handle::weak(id),
        });
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
    ///
    /// * [`AssetEvent::Created`]: Sent if the asset did not yet exist with the given handle.
    /// * [`AssetEvent::Modified`]: Sent if the asset with given handle already existed.
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

    /// Gets the asset for the given handle.
    ///
    /// This is the main method for accessing asset data from an [Assets] collection. If you need
    /// mutable access to the asset, use [`get_mut`](Assets::get_mut).
    pub fn get(&self, handle: &Handle<T>) -> Option<&T> {
        self.assets.get::<HandleId>(&handle.into())
    }

    /// Checks if an asset exists for the given handle
    pub fn contains(&self, handle: &Handle<T>) -> bool {
        self.assets.contains_key::<HandleId>(&handle.into())
    }

    /// Get mutable access to the asset for the given handle.
    ///
    /// This is the main method for mutably accessing asset data from an [Assets] collection. If you
    /// do not need mutable access to the asset, you may also use [get](Assets::get).
    pub fn get_mut(&mut self, handle: &Handle<T>) -> Option<&mut T> {
        let id: HandleId = handle.into();
        self.events.send(AssetEvent::Modified {
            handle: Handle::weak(id),
        });
        self.assets.get_mut(&id)
    }

    /// Gets a _Strong_ handle pointing to the same asset as the given one.
    pub fn get_handle<H: Into<HandleId>>(&self, handle: H) -> Handle<T> {
        Handle::strong(handle.into(), self.ref_change_sender.clone())
    }

    /// Gets mutable access to an asset for the given handle, inserting a new value if none exists.
    ///
    /// # Events
    ///
    /// * [`AssetEvent::Created`]: Sent if the asset did not yet exist with the given handle.
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

    /// Gets an iterator over all assets in the collection.
    pub fn iter(&self) -> impl Iterator<Item = (HandleId, &T)> {
        self.assets.iter().map(|(k, v)| (*k, v))
    }

    /// Gets a mutable iterator over all assets in the collection.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (HandleId, &mut T)> {
        self.assets.iter_mut().map(|(k, v)| {
            self.events.send(AssetEvent::Modified {
                handle: Handle::weak(*k),
            });
            (*k, v)
        })
    }

    /// Gets an iterator over all [`HandleId`]'s in the collection.
    pub fn ids(&self) -> impl Iterator<Item = HandleId> + '_ {
        self.assets.keys().cloned()
    }

    /// Removes an asset for the given handle.
    ///
    /// The asset is returned if it existed in the collection, otherwise `None`.
    ///
    /// # Events
    ///
    /// * [`AssetEvent::Removed`]
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
        self.assets.clear();
    }

    /// Reserves capacity for at least additional more elements to be inserted into the assets.
    ///
    /// The collection may reserve more space to avoid frequent reallocations.
    pub fn reserve(&mut self, additional: usize) {
        self.assets.reserve(additional);
    }

    /// Shrinks the capacity of the asset map as much as possible.
    ///
    /// It will drop down as much as possible while maintaining the internal rules and possibly
    /// leaving some space in accordance with the resize policy.
    pub fn shrink_to_fit(&mut self) {
        self.assets.shrink_to_fit();
    }

    /// A system that creates [`AssetEvent`]s at the end of the frame based on changes in the
    /// asset storage.
    pub fn asset_event_system(
        mut events: EventWriter<AssetEvent<T>>,
        mut assets: ResMut<Assets<T>>,
    ) {
        // Check if the events are empty before calling `drain`.
        // As `drain` triggers change detection.
        if !assets.events.is_empty() {
            events.send_batch(assets.events.drain());
        }
    }

    /// Gets the number of assets in the collection.
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// Returns `true` if there are no stored assets.
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

/// [`App`] extension methods for adding new asset types.
pub trait AddAsset {
    /// Registers `T` as a supported asset in the application.
    ///
    /// Adding the same type again after it has been added does nothing.
    fn add_asset<T>(&mut self) -> &mut Self
    where
        T: Asset;

    /// Registers the asset type `T` using [`App::register_type`],
    /// and adds [`ReflectAsset`] type data to `T` and [`ReflectHandle`] type data to [`Handle<T>`] in the type registry.
    ///
    /// This enables reflection code to access assets. For detailed information, see the docs on [`ReflectAsset`] and [`ReflectHandle`].
    fn register_asset_reflect<T>(&mut self) -> &mut Self
    where
        T: Asset + Reflect + FromReflect + GetTypeRegistration;

    /// Registers `T` as a supported internal asset in the application.
    ///
    /// Internal assets (e.g. shaders) are bundled directly into the app and can't be hot reloaded
    /// using the conventional API. See `DebugAssetServerPlugin`.
    ///
    /// Adding the same type again after it has been added does nothing.
    fn add_debug_asset<T: Clone>(&mut self) -> &mut Self
    where
        T: Asset;

    /// Adds an asset loader `T` using default values.
    ///
    /// The default values may come from the [`World`] or from `T::default()`.
    fn init_asset_loader<T>(&mut self) -> &mut Self
    where
        T: AssetLoader + FromWorld;

    /// Adds an asset loader `T` for internal assets using default values.
    ///
    /// Internal assets (e.g. shaders) are bundled directly into the app and can't be hot reloaded
    /// using the conventional API. See `DebugAssetServerPlugin`.
    ///
    /// The default values may come from the [`World`] or from `T::default()`.
    fn init_debug_asset_loader<T>(&mut self) -> &mut Self
    where
        T: AssetLoader + FromWorld;

    /// Adds the provided asset loader to the application.
    fn add_asset_loader<T>(&mut self, loader: T) -> &mut Self
    where
        T: AssetLoader;

    /// Preregisters a loader for the given extensions, that will block asset loads until a real loader
    /// is registered.
    fn preregister_asset_loader(&mut self, extensions: &[&str]) -> &mut Self;
}

impl AddAsset for App {
    fn add_asset<T>(&mut self) -> &mut Self
    where
        T: Asset,
    {
        if self.world.contains_resource::<Assets<T>>() {
            return self;
        }
        let assets = {
            let asset_server = self.world.resource::<AssetServer>();
            asset_server.register_asset_type::<T>()
        };

        self.insert_resource(assets)
            .add_systems(LoadAssets, update_asset_storage_system::<T>)
            .add_systems(AssetEvents, Assets::<T>::asset_event_system)
            .register_type::<Handle<T>>()
            .add_event::<AssetEvent<T>>()
    }

    fn register_asset_reflect<T>(&mut self) -> &mut Self
    where
        T: Asset + Reflect + FromReflect + GetTypeRegistration,
    {
        let type_registry = self.world.resource::<AppTypeRegistry>();
        {
            let mut type_registry = type_registry.write();

            type_registry.register::<T>();
            type_registry.register::<Handle<T>>();
            type_registry.register_type_data::<T, ReflectAsset>();
            type_registry.register_type_data::<Handle<T>, ReflectHandle>();
        }

        self
    }

    fn add_debug_asset<T: Clone>(&mut self) -> &mut Self
    where
        T: Asset,
    {
        #[cfg(feature = "debug_asset_server")]
        {
            self.add_systems(
                bevy_app::Update,
                crate::debug_asset_server::sync_debug_assets::<T>,
            );
            let mut app = self
                .world
                .non_send_resource_mut::<crate::debug_asset_server::DebugAssetApp>();
            app.add_asset::<T>()
                .init_resource::<crate::debug_asset_server::HandleMap<T>>();
        }
        self
    }

    fn init_asset_loader<T>(&mut self) -> &mut Self
    where
        T: AssetLoader + FromWorld,
    {
        let result = T::from_world(&mut self.world);
        self.add_asset_loader(result)
    }

    fn init_debug_asset_loader<T>(&mut self) -> &mut Self
    where
        T: AssetLoader + FromWorld,
    {
        #[cfg(feature = "debug_asset_server")]
        {
            let mut app = self
                .world
                .non_send_resource_mut::<crate::debug_asset_server::DebugAssetApp>();
            app.init_asset_loader::<T>();
        }
        self
    }

    fn add_asset_loader<T>(&mut self, loader: T) -> &mut Self
    where
        T: AssetLoader,
    {
        self.world.resource_mut::<AssetServer>().add_loader(loader);
        self
    }

    fn preregister_asset_loader(&mut self, extensions: &[&str]) -> &mut Self {
        self.world
            .resource_mut::<AssetServer>()
            .preregister_loader(extensions);
        self
    }
}

/// Loads an internal asset from a project source file.
/// the file and its path are passed to the loader function, together with any additional parameters.
/// the resulting asset is stored in the app's asset server.
///
/// Internal assets (e.g. shaders) are bundled directly into the app and can't be hot reloaded
/// using the conventional API. See [`DebugAssetServerPlugin`](crate::debug_asset_server::DebugAssetServerPlugin).
#[cfg(feature = "debug_asset_server")]
#[macro_export]
macro_rules! load_internal_asset {
    ($app: ident, $handle: ident, $path_str: expr, $loader: expr) => {{
        {
            let mut debug_app = $app
                .world
                .non_send_resource_mut::<$crate::debug_asset_server::DebugAssetApp>();
            $crate::debug_asset_server::register_handle_with_loader::<_, &'static str>(
                $loader,
                &mut debug_app,
                $handle,
                file!(),
                $path_str,
            );
        }
        let mut assets = $app.world.resource_mut::<$crate::Assets<_>>();
        assets.set_untracked(
            $handle,
            ($loader)(
                include_str!($path_str),
                std::path::Path::new(file!())
                .parent()
                .unwrap()
                .join($path_str)
                .to_string_lossy(),
            )
        );
    }};
    // we can't support params without variadic arguments, so internal assets with additional params can't be hot-reloaded
    ($app: ident, $handle: ident, $path_str: expr, $loader: expr $(, $param:expr)+) => {{
        let mut assets = $app.world.resource_mut::<$crate::Assets<_>>();
        assets.set_untracked(
            $handle,
            ($loader)(
                include_str!($path_str),
                std::path::Path::new(file!())
                    .parent()
                    .unwrap()
                    .join($path_str)
                    .to_string_lossy(),
                $($param),+
            ),
        );
    }};
}

/// Loads an internal asset from a project source file.
/// the file and its path are passed to the loader function, together with any additional parameters.
/// the resulting asset is stored in the app's asset server.
///
/// Internal assets (e.g. shaders) are bundled directly into the app and can't be hot reloaded
/// using the conventional API. See `DebugAssetServerPlugin`.
#[cfg(not(feature = "debug_asset_server"))]
#[macro_export]
macro_rules! load_internal_asset {
    ($app: ident, $handle: ident, $path_str: expr, $loader: expr $(, $param:expr)*) => {{
        let mut assets = $app.world.resource_mut::<$crate::Assets<_>>();
        assets.set_untracked(
            $handle,
            ($loader)(
                include_str!($path_str),
                std::path::Path::new(file!())
                    .parent()
                    .unwrap()
                    .join($path_str)
                    .to_string_lossy(),
                $($param),*
            ),
        );
    }};
}

/// Loads an internal binary asset.
///
/// Internal binary assets (e.g. spir-v shaders) are bundled directly into the app and can't be hot reloaded
/// using the conventional API. See `DebugAssetServerPlugin`.
#[cfg(feature = "debug_asset_server")]
#[macro_export]
macro_rules! load_internal_binary_asset {
    ($app: ident, $handle: ident, $path_str: expr, $loader: expr) => {{
        {
            let mut debug_app = $app
                .world
                .non_send_resource_mut::<$crate::debug_asset_server::DebugAssetApp>();
            $crate::debug_asset_server::register_handle_with_loader::<_, &'static [u8]>(
                $loader,
                &mut debug_app,
                $handle,
                file!(),
                $path_str,
            );
        }
        let mut assets = $app.world.resource_mut::<$crate::Assets<_>>();
        assets.set_untracked(
            $handle,
            ($loader)(
                include_bytes!($path_str).as_ref(),
                std::path::Path::new(file!())
                    .parent()
                    .unwrap()
                    .join($path_str)
                    .to_string_lossy()
                    .into(),
            ),
        );
    }};
}

/// Loads an internal binary asset.
///
/// Internal binary assets (e.g. spir-v shaders) are bundled directly into the app and can't be hot reloaded
/// using the conventional API. See `DebugAssetServerPlugin`.
#[cfg(not(feature = "debug_asset_server"))]
#[macro_export]
macro_rules! load_internal_binary_asset {
    ($app: ident, $handle: ident, $path_str: expr, $loader: expr) => {{
        let mut assets = $app.world.resource_mut::<$crate::Assets<_>>();
        assets.set_untracked(
            $handle,
            ($loader)(
                include_bytes!($path_str).as_ref(),
                std::path::Path::new(file!())
                    .parent()
                    .unwrap()
                    .join($path_str)
                    .to_string_lossy()
                    .into(),
            ),
        );
    }};
}

#[cfg(test)]
mod tests {
    use bevy_app::App;

    use crate::{AddAsset, Assets};

    #[test]
    fn asset_overwriting() {
        #[derive(bevy_reflect::TypeUuid, bevy_reflect::TypePath)]
        #[uuid = "44115972-f31b-46e5-be5c-2b9aece6a52f"]
        struct MyAsset;
        let mut app = App::new();
        app.add_plugins((
            bevy_core::TaskPoolPlugin::default(),
            bevy_core::TypeRegistrationPlugin::default(),
            crate::AssetPlugin::default(),
        ));
        app.add_asset::<MyAsset>();
        let mut assets_before = app.world.resource_mut::<Assets<MyAsset>>();
        let handle = assets_before.add(MyAsset);
        app.add_asset::<MyAsset>(); // Ensure this doesn't overwrite the Asset
        let assets_after = app.world.resource_mut::<Assets<MyAsset>>();
        assert!(assets_after.get(&handle).is_some());
    }
}
