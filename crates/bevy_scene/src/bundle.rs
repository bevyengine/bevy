use bevy_asset::Handle;
use bevy_ecs::{
    bundle::Bundle, change_detection::ResMut, entity::Entity, prelude::Changed, system::Query,
};
use bevy_transform::components::{GlobalTransform, Transform};

use crate::{DynamicScene, InstanceId, Scene, SceneSpawner};

#[derive(Default, Bundle)]
pub struct SceneBundle {
    pub scene: Handle<Scene>,
    pub instance_id: Option<InstanceId>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
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
        if let Some(id) = instance.replace(scene_spawner.spawn_as_child(scene.clone(), entity)) {
            scene_spawner.despawn_instance(id);
        }
    }
}

#[derive(Default, Bundle)]
pub struct DynamicSceneBundle {
    pub scene: Handle<DynamicScene>,
    pub instance_id: Option<InstanceId>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

#[allow(clippy::type_complexity)]
pub fn dynamic_scene_bundle_spawner(
    mut dynamic_scene_to_spawn: Query<
        (Entity, &Handle<DynamicScene>, &mut Option<InstanceId>),
        Changed<Handle<DynamicScene>>,
    >,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    for (entity, dynamic_scene, mut instance) in dynamic_scene_to_spawn.iter_mut() {
        if let Some(id) =
            instance.replace(scene_spawner.spawn_dynamic_as_child(dynamic_scene.clone(), entity))
        {
            scene_spawner.despawn_instance(id);
        }
    }
}
