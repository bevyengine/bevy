use bevy_app::Plugin;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    observer::Trigger,
    query::With,
    reflect::ReflectComponent,
    system::{Local, Query, ResMut, Resource, SystemState},
    world::{Mut, OnAdd, OnRemove, World},
};
use bevy_hierarchy::DespawnRecursiveExt;
use bevy_reflect::Reflect;
use bevy_utils::tracing::warn;

/// Marker component that indicates that its entity needs to be Synchronized to the render world
///
/// NOTE: This component should persist throughout the entity's entire lifecycle.
/// If this component is removed from its entity, the entity will be despawned.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect[Component]]
#[component(storage = "SparseSet")]
pub struct SyncRenderWorld;

// marker component that indicates that its entity needs to be despawned at the end of every frame.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[component(storage = "SparseSet")]
pub struct TemporaryRenderEntity;

// TODO: directly remove matched archetype for performance
pub(crate) fn despawn_temporary_render_entity(
    world: &mut World,
    state: &mut SystemState<Query<Entity, With<TemporaryRenderEntity>>>,
    mut local: Local<Vec<Entity>>,
) {
    let query = state.get(world);

    local.extend(query.iter());

    // ensure next frame allocation keeps order
    local.sort_unstable_by_key(|e| e.index());
    for e in local.drain(..).rev() {
        world.despawn(e);
    }
}
