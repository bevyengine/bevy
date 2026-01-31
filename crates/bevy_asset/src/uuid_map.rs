use alloc::sync::Arc;

use bevy_ecs::resource::Resource;
use bevy_platform::{
    collections::HashMap,
    sync::{PoisonError, RwLock, RwLockReadGuard},
};
use bevy_utils::TypeIdMap;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    Asset, AssetEntity, EntityHandle, Handle, UntypedAssetId, UntypedEntityHandle, UntypedHandle,
};

/// Maps asset UUIDs to the asset handle assigned to it.
#[derive(Resource, Clone, Default)]
pub struct AssetUuidMap(Arc<RwLock<TypeIdMap<HashMap<Uuid, UntypedEntityHandle>>>>);

impl AssetUuidMap {
    /// Sets the handle that a UUID refers to.
    pub fn set_uuid(&mut self, uuid: Uuid, handle: UntypedEntityHandle) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(handle.0.type_id)
            .or_default()
            .insert(uuid, handle);
    }

    /// Convenience function for accessing the internal uuid map.
    fn read(&self) -> RwLockReadGuard<'_, TypeIdMap<HashMap<Uuid, UntypedEntityHandle>>> {
        self.0.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Converts an untyped handle into the corresponding [`UntypedEntityHandle`].
    ///
    /// For [`UntypedHandle::Strong`], this is a no-op. For [`UntypedHandle::Uuid`], this lookups
    /// the corresponding UUID and returns [`Err`] if missing.
    pub fn resolve_untyped_handle(
        &self,
        handle: UntypedHandle,
    ) -> Result<UntypedEntityHandle, ResolveUuidError> {
        match handle {
            UntypedHandle::Strong(inner) => Ok(UntypedEntityHandle(inner)),
            UntypedHandle::Uuid { type_id, uuid } => self
                .read()
                .get(&type_id)
                .and_then(|map| map.get(&uuid))
                .cloned()
                .ok_or(ResolveUuidError(uuid)),
        }
    }

    /// Converts a handle into the corresponding [`EntityHandle`].
    ///
    /// For [`Handle::Strong`], this is a no-op. For [`Handle::Uuid`], this lookups the
    /// corresponding UUID and returns [`Err`] if missing.
    pub fn resolve_handle<A: Asset>(
        &self,
        handle: Handle<A>,
    ) -> Result<EntityHandle<A>, ResolveUuidError> {
        self.resolve_untyped_handle(handle.untyped())
            // It's safe to unwrap, since either the handle was just passed through, or we looked up
            // the handle by its type ID, so the types must match.
            .map(|handle| handle.try_typed().unwrap())
    }

    /// Converts an asset ID into the corresponding [`AssetEntity`].
    ///
    /// This is the same as [`Self::resolve_handle`], but is slightly more efficient for
    /// cases where you don't need the resolved handle.
    pub fn resolve_entity(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> Result<AssetEntity, ResolveUuidError> {
        match id.into() {
            UntypedAssetId::Entity { entity, .. } => Ok(entity),
            UntypedAssetId::Uuid { type_id, uuid } => self
                .read()
                .get(&type_id)
                .and_then(|map| map.get(&uuid))
                .map(|value| value.0.entity)
                .ok_or(ResolveUuidError(uuid)),
        }
    }
}

/// An error while resolve a [`Uuid`] in the [`AssetUuidMap`].
#[derive(Error, Debug)]
#[error("There is no asset handle assigned to uuid {0}")]
pub struct ResolveUuidError(pub Uuid);
