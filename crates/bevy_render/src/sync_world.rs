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
use bevy_ecs::entity::EntityHash;
use bevy_reflect::Reflect;
use bevy_utils::hashbrown;

/// A plugin that synchronizes entities with [`SyncToRenderWorld`] between the main world and the render world.
///
/// All entities with the [`SyncToRenderWorld`] component are kept in sync. It
/// is automatically added as a required component by [`ExtractComponentPlugin`]
/// and [`SyncComponentPlugin`], so it doesn't need to be added manually when
/// spawning or as a required component when either of these plugins are used.
///
/// # Implementation
///
/// Bevy's renderer is architected independently from the main app.
/// It operates in its own separate ECS [`World`], so the renderer logic can run in parallel with the main world logic.
/// This is called "Pipelined Rendering", see [`PipelinedRenderingPlugin`] for more information.
///
/// [`SyncWorldPlugin`] is the first thing that runs every frame and it maintains an entity-to-entity mapping
/// between the main world and the render world.
/// It does so by spawning and despawning entities in the render world, to match spawned and despawned entities in the main world.
/// The link between synced entities is maintained by the [`RenderEntity`] and [`MainEntity`] components.
/// The [`RenderEntity`] contains the corresponding render world entity of a main world entity, while [`MainEntity`] contains
/// the corresponding main world entity of a render world entity.
/// The entities can be accessed by calling `.id()` on either component.
///
/// Synchronization is necessary preparation for extraction ([`ExtractSchedule`](crate::ExtractSchedule)), which copies over component data from the main
/// to the render world for these entities.
///
/// ```text
/// |--------------------------------------------------------------------|
/// |      |         |          Main world update                        |
/// | sync | extract |---------------------------------------------------|
/// |      |         |         Render world update                       |
/// |--------------------------------------------------------------------|
/// ```
///
/// An example for synchronized main entities 1v1 and 18v1
///
/// ```text
/// |---------------------------Main World------------------------------|
/// |  Entity  |                    Component                           |
/// |-------------------------------------------------------------------|
/// | ID: 1v1  | PointLight | RenderEntity(ID: 3V1) | SyncToRenderWorld |
/// | ID: 18v1 | PointLight | RenderEntity(ID: 5V1) | SyncToRenderWorld |
/// |-------------------------------------------------------------------|
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
/// Note that this effectively establishes a link between the main world entity and the render world entity.
/// Not every entity needs to be synchronized, however; only entities with the [`SyncToRenderWorld`] component are synced.
/// Adding [`SyncToRenderWorld`] to a main world component will establish such a link.
/// Once a synchronized main entity is despawned, its corresponding render entity will be automatically
/// despawned in the next `sync`.
///
/// The sync step does not copy any of component data between worlds, since its often not necessary to transfer over all
/// the components of a main world entity.
/// The render world probably cares about a `Position` component, but not a `Velocity` component.
/// The extraction happens in its own step, independently from, and after synchronization.
///
/// Moreover, [`SyncWorldPlugin`] only synchronizes *entities*. [`RenderAsset`](crate::render_asset::RenderAsset)s like meshes and textures are handled
/// differently.
///
/// [`PipelinedRenderingPlugin`]: crate::pipelined_rendering::PipelinedRenderingPlugin
/// [`ExtractComponentPlugin`]: crate::extract_component::ExtractComponentPlugin
/// [`SyncComponentPlugin`]: crate::sync_component::SyncComponentPlugin
#[derive(Default)]
pub struct SyncWorldPlugin;

impl Plugin for SyncWorldPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<PendingSyncEntity>();
        app.observe(
            |trigger: Trigger<OnAdd, SyncToRenderWorld>, mut pending: ResMut<PendingSyncEntity>| {
                pending.push(EntityRecord::Added(trigger.entity()));
            },
        );
        app.observe(
            |trigger: Trigger<OnRemove, SyncToRenderWorld>,
             mut pending: ResMut<PendingSyncEntity>,
             query: Query<&RenderEntity>| {
                if let Ok(e) = query.get(trigger.entity()) {
                    pending.push(EntityRecord::Removed(*e));
                };
            },
        );
    }
}
/// Marker component that indicates that its entity needs to be synchronized to the render world.
///
/// This component is automatically added as a required component by [`ExtractComponentPlugin`] and [`SyncComponentPlugin`].
/// For more information see [`SyncWorldPlugin`].
///
/// NOTE: This component should persist throughout the entity's entire lifecycle.
/// If this component is removed from its entity, the entity will be despawned.
///
/// [`ExtractComponentPlugin`]: crate::extract_component::ExtractComponentPlugin
/// [`SyncComponentPlugin`]: crate::sync_component::SyncComponentPlugin
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect[Component]]
#[component(storage = "SparseSet")]
pub struct SyncToRenderWorld;

/// Component added on the main world entities that are synced to the Render World in order to keep track of the corresponding render world entity.
///
/// Can also be used as a newtype wrapper for render world entities.
#[derive(Component, Deref, Clone, Debug, Copy)]
pub struct RenderEntity(Entity);
impl RenderEntity {
    #[inline]
    pub fn id(&self) -> Entity {
        self.0
    }
}

/// Component added on the render world entities to keep track of the corresponding main world entity.
///
/// Can also be used as a newtype wrapper for main world entities.
#[derive(Component, Deref, Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct MainEntity(Entity);
impl MainEntity {
    #[inline]
    pub fn id(&self) -> Entity {
        self.0
    }
}

impl From<Entity> for MainEntity {
    fn from(entity: Entity) -> Self {
        MainEntity(entity)
    }
}

/// A [`HashMap`](hashbrown::HashMap) pre-configured to use [`EntityHash`] hashing with a [`MainEntity`].
pub type MainEntityHashMap<V> = hashbrown::HashMap<MainEntity, V, EntityHash>;

/// A [`HashSet`](hashbrown::HashSet) pre-configured to use [`EntityHash`] hashing with a [`MainEntity`]..
pub type MainEntityHashSet = hashbrown::HashSet<MainEntity, EntityHash>;

/// Marker component that indicates that its entity needs to be despawned at the end of the frame.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[component(storage = "SparseSet")]
pub struct TemporaryRenderEntity;

/// A record enum to what entities with [`SyncToRenderWorld`] have been added or removed.
#[derive(Debug)]
pub(crate) enum EntityRecord {
    /// When an entity is spawned on the main world, notify the render world so that it can spawn a corresponding
    /// entity. This contains the main world entity.
    Added(Entity),
    /// When an entity is despawned on the main world, notify the render world so that the corresponding entity can be
    /// despawned. This contains the render world entity.
    Removed(RenderEntity),
    /// When a component is removed from an entity, notify the render world so that the corresponding component can be
    /// removed. This contains the main world entity.
    ComponentRemoved(Entity),
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
                    if let Ok(mut main_entity) = world.get_entity_mut(e) {
                        match main_entity.entry::<RenderEntity>() {
                            bevy_ecs::world::Entry::Occupied(_) => {
                                panic!("Attempting to synchronize an entity that has already been synchronized!");
                            }
                            bevy_ecs::world::Entry::Vacant(entry) => {
                                let id = render_world.spawn(MainEntity(e)).id();

                                entry.insert(RenderEntity(id));
                            }
                        };
                    }
                }
                EntityRecord::Removed(render_entity) => {
                    if let Ok(ec) = render_world.get_entity_mut(render_entity.id()) {
                        ec.despawn();
                    };
                }
                EntityRecord::ComponentRemoved(main_entity) => {
                    let Some(mut render_entity) = world.get_mut::<RenderEntity>(main_entity) else {
                        continue;
                    };
                    if let Ok(render_world_entity) = render_world.get_entity_mut(render_entity.id()) {
                        // In order to handle components that extract to derived components, we clear the entity
                        // and let the extraction system re-add the components.
                        render_world_entity.despawn();

                        let id = render_world.spawn(MainEntity(main_entity)).id();
                        render_entity.0 = id;
                    }
                },
            }
        }
    });
}

pub(crate) fn despawn_temporary_render_entities(
    world: &mut World,
    state: &mut SystemState<Query<Entity, With<TemporaryRenderEntity>>>,
    mut local: Local<Vec<Entity>>,
) {
    let query = state.get(world);

    local.extend(query.iter());

    // Ensure next frame allocation keeps order
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
        SyncToRenderWorld,
    };

    #[derive(Component)]
    struct RenderDataComponent;

    #[test]
    fn sync_world() {
        let mut main_world = World::new();
        let mut render_world = World::new();
        main_world.init_resource::<PendingSyncEntity>();

        main_world.observe(
            |trigger: Trigger<OnAdd, SyncToRenderWorld>, mut pending: ResMut<PendingSyncEntity>| {
                pending.push(EntityRecord::Added(trigger.entity()));
            },
        );
        main_world.observe(
            |trigger: Trigger<OnRemove, SyncToRenderWorld>,
             mut pending: ResMut<PendingSyncEntity>,
             query: Query<&RenderEntity>| {
                if let Ok(e) = query.get(trigger.entity()) {
                    pending.push(EntityRecord::Removed(*e));
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
            // indicates that its entity needs to be synchronized to the render world
            .insert(SyncToRenderWorld)
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
