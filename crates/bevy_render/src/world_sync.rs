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

/// A Plugin that synchronizes entities with specific Components between the main world and render world.
///
/// Bevy's renderer is architected independently from main app, It operates in its own separate ECS World. Therefore, the renderer could run in parallel with main app logic, This is called "Pipelined Rendering". See [`PipelinedRenderingPlugin`] for more information.
///
/// Previously, `extract` will copy the related main world entity and its data into the render world , and then render world will clear all render entities at the end of frame to reserve enough entity space to ensure that no main world entity ID has been occupied during next `extract`.
///
/// With `* as entities`, we should not clear all entities in render world because some core metadata (e.g. [`Component`], [`Query`]) are also stored in the form of entity.
///
/// So we turn to an entity-to-entity mapping strategy to sync between main world entity and render world entity,where each `synchronized` main entity has a component [`RenderEntity`] that holds an Entity ID pointer to its unique counterpart entity in the render world.
///
/// A example for `synchronized` main entity 1v1 and 18v1
///
/// ```text
/// |---------------------------Main World----------------------------|
/// |  Entity  |                    Component                         |
/// |-----------------------------------------------------------------|
/// | ID: 1v1  | PointLight | RenderEntity(ID: 3V1) | SyncRenderWorld |
/// | ID: 18v1 | PointLight | RenderEntity(ID: 5V1) | SyncRenderWorld |
/// |-----------------------------------------------------------------|
///
/// |----------Render World-----------|
/// |  Entity  |       Component      |
/// |---------------------------------|
/// | ID: 3v1  | MainEntity(ID: 1V1)  |
/// | ID: 5v1  | MainEntity(ID: 18V1) |
/// |---------------------------------|
///
/// ```
///
/// To establish a "Synchronous Relationship", you can add a [`SyncRenderWorld`] component to an entity, indicating that it needs to be synchronized with the render world.
///
/// Now a single frame of execution looks something like below
///
/// ```text
/// |--------------------------------------------------------------------|
/// |      |         |          Main   world lopp                        |
/// | Sync | extract |---------------------------------------------------|
/// |      |         |         Render wrold loop                         |
/// |--------------------------------------------------------------------|
/// ```
///
/// `Sync` is the step that syncs main entity behavior(add, remove) to its counterpart render entity.
///
/// [`PipelinedRenderingPlugin`]: crate::pipelined_rendering::PipelinedRenderingPlugin
#[derive(Default)]
pub struct WorldSyncPlugin;

impl Plugin for WorldSyncPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<PendingSyncEntity>();
        app.observe(
            |trigger: Trigger<OnAdd, SyncRenderWorld>, mut pending: ResMut<PendingSyncEntity>| {
                pending.push(EntityRecord::Added(trigger.entity()));
            },
        );
        app.observe(
            |trigger: Trigger<OnRemove, SyncRenderWorld>,
             mut pending: ResMut<PendingSyncEntity>,
             query: Query<&RenderEntity>| {
                if let Ok(e) = query.get(trigger.entity()) {
                    pending.push(EntityRecord::Removed(e.id()));
                };
            },
        );
    }
}
/// Marker component that indicates that its entity needs to be Synchronized to the render world
///
/// NOTE: This component should persist throughout the entity's entire lifecycle.
/// If this component is removed from its entity, the entity will be despawned.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect[Component]]
#[component(storage = "SparseSet")]
pub struct SyncRenderWorld;

#[derive(Component, Deref, Clone, Debug, Copy)]
/// Marker component added on the main world entities that are synced to the Render World in order to keep track of the corresponding render world entity
pub struct RenderEntity(Entity);
impl RenderEntity {
    #[inline]
    pub fn id(&self) -> Entity {
        self.0
    }
}

#[derive(Component, Deref, Clone, Debug)]
/// Marker component added on the render world entities to keep track of the corresponding main world entity
pub struct MainEntity(Entity);
impl MainEntity {
    #[inline]
    pub fn id(&self) -> Entity {
        self.0
    }
}

// marker component that indicates that its entity needs to be despawned at the end of every frame.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct RenderFlyEntity;

pub(crate) enum EntityRecord {
    // When an entity is spawned on the main world, notify the render world so that it can spawn a corresponding entity. This contains the main world entity
    Added(Entity),
    // When an entity is despawned on the main world, notify the render world so that the corresponding entity can be despawned. This contains the render world entity.
    Removed(Entity),
}

// Entity Record in MainWorld pending to Sync
#[derive(Resource, Default, Deref, DerefMut)]
pub(crate) struct PendingSyncEntity {
    records: Vec<EntityRecord>,
}

pub(crate) fn entity_sync_system(main_world: &mut World, render_world: &mut World) {
    main_world.resource_scope(|world, mut pending: Mut<PendingSyncEntity>| {
        // TODO : batching record
        for record in pending.drain(..) {
            match record {
                EntityRecord::Added(e) => {
                    if let Some(mut entity) = world.get_entity_mut(e) {
                        match entity.entry::<RenderEntity>() {
                            bevy_ecs::world::Entry::Occupied(_) => {
                                warn!("Attempting to synchronize an entity that has already been synchronized!");
                            }
                            bevy_ecs::world::Entry::Vacant(entry) => {
                                let id = render_world.spawn(MainEntity(e)).id();

                                entry.insert(RenderEntity(id));
                            }
                        };
                    }
                }
                EntityRecord::Removed(e) => {
                    if let Some(ec) = render_world.get_entity_mut(e) {
                        ec.despawn_recursive();
                    };
                }
            }
        }
    });
}

// TODO: directly remove matched archetype for performance
pub(crate) fn despawn_fly_entity(
    world: &mut World,
    state: &mut SystemState<Query<Entity, With<RenderFlyEntity>>>,
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

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component,
        entity::Entity,
        observer::Trigger,
        query::With,
        system::{Query, ResMut},
        world::{OnAdd, OnRemove, World},
    };

    use super::{
        entity_sync_system, EntityRecord, MainEntity, PendingSyncEntity, RenderEntity,
        SyncRenderWorld,
    };

    #[derive(Component)]
    struct RenderDataComponent;

    #[test]
    fn world_sync() {
        let mut main_world = World::new();
        let mut render_world = World::new();
        main_world.init_resource::<PendingSyncEntity>();

        main_world.observe(
            |trigger: Trigger<OnAdd, SyncRenderWorld>, mut pending: ResMut<PendingSyncEntity>| {
                pending.push(EntityRecord::Added(trigger.entity()));
            },
        );
        main_world.observe(
            |trigger: Trigger<OnRemove, SyncRenderWorld>,
             mut pending: ResMut<PendingSyncEntity>,
             query: Query<&RenderEntity>| {
                if let Ok(e) = query.get(trigger.entity()) {
                    pending.push(EntityRecord::Removed(e.id()));
                };
            },
        );

        // spawn some empty entities for test
        for _ in 0..99 {
            main_world.spawn_empty();
        }

        // spawn
        let main_entity = main_world
            .spawn(RenderDataComponent)
            // indicates that its entity needs to be Synchronized to the render world
            .insert(SyncRenderWorld)
            .id();

        entity_sync_system(&mut main_world, &mut render_world);

        let mut q = render_world.query_filtered::<Entity, With<MainEntity>>();

        // Only one synchronized entity
        assert!(q.iter(&render_world).count() == 1);

        let render_entity = q.get_single(&render_world).unwrap();
        let render_entity_component = main_world.get::<RenderEntity>(main_entity).unwrap();

        assert!(render_entity_component.id() == render_entity);

        let main_entity_component = render_world
            .get::<MainEntity>(render_entity_component.id())
            .unwrap();

        assert!(main_entity_component.id() == main_entity);

        // despawn
        main_world.despawn(main_entity);

        entity_sync_system(&mut main_world, &mut render_world);

        // Only one synchronized entity
        assert!(q.iter(&render_world).count() == 0);
    }
}
