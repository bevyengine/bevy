use std::marker::PhantomData;

use bevy_app::Plugin;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    bundle::Bundle,
    change_detection::DetectChanges,
    component::{Component, ComponentHooks, StorageType},
    entity::{Entity, EntityHashSet},
    observer::Trigger,
    query::Has,
    system::{Commands, Query, ResMut, Resource},
    world::{self, Mut, OnAdd, OnRemove, World},
};
use bevy_hierarchy::DespawnRecursiveExt;

#[derive(Component, Deref, Default, Clone, Debug)]
#[component(storage = "SparseSet")]
pub struct RenderWorldSyncEntity(Option<Entity>);

impl RenderWorldSyncEntity {
    pub fn entity(&self) -> Option<Entity> {
        self.0
    }
}
enum EntityRecord {
    Added(Entity),
    Removed(Option<Entity>),
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
                    println!("sync added :main [{:?}],render:[{:?}]", e, id);
                    world.get_mut::<RenderWorldSyncEntity>(e).map(|mut e| {
                        e.0 = Some(id);
                    });
                }
                EntityRecord::Removed(e) => {
                    if let Some(e) = e {
                        render_world
                            .get_entity_mut(e)
                            .map(|ec| ec.despawn_recursive());
                    }
                }
            }
        }
    });
}

pub(crate) struct WorldSyncPlugin;

impl Plugin for WorldSyncPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<PendingSyncEntity>();
        app.observe(
            |trigger: Trigger<OnAdd, RenderWorldSyncEntity>,
             mut pending: ResMut<PendingSyncEntity>| {
                pending.push(EntityRecord::Added(trigger.entity()));
            },
        );
        app.observe(
            |trigger: Trigger<OnRemove, RenderWorldSyncEntity>,
             mut pending: ResMut<PendingSyncEntity>,
             query: Query<&RenderWorldSyncEntity>| {
                let render_entity = query.get(trigger.entity()).unwrap().0;
                pending.push(EntityRecord::Removed(render_entity));
            },
        );
    }
}
