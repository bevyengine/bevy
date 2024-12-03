use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Asset, AssetEvent, AssetId};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Component;
use bevy_ecs::entity::{Entity, EntityHashSet};
use bevy_ecs::event::EventReader;
use bevy_ecs::observer::Trigger;
use bevy_ecs::query::Changed;
use bevy_ecs::system::{Query, ResMut, Resource};
use bevy_ecs::world::{OnAdd, OnRemove, OnReplace};
use bevy_utils::{HashMap, HashSet};
use core::marker::PhantomData;

/// A plugin that tracks added assets and changes to entities that hold them. Provides
/// the following resources: [`ChangedAssets`], [`AssetEntityMap`].
pub struct ChangedAssetsPlugin<A, H> {
    asset_marker: PhantomData<A>,
    handle_marker: PhantomData<H>,
}

impl<A, H> Plugin for ChangedAssetsPlugin<A, H>
where
    A: Asset,
    H: Component,
    AssetId<A>: for<'a> From<&'a H>,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<ChangedAssets<A>>()
            .init_resource::<AssetEntityMap<A>>()
            .add_systems(PostUpdate, maintain_changed_assets::<A, H>)
            .add_observer(on_add_handle::<A, H>)
            .add_observer(on_replace_handle::<A, H>)
            .add_observer(on_remove_handle::<A, H>);
    }
}

fn on_add_handle<A, H>(
    added: Trigger<OnAdd, H>,
    query: Query<&H>,
    mut asset_entity_map: ResMut<AssetEntityMap<A>>,
) where
    A: Asset,
    H: Component,
    AssetId<A>: for<'a> From<&'a H>,
{
    let handle = query.get(added.entity()).unwrap();
    asset_entity_map
        .entry(AssetId::<A>::from(handle))
        .or_default()
        .insert(added.entity());
}

fn on_replace_handle<A, H>(
    replaced: Trigger<OnReplace, H>,
    query: Query<&H>,
    mut asset_entity_map: ResMut<AssetEntityMap<A>>,
) where
    A: Asset,
    H: Component,
    AssetId<A>: for<'a> From<&'a H>,
{
    let handle = query.get(replaced.entity()).unwrap();
    asset_entity_map
        .entry(AssetId::<A>::from(handle))
        .or_default()
        .remove(&replaced.entity());
}

fn on_remove_handle<A, H>(
    removed: Trigger<OnRemove, H>,
    query: Query<&H>,
    mut asset_entity_map: ResMut<AssetEntityMap<A>>,
) where
    A: Asset,
    H: Component,
    AssetId<A>: for<'a> From<&'a H>,
{
    let handle = query.get(removed.entity()).unwrap();
    asset_entity_map
        .entry(AssetId::<A>::from(handle))
        .or_default()
        .remove(&removed.entity());
}

impl<A, H> Default for ChangedAssetsPlugin<A, H>
where
    A: Asset,
    H: Component,
    AssetId<A>: for<'a> From<&'a H>,
{
    fn default() -> Self {
        Self {
            asset_marker: PhantomData,
            handle_marker: PhantomData,
        }
    }
}

pub fn maintain_changed_assets<A, H>(
    mut events: EventReader<AssetEvent<A>>,
    mut changed_assets: ResMut<ChangedAssets<A>>,
    mut asset_entity_map: ResMut<AssetEntityMap<A>>,
    changed_handles: Query<(Entity, &H), Changed<H>>,
) where
    A: Asset,
    H: Component,
    AssetId<A>: for<'a> From<&'a H>,
{
    changed_assets.clear();
    let mut removed = HashSet::new();

    for event in events.read() {
        #[allow(clippy::match_same_arms)]
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                changed_assets.insert(*id);
                removed.remove(id);
            }
            AssetEvent::Removed { id } => {
                changed_assets.remove(id);
                removed.insert(id);
            }
            AssetEvent::Unused { .. } => {}
            AssetEvent::LoadedWithDependencies { .. } => {
                // TODO: handle this
            }
        }
    }

    // Update asset entity map if the handle of an entity has changed, i.e. was mutated.
    for (entity, handle) in changed_handles.iter() {
        asset_entity_map
            .entry(AssetId::<A>::from(handle))
            .or_default()
            .insert(entity);
    }

    for asset in removed.drain() {
        asset_entity_map.remove(asset);
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct ChangedAssets<A: Asset>(HashSet<AssetId<A>>);

impl<A: Asset> Default for ChangedAssets<A> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct AssetEntityMap<A: Asset>(HashMap<AssetId<A>, EntityHashSet>);

impl<A: Asset> Default for AssetEntityMap<A> {
    fn default() -> Self {
        Self(Default::default())
    }
}
