use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    bundle::Bundle,
    change_detection::ResMut,
    entity::Entity,
    prelude::{Changed, Component, Without},
    system::{Commands, Query},
};
use bevy_render::prelude::{ComputedVisibility, Visibility};
use bevy_transform::components::{GlobalTransform, Transform};

use crate::{DynamicScene, InstanceId, Scene, SceneSpawner};

/// [`InstanceId`] of a spawned scene. It can be used with the [`SceneSpawner`] to
/// interact with the spawned scene.
#[derive(Component, Deref, DerefMut)]
pub struct SceneInstance(InstanceId);

/// A component bundle for a [`Scene`] root.
///
/// The scene from `scene` will be spawn as a child of the entity with this component.
/// Once it's spawned, the entity will have a [`SceneInstance`] component.
#[derive(Default, Bundle)]
pub struct SceneBundle {
    /// Handle to the scene to spawn
    pub scene: Handle<Scene>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

/// A component bundle for a [`DynamicScene`] root.
///
/// The dynamic scene from `scene` will be spawn as a child of the entity with this component.
/// Once it's spawned, the entity will have a [`SceneInstance`] component.
#[derive(Default, Bundle)]
pub struct DynamicSceneBundle {
    /// Handle to the scene to spawn
    pub scene: Handle<DynamicScene>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

/// System that will spawn scenes from [`SceneBundle`].
pub fn scene_spawner(
    mut commands: Commands,
    mut scene_to_spawn: Query<
        (Entity, &Handle<Scene>, Option<&mut SceneInstance>),
        (Changed<Handle<Scene>>, Without<Handle<DynamicScene>>),
    >,
    mut dynamic_scene_to_spawn: Query<
        (Entity, &Handle<DynamicScene>, Option<&mut SceneInstance>),
        (Changed<Handle<DynamicScene>>, Without<Handle<Scene>>),
    >,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    for (entity, scene, instance) in &mut scene_to_spawn {
        let new_instance = scene_spawner.spawn_as_child(scene.clone(), entity);
        if let Some(mut old_instance) = instance {
            scene_spawner.despawn_instance(**old_instance);
            *old_instance = SceneInstance(new_instance);
        } else {
            commands.entity(entity).insert(SceneInstance(new_instance));
        }
    }
    for (entity, dynamic_scene, instance) in &mut dynamic_scene_to_spawn {
        let new_instance = scene_spawner.spawn_dynamic_as_child(dynamic_scene.clone(), entity);
        if let Some(mut old_instance) = instance {
            scene_spawner.despawn_instance(**old_instance);
            *old_instance = SceneInstance(new_instance);
        } else {
            commands.entity(entity).insert(SceneInstance(new_instance));
        }
    }
}
