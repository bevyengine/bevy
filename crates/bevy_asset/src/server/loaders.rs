use crate::{loader::ErasedAssetLoader, AssetLoader};
use bevy_log::warn;
use bevy_tasks::IoTaskPool;
use bevy_utils::HashMap;
use futures_lite::Future;
use std::{any::TypeId, fmt::Debug, sync::Arc};

/// Storage for [`AssetLoader`]'s, providing helper methods for efficient access.
#[derive(Debug, Default)]
pub struct AssetLoaders {
    type_id_to_loader: HashMap<TypeId, MaybeAssetLoader>,
    extension_to_type_id: HashMap<String, TypeId>,
    type_name_to_type_id: HashMap<&'static str, TypeId>,
    preregistered_loaders: HashMap<&'static str, TypeId>,
}

/// Named type to ensure all APIs for `AssetLoaders` stay consistent
type AssetLoaderSmartPointer = Arc<dyn ErasedAssetLoader>;

#[derive(Clone)]
enum MaybeAssetLoader {
    Ready(AssetLoaderSmartPointer),
    Pending {
        sender: async_broadcast::Sender<AssetLoaderSmartPointer>,
        receiver: async_broadcast::Receiver<AssetLoaderSmartPointer>,
    },
}

impl Debug for MaybeAssetLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready(loader) => f.debug_tuple("Ready").field(&loader.type_name()).finish(),
            Self::Pending { .. } => f.debug_struct("Pending").finish(),
        }
    }
}

impl AssetLoaders {
    /// Get an [`AssetLoader`] by the [`TypeId`] of the [`Asset`](`crate::Asset`) it can load. This
    /// resolves to `None` immediately if no loader has been registered for this [`Asset`](`crate::Asset`) type.
    ///
    /// If a loader has been pre-registered for this type, then it will resolve once it is available.
    pub fn get_by_asset_type_id(
        &self,
        type_id: TypeId,
    ) -> impl Future<Output = Option<AssetLoaderSmartPointer>> + 'static {
        let loader = self.type_id_to_loader.get(&type_id).cloned();

        async {
            match loader? {
                MaybeAssetLoader::Ready(loader) => Some(loader),
                MaybeAssetLoader::Pending { mut receiver, .. } => {
                    Some(receiver.recv().await.unwrap())
                }
            }
        }
    }

    /// Get an [`AssetLoader`] by its [`std::any::type_name`]. This resolves to `None` immediately if
    /// no loader has been registered by that name.
    ///
    /// If a loader has been pre-registered for this name, then it will resolve once it is available.
    pub fn get_by_loader_type_name(
        &self,
        name: &str,
    ) -> impl Future<Output = Option<AssetLoaderSmartPointer>> + 'static {
        let type_id = self.type_name_to_type_id.get(name).copied();
        let loader = type_id.map(|type_id| self.get_by_asset_type_id(type_id));

        async { loader?.await }
    }

    /// Get an [`AssetLoader`] by a file extension it supports. This resolves to `None` immediately if
    /// no loader has been registered for that extension.
    ///
    /// If a loader has been pre-registered for this extension, then it will resolve once it is available.
    pub fn get_by_extension(
        &self,
        extension: &str,
    ) -> impl Future<Output = Option<AssetLoaderSmartPointer>> + 'static {
        let type_id = self.extension_to_type_id.get(extension).copied();
        let loader = type_id.map(|type_id| self.get_by_asset_type_id(type_id));

        async { loader?.await }
    }

    /// Registers a new [`AssetLoader`]. [`AssetLoader`]s must be registered before they can be used.
    pub fn register_loader<L: AssetLoader>(&mut self, loader: L) {
        let type_name = std::any::type_name::<L>();
        let loader = Arc::new(loader);

        let (type_id, is_new) = if let Some(index) = self.preregistered_loaders.remove(type_name) {
            (index, false)
        } else {
            (TypeId::of::<L::Asset>(), true)
        };

        for extension in loader.extensions() {
            self.extension_to_type_id
                .insert(extension.to_string(), type_id);
        }

        if is_new {
            self.type_name_to_type_id.insert(type_name, type_id);
            self.type_id_to_loader
                .insert(type_id, MaybeAssetLoader::Ready(loader));
        } else {
            let maybe_loader = std::mem::replace(
                self.type_id_to_loader.get_mut(&type_id).unwrap(),
                MaybeAssetLoader::Ready(loader.clone()),
            );
            match maybe_loader {
                MaybeAssetLoader::Ready(_) => unreachable!(),
                MaybeAssetLoader::Pending { sender, .. } => {
                    IoTaskPool::get()
                        .spawn(async move {
                            let _ = sender.broadcast(loader).await;
                        })
                        .detach();
                }
            }
        }
    }

    /// Pre-register a loader that will later be added.
    ///
    /// Assets loaded with matching extensions will be blocked until the
    /// real loader is added.
    pub fn preregister_loader<L: AssetLoader>(&mut self, extensions: &[&str]) {
        let type_id = TypeId::of::<L::Asset>();
        let type_name = std::any::type_name::<L>();

        self.preregistered_loaders.insert(type_name, type_id);
        self.type_name_to_type_id.insert(type_name, type_id);

        for extension in extensions {
            if self
                .extension_to_type_id
                .insert(extension.to_string(), type_id)
                .is_some()
            {
                warn!("duplicate preregistration for `{extension}`, any assets loaded with the previous loader will never complete.");
            }
        }

        let (mut sender, receiver) = async_broadcast::broadcast(1);
        sender.set_overflow(true);

        let loader = MaybeAssetLoader::Pending { sender, receiver };

        self.type_id_to_loader.insert(type_id, loader);
    }
}
