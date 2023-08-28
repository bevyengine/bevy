use crate::{
    path::AssetPath, AssetIo, AssetIoError, AssetMeta, AssetServer, Assets, Handle, HandleId,
    HandleUntyped, RefChangeChannel,
};
use anyhow::Error;
use anyhow::Result;
use bevy_ecs::system::{Res, ResMut};
use bevy_reflect::TypePath;
use bevy_reflect::{TypeUuid, TypeUuidDynamic};
use bevy_utils::{BoxedFuture, HashMap};
use crossbeam_channel::{Receiver, Sender};
use downcast_rs::{impl_downcast, Downcast};
use std::path::Path;

/// A loader for an asset source.
///
/// Types implementing this trait are used by the [`AssetServer`] to load assets
/// into their respective asset storages.
pub trait AssetLoader: Send + Sync + 'static {
    /// Processes the asset in an asynchronous closure.
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), Error>>;

    /// Returns a list of extensions supported by this asset loader, without the preceding dot.
    fn extensions(&self) -> &[&str];
}

/// An essential piece of data of an application.
///
/// Assets are the building blocks of games. They can be anything, from images and sounds to scenes
/// and scripts. In Bevy, an asset is any struct that has an unique type id, as shown below:
///
/// ```rust
/// use bevy_reflect::{TypePath, TypeUuid};
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize, TypeUuid, TypePath)]
/// #[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
/// pub struct CustomAsset {
///     pub value: i32,
/// }
/// # fn is_asset<T: bevy_asset::Asset>() {}
/// # is_asset::<CustomAsset>();
/// ```
///
/// See the `assets/custom_asset.rs` example in the repository for more details.
///
/// In order to load assets into your game you must either add them manually to an asset storage
/// with [`Assets::add`] or load them from the filesystem with [`AssetServer::load`].
pub trait Asset: TypeUuid + TypePath + AssetDynamic {}

/// An untyped version of the [`Asset`] trait.
pub trait AssetDynamic: Downcast + TypeUuidDynamic + Send + Sync + 'static {}
impl_downcast!(AssetDynamic);

impl<T> Asset for T where T: TypeUuid + TypePath + AssetDynamic + TypeUuidDynamic {}

impl<T> AssetDynamic for T where T: Send + Sync + 'static + TypeUuidDynamic {}

/// A complete asset processed in an [`AssetLoader`].
pub struct LoadedAsset<T: Asset> {
    pub(crate) value: Option<T>,
    pub(crate) dependencies: Vec<AssetPath<'static>>,
}

impl<T: Asset> LoadedAsset<T> {
    /// Creates a new loaded asset.
    pub fn new(value: T) -> Self {
        Self {
            value: Some(value),
            dependencies: Vec::new(),
        }
    }

    /// Adds a dependency on another asset at the provided path.
    pub fn add_dependency(&mut self, asset_path: AssetPath) {
        self.dependencies.push(asset_path.to_owned());
    }

    /// Adds a dependency on another asset at the provided path.
    #[must_use]
    pub fn with_dependency(mut self, asset_path: AssetPath) -> Self {
        self.add_dependency(asset_path);
        self
    }

    /// Adds dependencies on other assets at the provided paths.
    #[must_use]
    pub fn with_dependencies(mut self, asset_paths: Vec<AssetPath<'static>>) -> Self {
        for asset_path in asset_paths {
            self.add_dependency(asset_path);
        }
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

/// An asynchronous context where an [`Asset`] is processed.
///
/// The load context is created by the [`AssetServer`] to process an asset source after loading its
/// contents into memory. It is then passed to the appropriate [`AssetLoader`] based on the file
/// extension of the asset's path.
///
/// An asset source can define one or more assets from a single source path. The main asset is set
/// using [`LoadContext::set_default_asset`] and sub-assets are defined with
/// [`LoadContext::set_labeled_asset`].
pub struct LoadContext<'a> {
    pub(crate) ref_change_channel: &'a RefChangeChannel,
    pub(crate) asset_io: &'a dyn AssetIo,
    pub(crate) labeled_assets: HashMap<Option<String>, BoxedLoadedAsset>,
    pub(crate) path: &'a Path,
    pub(crate) version: usize,
}

impl<'a> LoadContext<'a> {
    pub(crate) fn new(
        path: &'a Path,
        ref_change_channel: &'a RefChangeChannel,
        asset_io: &'a dyn AssetIo,
        version: usize,
    ) -> Self {
        Self {
            ref_change_channel,
            asset_io,
            labeled_assets: Default::default(),
            version,
            path,
        }
    }

    /// Gets the source path for this load context.
    pub fn path(&self) -> &Path {
        self.path
    }

    /// Returns `true` if the load context contains an asset with the specified label.
    pub fn has_labeled_asset(&self, label: &str) -> bool {
        self.labeled_assets.contains_key(&Some(label.to_string()))
    }

    /// Sets the primary asset loaded from the asset source.
    pub fn set_default_asset<T: Asset>(&mut self, asset: LoadedAsset<T>) {
        self.labeled_assets.insert(None, asset.into());
    }

    /// Sets a secondary asset loaded from the asset source.
    pub fn set_labeled_asset<T: Asset>(&mut self, label: &str, asset: LoadedAsset<T>) -> Handle<T> {
        assert!(!label.is_empty());
        self.labeled_assets
            .insert(Some(label.to_string()), asset.into());
        self.get_handle(AssetPath::new_ref(self.path(), Some(label)))
    }

    /// Gets a strong handle to an asset of type `T` from its id.
    pub fn get_handle<I: Into<HandleId>, T: Asset>(&self, id: I) -> Handle<T> {
        Handle::strong(id.into(), self.ref_change_channel.sender.clone())
    }

    /// Gets an untyped strong handle for an asset with the provided id.
    pub fn get_handle_untyped<I: Into<HandleId>>(&self, id: I) -> HandleUntyped {
        HandleUntyped::strong(id.into(), self.ref_change_channel.sender.clone())
    }

    /// Reads the contents of the file at the specified path through the [`AssetIo`] associated
    /// with this context.
    pub async fn read_asset_bytes<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>, AssetIoError> {
        self.asset_io
            .watch_path_for_changes(path.as_ref(), Some(self.path.to_owned()))?;
        self.asset_io.load_path(path.as_ref()).await
    }

    /// Generates metadata for the assets managed by this load context.
    pub fn get_asset_metas(&self) -> Vec<AssetMeta> {
        let mut asset_metas = Vec::new();
        for (label, asset) in &self.labeled_assets {
            asset_metas.push(AssetMeta {
                dependencies: asset.dependencies.clone(),
                label: label.clone(),
                type_uuid: asset.value.as_ref().unwrap().type_uuid(),
            });
        }
        asset_metas
    }

    /// Gets the asset I/O associated with this load context.
    pub fn asset_io(&self) -> &dyn AssetIo {
        self.asset_io
    }
}

/// The result of loading an asset of type `T`.
#[derive(Debug)]
pub struct AssetResult<T> {
    /// The asset itself.
    pub asset: Box<T>,
    /// The unique id of the asset.
    pub id: HandleId,
    /// Change version.
    pub version: usize,
}

/// An event channel used by asset server to update the asset storage of a `T` asset.
#[derive(Debug)]
pub struct AssetLifecycleChannel<T> {
    /// The sender endpoint of the channel.
    pub sender: Sender<AssetLifecycleEvent<T>>,
    /// The receiver endpoint of the channel.
    pub receiver: Receiver<AssetLifecycleEvent<T>>,
}

/// Events for the [`AssetLifecycleChannel`].
pub enum AssetLifecycleEvent<T> {
    /// An asset was created.
    Create(AssetResult<T>),
    /// An asset was freed.
    Free(HandleId),
}

/// A trait for sending lifecycle notifications from assets in the asset server.
pub trait AssetLifecycle: Downcast + Send + Sync + 'static {
    /// Notifies the asset server that a new asset was created.
    fn create_asset(&self, id: HandleId, asset: Box<dyn AssetDynamic>, version: usize);
    /// Notifies the asset server that an asset was freed.
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
                .unwrap();
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

impl<T> Default for AssetLifecycleChannel<T> {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        AssetLifecycleChannel { sender, receiver }
    }
}

/// Updates the [`Assets`] collection according to the changes queued up by [`AssetServer`].
pub fn update_asset_storage_system<T: Asset + AssetDynamic>(
    asset_server: Res<AssetServer>,
    assets: ResMut<Assets<T>>,
) {
    asset_server.update_asset_storage(assets);
}
