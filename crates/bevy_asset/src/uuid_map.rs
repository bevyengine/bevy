use core::any::TypeId;

use alloc::sync::Arc;

use bevy_ecs::resource::Resource;
use bevy_platform::{
    collections::{hash_map::Entry, HashMap, HashSet},
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
pub struct AssetUuidMap(Arc<RwLock<TypeIdMap<AssetUuidMapInner>>>);

#[derive(Default)]
pub(crate) struct AssetUuidMapInner {
    pub(crate) uuid_to_handle: HashMap<Uuid, UntypedEntityHandle>,
    entity_to_uuids: HashMap<AssetEntity, HashSet<Uuid>>,
}

impl AssetUuidMap {
    /// Sets the handle that a UUID refers to.
    pub fn set_uuid(&mut self, uuid: Uuid, handle: UntypedEntityHandle) {
        let mut type_id_map = self.0.write().unwrap_or_else(PoisonError::into_inner);
        let inner = type_id_map.entry(handle.0.type_id).or_default();
        let new_entity = handle.entity();
        match inner.uuid_to_handle.entry(uuid) {
            Entry::Vacant(entry) => {
                entry.insert(handle);
            }
            Entry::Occupied(mut entry) => {
                let old_entity = entry.get().entity();
                inner
                    .entity_to_uuids
                    .get_mut(&old_entity)
                    .unwrap()
                    .remove(&uuid);
                entry.insert(handle);
            }
        }
        inner
            .entity_to_uuids
            .entry(new_entity)
            .or_default()
            .insert(uuid);
    }

    /// Convenience function for accessing the internal uuid map.
    pub(crate) fn read(&self) -> RwLockReadGuard<'_, TypeIdMap<AssetUuidMapInner>> {
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
                .and_then(|inner| inner.uuid_to_handle.get(&uuid))
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
                .and_then(|inner| inner.uuid_to_handle.get(&uuid))
                .map(|value| value.0.entity)
                .ok_or(ResolveUuidError(uuid)),
        }
    }

    /// Returns a reverse mapping from an entity to all UUIDs that reference it.
    pub(crate) fn entity_to_uuids(
        &self,
        entity: AssetEntity,
        type_id: TypeId,
    ) -> Option<HashSet<Uuid>> {
        Some(
            self.read()
                .get(&type_id)?
                .entity_to_uuids
                .get(&entity)?
                .clone(),
        )
    }
}

/// An error while resolve a [`Uuid`] in the [`AssetUuidMap`].
#[derive(Error, Debug)]
#[error("There is no asset handle assigned to uuid {0}")]
pub struct ResolveUuidError(pub Uuid);
