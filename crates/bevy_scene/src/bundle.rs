use bevy_asset::Handle;
use bevy_ecs::{
    bundle::Bundle, change_detection::ResMut, entity::Entity, prelude::Changed, system::Query,
};
use bevy_transform::components::{GlobalTransform, Transform};

use crate::{InstanceId, Scene, SceneSpawner};

#[derive(Default, Bundle)]
pub struct SceneBundle {
    pub scene: Handle<Scene>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub instance_id: Option<InstanceId>,
}

#[allow(clippy::type_complexity)]
pub fn scene_bundle_spawner(
    mut scene_to_spawn: Query<
        (Entity, &Handle<Scene>, &mut Option<InstanceId>),
        Changed<Handle<Scene>>,
    >,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    for (entity, scene, mut instance) in scene_to_spawn.iter_mut() {
        if let Some(id) = *instance {
            scene_spawner.despawn_instance(id);
        }
        *instance = Some(scene_spawner.spawn_as_child(scene.clone(), entity));
    }
}
