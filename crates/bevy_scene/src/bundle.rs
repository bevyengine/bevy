use std::ops::{Deref, DerefMut};

use bevy_asset::Handle;
use bevy_ecs::{
    bundle::Bundle,
    change_detection::ResMut,
    entity::Entity,
    prelude::{Changed, Component},
    system::Query,
};
use bevy_transform::components::{GlobalTransform, Transform};

use crate::{DynamicScene, InstanceId, Scene, SceneSpawner};

/// [`InstanceId`] of a spawned scene. It can be used with the [`SceneSpawner`] to
/// interact with the spawned scene.
#[derive(Default, Component)]
pub struct HasSceneInstance(Option<InstanceId>);

impl Deref for HasSceneInstance {
    type Target = Option<InstanceId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HasSceneInstance {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.0
    }
}

/// A component bundle for a [`Scene`] root.
#[derive(Default, Bundle)]
pub struct SceneBundle {
    /// Handle to the scene to spawn
    pub scene: Handle<Scene>,
    /// [`InstanceId`] of the scene once its spawned
    pub instance_id: HasSceneInstance,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

/// System that will spawn scenes from [`SceneBundle`].
#[allow(clippy::type_complexity)]
pub fn scene_bundle_spawner(
    mut scene_to_spawn: Query<
        (Entity, &Handle<Scene>, &mut HasSceneInstance),
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

/// A component bundle for a [`DynamicScene`] root.
#[derive(Default, Bundle)]
pub struct DynamicSceneBundle {
    /// Handle to the scene to spawn
    pub scene: Handle<DynamicScene>,
    /// [`InstanceId`] of the scene once its spawned
    pub instance_id: HasSceneInstance,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

/// System that will spawn scenes from [`DynamicSceneBundle`].
#[allow(clippy::type_complexity)]
pub fn dynamic_scene_bundle_spawner(
    mut dynamic_scene_to_spawn: Query<
        (Entity, &Handle<DynamicScene>, &mut HasSceneInstance),
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
