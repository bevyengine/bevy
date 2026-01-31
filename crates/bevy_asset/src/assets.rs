use crate::{
    asset_changed::AssetChanges, Asset, AssetEntity, AssetEvent, AssetEventUnusedWriters,
    AssetHandleProvider, AssetId, AssetSelfHandle, AssetUuidMap, Handle, StrongHandle,
    UntypedHandle,
};
use alloc::sync::Arc;
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    lifecycle::HookContext,
    message::MessageWriter,
    query::Changed,
    system::{Query, Res, ResMut, SystemChangeTick, SystemParam},
    world::{DeferredWorld, Mut, Ref, World},
};
use bevy_reflect::TypePath;
use core::{marker::PhantomData, ops::Deref};
use derive_more::{Deref, DerefMut};
use thiserror::Error;

/// Stores the actual data of an asset.
#[derive(Component, Default, Deref, DerefMut)]
#[component(on_add=write_added_asset_event::<A>, on_remove=write_removed_asset_event::<A>)]
pub struct AssetData<A: Asset>(pub A);

/// [`SystemParam`] providing convenient access to assets using handles.
///
/// While it is possible to access assets without this [`SystemParam`], this param resolves the
/// handles into entities for you.
///
/// For mutable access, use [`AssetsMut`] instead. For creating new assets, see
/// [`DirectAssetAccessExt`](crate::DirectAssetAccessExt) or
/// [`AssetCommands`](crate::AssetCommands).
#[derive(SystemParam)]
pub struct Assets<'w, 's, A: Asset> {
    /// Uuid map allowing us to resolve [`Handle::Uuid`] into entities that we can query for.
    uuid_map: Res<'w, AssetUuidMap>,
    /// The query for the actual asset data.
    ///
    /// Includes [`Entity`] to support iterating and returning [`AssetEntity`].
    assets: Query<'w, 's, (Entity, &'static AssetData<A>)>,
    /// The query for the self-handles of assets, allowing us to return an asset's strong handle.
    handles: Query<'w, 's, &'static AssetSelfHandle>,
}

/// [`SystemParam`] providing convenient access to assets using handles.
///
/// While it is possible to access assets without this [`SystemParam`], this param resolves the
/// handles into entities for you.
///
/// For only immutable access, use [`Assets`] instead. For creating new assets, see
/// [`DirectAssetAccessExt`](crate::DirectAssetAccessExt) or
/// [`AssetCommands`](crate::AssetCommands).
#[derive(SystemParam)]
pub struct AssetsMut<'w, 's, A: Asset> {
    /// Uuid map allowing us to resolve [`Handle::Uuid`] into entities that we can query for.
    uuid_map: Res<'w, AssetUuidMap>,
    /// The query for the actual asset data.
    ///
    /// Includes [`Entity`] to support iterating and returning [`AssetEntity`].
    assets: Query<'w, 's, (Entity, &'static mut AssetData<A>)>,
    /// The query for the self-handles of assets, allowing us to return an asset's strong handle.
    handles: Query<'w, 's, &'static AssetSelfHandle>,
}

impl<A: Asset> Assets<'_, '_, A> {
    /// Gets the data associated with the `handle` if it exists.
    ///
    /// This can return [`None`] for several reasons. For example, the asset could be despawned,
    /// the asset may not currently have the asset data, or the UUID of the `id` may not resolve.
    pub fn get(&self, id: impl Into<AssetId<A>>) -> Option<&A> {
        let entity = self.uuid_map.resolve_entity(id.into()).ok()?;
        self.assets
            .get(entity.raw_entity())
            .ok()
            .map(|(_, data)| &data.0)
    }

    /// Gets the strong handle for an entity.
    ///
    /// Handles are the primary "reference" for assets - most APIs will store handles. This also
    /// allows you to keep the asset from being automatically despawned for as long as you hold the
    /// handle.
    ///
    /// Returns [`None`] if the handles for this asset have already expired, meaning this asset is
    /// queued for despawning.
    pub fn get_strong_handle(&self, entity: AssetEntity) -> Option<Handle<A>> {
        let self_handle = self.handles.get(entity.raw_entity()).ok()?;
        self_handle.upgrade().ok()
    }

    /// Returns `true` if the corresponding entity contains asset data, and `false` otherwise.
    pub fn contains(&self, id: impl Into<AssetId<A>>) -> bool {
        let Ok(entity) = self.uuid_map.resolve_entity(id.into()) else {
            return false;
        };
        self.assets.contains(entity.raw_entity())
    }

    /// Iterates through all assets.
    pub fn iter(&self) -> impl Iterator<Item = (AssetEntity, &'_ A)> {
        self.assets
            .iter()
            .map(|(entity, data)| (AssetEntity::new_unchecked(entity), data.deref()))
    }

    /// Returns `true` if there are no assets.
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// Returns the number of asset currently stored.
    pub fn count(&self) -> usize {
        self.assets.count()
    }
}

impl<A: Asset> AssetsMut<'_, '_, A> {
    /// Gets the data associated with the `id` if it exists.
    ///
    /// This can return [`None`] for several reasons. For example, the asset could be despawned,
    /// the asset may not currently have the asset data, or the UUID of the `id` may not resolve.
    pub fn get(&self, id: impl Into<AssetId<A>>) -> Option<&A> {
        let entity = self.uuid_map.resolve_entity(id.into()).ok()?;
        self.assets
            .get(entity.raw_entity())
            .ok()
            .map(|(_, data)| &data.0)
    }

    /// Gets the data (mutably) associated with the `id` if it exists.
    ///
    /// This can return [`None`] for several reasons. For example, the asset could be despawned,
    /// the asset may not currently have the asset data, or the UUID of the `id` may not resolve.
    pub fn get_mut(&mut self, id: impl Into<AssetId<A>>) -> Option<AssetMut<'_, A>> {
        let entity = self.uuid_map.resolve_entity(id.into()).ok()?;
        self.assets
            .get_mut(entity.raw_entity())
            .ok()
            .map(|(_, data)| AssetMut(data))
    }

    /// Gets the strong handle for an entity.
    ///
    /// Handles are the primary "reference" for assets - most APIs will store handles. This also
    /// allows you to keep the asset from being automatically despawned for as long as you hold the
    /// handle.
    ///
    /// Returns [`None`] if the handles for this asset have already expired, meaning this asset is
    /// queued for despawning.
    pub fn get_strong_handle(&self, entity: Entity) -> Option<Handle<A>> {
        let self_handle = self.handles.get(entity).ok()?;
        self_handle.upgrade().ok()
    }

    /// Returns `true` if the corresponding entity contains asset data, and `false` otherwise.
    pub fn contains(&self, id: impl Into<AssetId<A>>) -> bool {
        let Ok(entity) = self.uuid_map.resolve_entity(id.into()) else {
            return false;
        };
        self.assets.contains(entity.raw_entity())
    }

    /// Iterates through all assets.
    pub fn iter(&mut self) -> impl Iterator<Item = (AssetEntity, &'_ A)> {
        self.assets
            .iter()
            .map(|(entity, data)| (AssetEntity::new_unchecked(entity), data.deref()))
    }

    /// Iterates through all assets mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (AssetEntity, AssetMut<'_, A>)> {
        self.assets
            .iter_mut()
            .map(|(entity, data)| (AssetEntity::new_unchecked(entity), AssetMut(data)))
    }

    /// Returns `true` if there are no assets.
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// Returns the number of asset currently stored.
    pub fn count(&self) -> usize {
        self.assets.count()
    }
}

/// Mutable access to asset data of type `A`.
///
/// To acquite an instance, see [`AssetsMut::get_mut`] or [`AssetsMut::iter_mut`].
///
/// This mirrors the [`Mut`] type (providing mutable access to the asset component, as well as
/// change detection info), but hides the [`AssetData`] wrapper (the component actually storing the
/// asset data).
#[derive(Deref, DerefMut)]
pub struct AssetMut<'w, A: Asset>(pub(crate) Mut<'w, AssetData<A>>);

impl<'w, A: Asset> AssetMut<'w, A> {
    /// Consumes `self` and returns a mutable reference to the contained value while marking `self`
    /// as "changed".
    pub fn into_inner(self) -> &'w mut A {
        self.0.into_inner()
    }
}

/// A "loaded asset" containing the untyped handle for an asset stored in a given [`AssetPath`].
///
/// [`AssetPath`]: crate::AssetPath
#[derive(Asset, TypePath)]
pub struct LoadedUntypedAsset {
    /// The handle to the loaded asset.
    #[dependency]
    pub handle: UntypedHandle,
}

pub(crate) fn setup_asset(
    world: &mut World,
    handle: &Arc<StrongHandle>,
) -> Result<(), AssetEntityDoesNotExistError> {
    let Ok(mut entity) = world.get_entity_mut(handle.entity.raw_entity()) else {
        return Err(AssetEntityDoesNotExistError(handle.entity));
    };
    entity.insert(AssetSelfHandle(Arc::downgrade(handle)));
    Ok(())
}

pub(crate) fn insert_asset<A: Asset>(
    world: &mut World,
    entity: AssetEntity,
    asset: A,
) -> Result<(), AssetEntityDoesNotExistError> {
    let Ok(mut entity) = world.get_entity_mut(entity.raw_entity()) else {
        return Err(AssetEntityDoesNotExistError(entity));
    };

    entity.insert(AssetData(asset));
    Ok(())
}

/// Writes [`AssetEvent::Added`] for this asset.
fn write_added_asset_event<A: Asset>(
    mut world: DeferredWorld,
    HookContext { entity, .. }: HookContext,
) {
    let entity = AssetEntity::new_unchecked(entity);
    world.write_message(AssetEvent::<A>::Added {
        id: AssetId::Entity {
            entity,
            marker: PhantomData,
        },
    });
    // TODO: Replace this with `change_tick` once it's supported. See #22788.
    let tick = world.read_change_tick();
    if let Some(mut changes) = world.get_resource_mut::<AssetChanges<A>>() {
        changes.insert(entity, tick);
    }
}

/// Writes [`AssetEvent::Removed`] for this asset.
fn write_removed_asset_event<A: Asset>(
    mut world: DeferredWorld,
    HookContext { entity, .. }: HookContext,
) {
    let entity = AssetEntity::new_unchecked(entity);
    world.write_message(AssetEvent::<A>::Removed {
        id: AssetId::Entity {
            entity,
            marker: PhantomData,
        },
    });
    if let Some(mut changes) = world.get_resource_mut::<AssetChanges<A>>() {
        changes.remove(&entity);
    }
}

/// Writes [`AssetEvent::Modified`] messages for any assets that have changed.
// TODO: We should remove this and leave it up to users to listen for these events.
pub(crate) fn write_modified_asset_events<A: Asset>(
    assets: Query<(Entity, Ref<AssetData<A>>), Changed<AssetData<A>>>,
    ticks: SystemChangeTick,
    mut messages: MessageWriter<AssetEvent<A>>,
    mut changes: Option<ResMut<AssetChanges<A>>>,
) {
    for (entity, data_ref) in assets.iter() {
        let entity = AssetEntity::new_unchecked(entity);
        // Always count added assets for `AssetChanged`.
        if let Some(changes) = changes.as_mut() {
            changes.insert(entity, ticks.this_run());
        }
        if data_ref.last_changed() == data_ref.added() {
            // The change corresponds to new asset data, which would have been handled by
            // `AssetData`s hooks.
            continue;
        }
        messages.write(AssetEvent::Modified {
            id: AssetId::Entity {
                entity,
                marker: PhantomData,
            },
        });
    }
}

/// An exclusive system that despawns assets whose handles have been dropped and so are "unused".
pub(crate) fn despawn_unused_assets(world: &mut World) {
    // Note: we use an exclusive system here so that despawning an asset can trigger drops of other
    // handles, which themselves get despawned in the same system (until there are no more dropped
    // handles).

    let drop_receiver = world
        .resource::<AssetHandleProvider>()
        .drop_receiver
        .clone();
    for (entity, type_id) in drop_receiver.try_iter() {
        AssetEventUnusedWriters::write_message(world, entity, type_id)
            .expect("Asset type has been registered");

        world.despawn(entity.raw_entity());
    }
}

/// An error for asset actions when the entity does not exist for that asset.
#[derive(Error, Debug)]
#[error("The requested entity {0:?} does not exist")]
pub struct AssetEntityDoesNotExistError(pub AssetEntity);
