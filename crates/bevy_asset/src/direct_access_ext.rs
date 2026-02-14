//! Add methods on `World` to simplify loading assets when all
//! you have is a `World`.

use core::{any::TypeId, marker::PhantomData};

use bevy_ecs::{
    system::{Commands, Res, SystemParam},
    world::{error::EntityMutableFetchError, DeferredWorld, World},
};
use thiserror::Error;
use tracing::warn;
use uuid::Uuid;

use crate::{
    insert_asset, meta::Settings, setup_asset, Asset, AssetData, AssetEntity,
    AssetEntityDoesNotExistError, AssetEvent, AssetHandleProvider, AssetId, AssetMut, AssetPath,
    AssetSelfHandle, AssetServer, AssetUuidMap, EntityHandle, Handle, ResolveUuidError,
    UntypedEntityHandle, UntypedHandle,
};

/// An extension trait for methods for working with assets directly from a [`World`].
pub trait DirectAssetAccessExt {
    /// Spawn a new asset and return the handle.
    #[must_use]
    fn spawn_asset<A: Asset>(&mut self, asset: A) -> Handle<A>;

    /// Spawns a new asset referenced by `uuid`.
    ///
    /// Calling this multiples times with the same `uuid` will replace the asset each time - so only
    /// the final asset will remain. Calling this does not free the asset if the handle is dropped:
    /// UUID assets are kept alive unless they are manually despawned or set to another handle with
    /// [`AssetUuidMap::set_uuid`].
    fn spawn_uuid_asset<A: Asset>(&mut self, uuid: Uuid, asset: A) -> Handle<A>;

    /// Attempts to insert the asset into the entity referenced by `id`.
    fn insert_asset<A: Asset>(
        &mut self,
        id: impl Into<AssetId<A>>,
        asset: A,
    ) -> Result<(), InsertAssetError>;

    /// Attempts to remove the asset from the entity referenced by `id`.
    ///
    /// This does not despawn the asset - this asset can still be inserted into.
    fn remove_asset<A: Asset>(&mut self, id: AssetId<A>) -> Result<A, RemoveAssetError>;

    /// Reserves a handle for an asset that can later be inserted.
    #[must_use]
    fn reserve_asset_handle<A: Asset>(&mut self) -> Handle<A>;

    /// Gets the asset data associated with the given `id`.
    fn get_asset<A: Asset>(&self, id: AssetId<A>) -> Option<&A>;

    /// Gets the asset data (mutably) associated with the given `id`.
    fn get_asset_mut<A: Asset>(&mut self, id: AssetId<A>) -> Option<AssetMut<'_, A>>;

    /// Gets the strong handle associated with an asset, ensuring its type matches `A`.
    fn get_asset_strong_handle<A: Asset>(&self, entity: AssetEntity) -> Option<Handle<A>>;

    /// Gets the strong handle associated with an asset.
    fn get_asset_strong_handle_untyped(&self, entity: AssetEntity) -> Option<UntypedHandle>;
}

pub trait AssetServerAccessExt {
    /// Load an asset similarly to [`AssetServer::load`].
    #[must_use]
    fn load_asset<'a, A: Asset>(&self, path: impl Into<AssetPath<'a>>) -> Handle<A>;

    /// Load an asset with settings, similarly to [`AssetServer::load_with_settings`].
    #[must_use]
    fn load_asset_with_settings<'a, A: Asset, S: Settings>(
        &self,
        path: impl Into<AssetPath<'a>>,
        settings: impl Fn(&mut S) + Send + Sync + 'static,
    ) -> Handle<A>;
}

impl DirectAssetAccessExt for World {
    fn spawn_asset<A: Asset>(&mut self, asset: A) -> Handle<A> {
        let handle = self.reserve_asset_handle();
        let entity = handle.entity().unwrap();
        insert_asset(self, entity, asset).unwrap();
        handle
    }

    fn spawn_uuid_asset<A: Asset>(&mut self, uuid: Uuid, asset: A) -> Handle<A> {
        let handle = self.spawn_asset(asset);
        // spawn_asset always returns a Strong variant, so unwrap here is safe.
        let entity_handle = UntypedEntityHandle::from(EntityHandle::try_from(handle).unwrap());
        self.resource_mut::<AssetUuidMap>()
            .set_uuid(uuid, entity_handle);
        // Send an asset event that this UUID has been added.
        // TODO: This isn't quite sufficient, since users can call set_uuid themselves. We should be
        // sending a message in that case as well. This also doesn't work correctly if the UUID
        // was already set.
        self.write_message(AssetEvent::<A>::Added {
            id: AssetId::Uuid { uuid },
        });
        Handle::Uuid(uuid, PhantomData)
    }

    fn insert_asset<A: Asset>(
        &mut self,
        id: impl Into<AssetId<A>>,
        asset: A,
    ) -> Result<(), InsertAssetError> {
        let entity = self.resource::<AssetUuidMap>().resolve_entity(id.into())?;
        insert_asset(self, entity, asset)?;
        Ok(())
    }

    fn remove_asset<A: Asset>(&mut self, id: AssetId<A>) -> Result<A, RemoveAssetError> {
        let entity = self.resource::<AssetUuidMap>().resolve_entity(id)?;
        let mut entity_mut = self.get_entity_mut(entity.raw_entity())?;
        let data = entity_mut
            .take::<AssetData<A>>()
            .ok_or(RemoveAssetError::AssetMissingData(entity))?;
        Ok(data.0)
    }

    fn reserve_asset_handle<A: Asset>(&mut self) -> Handle<A> {
        let entity = AssetEntity::new_unchecked(self.spawn_empty().id());
        let handle = self.resource::<AssetHandleProvider>().create_handle(
            entity,
            TypeId::of::<A>(),
            None,
            None,
        );
        setup_asset(self, &handle).unwrap();
        Handle::Strong(handle)
    }

    fn get_asset<A: Asset>(&self, id: AssetId<A>) -> Option<&A> {
        let entity = self.resource::<AssetUuidMap>().resolve_entity(id).ok()?;
        let entity = self.get_entity(entity.raw_entity()).ok()?;
        let data = entity.get::<AssetData<A>>()?;
        Some(&data.0)
    }

    fn get_asset_mut<A: Asset>(&mut self, id: AssetId<A>) -> Option<AssetMut<'_, A>> {
        let entity = self.resource::<AssetUuidMap>().resolve_entity(id).ok()?;
        let entity = self.get_entity_mut(entity.raw_entity()).ok()?;
        Some(AssetMut(entity.into_mut::<AssetData<A>>()?))
    }

    fn get_asset_strong_handle<A: Asset>(&self, entity: AssetEntity) -> Option<Handle<A>> {
        let entity = self.get_entity(entity.raw_entity()).ok()?;
        let handle = entity.get::<AssetSelfHandle>()?;
        handle.upgrade().ok()
    }

    fn get_asset_strong_handle_untyped(&self, entity: AssetEntity) -> Option<UntypedHandle> {
        let entity = self.get_entity(entity.raw_entity()).ok()?;
        let handle = entity.get::<AssetSelfHandle>()?;
        handle.upgrade_untyped()
    }
}

impl AssetServerAccessExt for World {
    fn load_asset<'a, A: Asset>(&self, path: impl Into<AssetPath<'a>>) -> Handle<A> {
        self.resource::<AssetServer>().load(path)
    }

    fn load_asset_with_settings<'a, A: Asset, S: Settings>(
        &self,
        path: impl Into<AssetPath<'a>>,
        settings: impl Fn(&mut S) + Send + Sync + 'static,
    ) -> Handle<A> {
        self.resource::<AssetServer>()
            .load_with_settings(path, settings)
    }
}

pub trait WorldAssetCommandsExt {
    /// Creates a new [`WorldAssetCommands`] instance that writes to the world's command queue.
    ///
    /// Use [`World::flush`] to apply all queued commands.
    fn asset_commands(&mut self) -> WorldAssetCommands<'_, '_>;
}

impl WorldAssetCommandsExt for World {
    fn asset_commands(&mut self) -> WorldAssetCommands<'_, '_> {
        WorldAssetCommands {
            provider: self.resource::<AssetHandleProvider>().clone(),
            commands: self.commands(),
        }
    }
}

impl WorldAssetCommandsExt for DeferredWorld<'_> {
    fn asset_commands(&mut self) -> WorldAssetCommands<'_, '_> {
        WorldAssetCommands {
            provider: self.resource::<AssetHandleProvider>().clone(),
            commands: self.commands(),
        }
    }
}

/// A [`SystemParam`] with commands for manipulating assets.
///
/// Similar to [`Commands`], these actions are applied at the next sync point.
#[derive(SystemParam)]
pub struct AssetCommands<'w, 's> {
    /// Commands to use for creating new asset.
    commands: Commands<'w, 's>,
    /// The provider for the actual handles.
    provider: Res<'w, AssetHandleProvider>,
}

impl<'w, 's> AssetCommands<'w, 's> {
    /// Spawn a new asset and return the handle.
    #[must_use]
    pub fn spawn_asset<A: Asset>(&mut self, asset: A) -> Handle<A> {
        InternalAssetCommands {
            commands: &mut self.commands,
            provider: &self.provider,
        }
        .spawn_asset(asset)
    }

    /// Spawns a new asset referenced by `uuid`.
    ///
    /// Calling this multiples times with the same `uuid` will replace the asset each time - so only
    /// the final asset will remain. Calling this does not free the asset if the handle is dropped:
    /// UUID assets are kept alive unless they are manually despawned or set to another handle with
    /// [`AssetUuidMap::set_uuid`].
    pub fn spawn_uuid_asset<A: Asset>(&mut self, uuid: Uuid, asset: A) -> Handle<A> {
        InternalAssetCommands {
            commands: &mut self.commands,
            provider: &self.provider,
        }
        .spawn_uuid_asset(uuid, asset)
    }

    /// Attempts to insert the asset into the entity referenced by `id`.
    pub fn insert_asset<A: Asset>(&mut self, id: impl Into<AssetId<A>>, asset: A) {
        InternalAssetCommands {
            commands: &mut self.commands,
            provider: &self.provider,
        }
        .insert_asset(id.into(), asset);
    }

    /// Removes the asset referenced by `id`.
    pub fn remove_asset<A: Asset>(&mut self, id: AssetId<A>) {
        InternalAssetCommands {
            commands: &mut self.commands,
            provider: &self.provider,
        }
        .remove_asset(id);
    }

    /// Reserves a handle for an asset of type `A`.
    #[must_use]
    pub fn reserve_handle<A: Asset>(&mut self) -> Handle<A> {
        InternalAssetCommands {
            commands: &mut self.commands,
            provider: &self.provider,
        }
        .reserve_handle()
    }

    /// Returns the internal [`Commands`] used to update assets.
    ///
    /// This is an "escape hatch" to allow adding commands to the same command buffer as that used
    /// for assets. This allows users to ensure enqueued commands run after any assets are
    /// spawned/inserted/removed.
    pub fn commands(&mut self) -> &mut Commands<'w, 's> {
        &mut self.commands
    }
}

/// A version of [`AssetCommands`] which is not a [`SystemParam`], allowing it to be used with
/// [`DeferredWorld`] access.
pub struct WorldAssetCommands<'w, 's> {
    /// Commands to use for creating new asset.
    commands: Commands<'w, 's>,
    /// The provider for the actual handles.
    provider: AssetHandleProvider,
}

impl<'w, 's> WorldAssetCommands<'w, 's> {
    /// Spawn a new asset and return the handle.
    #[must_use]
    pub fn spawn_asset<A: Asset>(&mut self, asset: A) -> Handle<A> {
        InternalAssetCommands {
            commands: &mut self.commands,
            provider: &self.provider,
        }
        .spawn_asset(asset)
    }

    /// Spawns a new asset referenced by `uuid`.
    ///
    /// Calling this multiples times with the same `uuid` will replace the asset each time - so only
    /// the final asset will remain. Calling this does not free the asset if the handle is dropped:
    /// UUID assets are kept alive unless they are manually despawned or set to another handle with
    /// [`AssetUuidMap::set_uuid`].
    pub fn spawn_uuid_asset<A: Asset>(&mut self, uuid: Uuid, asset: A) -> Handle<A> {
        InternalAssetCommands {
            commands: &mut self.commands,
            provider: &self.provider,
        }
        .spawn_uuid_asset(uuid, asset)
    }

    /// Attempts to insert the asset into the entity referenced by `id`.
    pub fn insert_asset<A: Asset>(&mut self, id: impl Into<AssetId<A>>, asset: A) {
        InternalAssetCommands {
            commands: &mut self.commands,
            provider: &self.provider,
        }
        .insert_asset(id.into(), asset);
    }

    /// Removes the asset referenced by `id`.
    pub fn remove_asset<A: Asset>(&mut self, id: AssetId<A>) {
        InternalAssetCommands {
            commands: &mut self.commands,
            provider: &self.provider,
        }
        .remove_asset(id);
    }

    /// Reserves a handle for an asset of type `A`.
    #[must_use]
    pub fn reserve_handle<A: Asset>(&mut self) -> Handle<A> {
        InternalAssetCommands {
            commands: &mut self.commands,
            provider: &self.provider,
        }
        .reserve_handle()
    }

    /// Returns the internal [`Commands`] used to update assets.
    ///
    /// This is an "escape hatch" to allow adding commands to the same command buffer as that used
    /// for assets. This allows users to ensure enqueued commands run after any assets are
    /// spawned/inserted/removed.
    pub fn commands(&mut self) -> &mut Commands<'w, 's> {
        &mut self.commands
    }
}

/// Implementation of [`AssetCommands`] and [`WorldAssetCommands`].
struct InternalAssetCommands<'a, 'w, 's> {
    /// Commands to use for creating new asset.
    commands: &'a mut Commands<'w, 's>,
    /// The provider for the actual handles.
    provider: &'a AssetHandleProvider,
}

impl InternalAssetCommands<'_, '_, '_> {
    /// Same as [`AssetCommands::spawn_asset`].
    #[must_use]
    fn spawn_asset<A: Asset>(&mut self, asset: A) -> Handle<A> {
        let handle = self.reserve_handle();
        // reserve_handle always returns a Strong so unwrapping is safe.
        let entity = handle.entity().unwrap();
        self.commands.queue(move |world: &mut World| {
            // We just spawned the asset, so inserting into it should be safe.
            insert_asset(world, entity, asset).unwrap();
        });
        handle
    }

    /// Same as [`AssetCommands::spawn_uuid_asset`].
    fn spawn_uuid_asset<A: Asset>(&mut self, uuid: Uuid, asset: A) -> Handle<A> {
        let handle = self.spawn_asset::<A>(asset);
        // spawn_asset always returns a Strong variant, so unwrap here is safe.
        let entity_handle = UntypedEntityHandle::from(EntityHandle::try_from(handle).unwrap());
        self.commands.queue(move |world: &mut World| {
            world
                .resource_mut::<AssetUuidMap>()
                .set_uuid(uuid, entity_handle);
            // Send an asset event that this UUID has been added.
            // TODO: This isn't quite sufficient, since users can call set_uuid themselves. We should be
            // sending a message in that case as well. This also doesn't work correctly if the UUID
            // was already set.
            world.write_message(AssetEvent::<A>::Added {
                id: AssetId::Uuid { uuid },
            });
        });
        Handle::Uuid(uuid, PhantomData)
    }

    /// Same as [`AssetCommands::insert_asset`].
    fn insert_asset<A: Asset>(&mut self, id: AssetId<A>, asset: A) {
        self.commands.queue(move |world: &mut World| {
            let entity = match world.resource::<AssetUuidMap>().resolve_entity(id) {
                Ok(entity) => entity,
                Err(err) => {
                    warn!("Failed to insert into handle: {err}");
                    return;
                }
            };
            if let Err(err) = insert_asset(world, entity, asset) {
                warn!("Failed to insert asset: {err}");
            }
        });
    }

    /// Same as [`AssetCommands::remove_asset`].
    fn remove_asset<A: Asset>(&mut self, id: AssetId<A>) {
        self.commands.queue(move |world: &mut World| {
            let _ = world.remove_asset(id);
        });
    }

    /// Same as [`AssetCommands::reserve_handle`].
    #[must_use]
    fn reserve_handle<A: Asset>(&mut self) -> Handle<A> {
        let entity = AssetEntity::new_unchecked(self.commands.spawn_empty().id());
        let handle = self
            .provider
            .create_handle(entity, TypeId::of::<A>(), None, None);
        {
            let handle = handle.clone();
            self.commands.queue(move |world: &mut World| {
                setup_asset(world, &handle).unwrap();
            });
        }
        Handle::Strong(handle)
    }
}

/// An error when inserting an asset.
#[derive(Error, Debug)]
pub enum InsertAssetError {
    /// The handle held a UUID which is not assigned.
    #[error(transparent)]
    ResolveHandle(#[from] ResolveUuidError),
    /// The entity for the handle does not exist.
    #[error(transparent)]
    EntityDoesNotExist(#[from] AssetEntityDoesNotExistError),
}

/// An error when inserting an asset.
#[derive(Error, Debug)]
pub enum RemoveAssetError {
    /// The id held a UUID which is not assigned.
    #[error(transparent)]
    ResolveHandle(#[from] ResolveUuidError),
    /// The entity for the id does not exist.
    #[error(transparent)]
    EntityFetchError(#[from] EntityMutableFetchError),
    /// The entity does not contain the given asset data.
    #[error("The entity {0} does not contain asset data")]
    AssetMissingData(AssetEntity),
}
