use std::ops::{Deref, DerefMut};

use bevy_asset::Handle;
use bevy_ecs::{
    bundle::Bundle,
    change_detection::ResMut,
    entity::Entity,
    prelude::{Changed, Component},
    system::{Commands, Query},
};
use bevy_transform::components::{GlobalTransform, Transform};

use crate::{DynamicScene, InstanceId, Scene, SceneSpawner};

/// [`InstanceId`] of a spawned scene. It can be used with the [`SceneSpawner`] to
/// interact with the spawned scene.
#[derive(Component)]
pub struct SceneInstance(InstanceId);

impl Deref for SceneInstance {
    type Target = InstanceId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SceneInstance {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.0
    }
}

/// A component bundle for a [`Scene`] root.
///
/// Once the scene is spawned, the entity will have a [`SceneInstance`] component.
#[derive(Default, Bundle)]
pub struct SceneBundle {
    /// Handle to the scene to spawn
    pub scene: Handle<Scene>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

/// System that will spawn scenes from [`SceneBundle`].
#[allow(clippy::type_complexity)]
pub fn scene_bundle_spawner(
    mut commands: Commands,
    mut scene_to_spawn: Query<
        (Entity, &Handle<Scene>, Option<&mut SceneInstance>),
        Changed<Handle<Scene>>,
    >,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    for (entity, scene, instance) in scene_to_spawn.iter_mut() {
        let new_instance = scene_spawner.spawn_as_child(scene.clone(), entity);
        if let Some(mut old_instance) = instance {
            scene_spawner.despawn_instance(**old_instance);
            *old_instance = SceneInstance(new_instance);
        } else {
            commands.entity(entity).insert(SceneInstance(new_instance));
        }
    }
}

/// A component bundle for a [`DynamicScene`] root.
///
/// Once the dynamic scene is spawned, the entity will have a [`SceneInstance`] component.
#[derive(Default, Bundle)]
pub struct DynamicSceneBundle {
    /// Handle to the scene to spawn
    pub scene: Handle<DynamicScene>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

/// System that will spawn scenes from [`DynamicSceneBundle`].
#[allow(clippy::type_complexity)]
pub fn dynamic_scene_bundle_spawner(
    mut commands: Commands,
    mut dynamic_scene_to_spawn: Query<
        (Entity, &Handle<DynamicScene>, Option<&mut SceneInstance>),
        Changed<Handle<DynamicScene>>,
    >,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    for (entity, dynamic_scene, instance) in dynamic_scene_to_spawn.iter_mut() {
        let new_instance = scene_spawner.spawn_dynamic_as_child(dynamic_scene.clone(), entity);
        if let Some(mut old_instance) = instance {
            scene_spawner.despawn_instance(**old_instance);
            *old_instance = SceneInstance(new_instance);
        } else {
            commands.entity(entity).insert(SceneInstance(new_instance));
        }
    }
}
