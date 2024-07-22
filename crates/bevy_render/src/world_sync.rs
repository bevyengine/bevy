use bevy_app::Plugin;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    observer::Trigger,
    query::With,
    system::{Commands, Query, ResMut, Resource},
    world::{Mut, OnAdd, OnRemove, World},
};
use bevy_hierarchy::DespawnRecursiveExt;

// marker component to indicate that its entity needs to be synchronized between RenderWorld and MainWorld
#[derive(Component, Clone, Debug, Default)]
pub struct ToRenderWorld;

#[derive(Component, Deref, Clone, Debug)]
pub struct RenderEntity(Entity);

// marker component that its entity needs to be despawned per frame.
#[derive(Component, Clone, Debug, Default)]
pub struct RenderFlyEntity;

impl RenderEntity {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

enum EntityRecord {
    Added(Entity),
    Removed(Entity),
}

#[derive(Resource, Default, Deref, DerefMut)]
struct PendingSyncEntity {
    records: Vec<EntityRecord>,
}

pub(crate) fn entity_sync_system(main_world: &mut World, render_world: &mut World) {
    main_world.resource_scope(|world, mut pending: Mut<PendingSyncEntity>| {
        for record in pending.drain(..) {
            match record {
                EntityRecord::Added(e) => {
                    let id = render_world.spawn_empty().id();
                    // println!("sync added :main [{:?}],render:[{:?}]", e, id);
                    if let Some(mut entity) = world.get_entity_mut(e) {
                        entity.insert(RenderEntity(id));
                    }
                }
                EntityRecord::Removed(e) => {
                    render_world
                        .get_entity_mut(e)
                        .map(|ec| ec.despawn_recursive());
                }
            }
        }
    });
}

pub(crate) fn despawn_fly_entity(
    mut command: Commands,
    query: Query<Entity, With<RenderFlyEntity>>,
) {
    query.iter().for_each(|e| {
        // TODO : performant delete
        command.entity(e).despawn_recursive();
    })
}
pub(crate) struct WorldSyncPlugin;

impl Plugin for WorldSyncPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<PendingSyncEntity>();
        app.observe(
            |trigger: Trigger<OnAdd, ToRenderWorld>, mut pending: ResMut<PendingSyncEntity>| {
                pending.push(EntityRecord::Added(trigger.entity()));
            },
        );
        app.observe(
            |trigger: Trigger<OnRemove, ToRenderWorld>, mut pending: ResMut<PendingSyncEntity>| {
                pending.push(EntityRecord::Removed(trigger.entity()));
            },
        );
    }
}
