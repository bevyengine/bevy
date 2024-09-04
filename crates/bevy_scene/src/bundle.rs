use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    bundle::Bundle,
    change_detection::ResMut,
    entity::Entity,
    prelude::{Changed, Component, Without},
    system::{Commands, Query},
};
#[cfg(feature = "bevy_render")]
use bevy_render::prelude::{InheritedVisibility, ViewVisibility, Visibility};
use bevy_transform::components::{GlobalTransform, Transform};

use crate::{DynamicScene, InstanceId, Scene, SceneSpawner};

/// [`InstanceId`] of a spawned scene. It can be used with the [`SceneSpawner`] to
/// interact with the spawned scene.
#[derive(Component, Deref)]
pub struct SceneInstance(pub(crate) InstanceId);

/// Adding this component will spawn the scene as a child of that entity.
/// Once it's spawned, the entity will have a [`SceneInstance`] component.
#[derive(Component, Default, Clone, Deref, DerefMut)]
pub struct SceneHandle(pub Handle<Scene>);

impl From<Handle<Scene>> for SceneHandle {
    fn from(value: Handle<Scene>) -> Self {
        SceneHandle(value)
    }
}

impl From<SceneHandle> for Handle<Scene> {
    fn from(value: SceneHandle) -> Self {
        value.0
    }
}

/// Adding this component will spawn the scene as a child of that entity.
/// Once it's spawned, the entity will have a [`SceneInstance`] component.
#[derive(Component, Default, Clone, Deref, DerefMut)]
pub struct DynamicSceneHandle(pub Handle<DynamicScene>);

impl From<Handle<DynamicScene>> for DynamicSceneHandle {
    fn from(value: Handle<DynamicScene>) -> Self {
        DynamicSceneHandle(value)
    }
}

impl From<DynamicSceneHandle> for Handle<DynamicScene> {
    fn from(value: DynamicSceneHandle) -> Self {
        value.0
    }
}

/// A component bundle for a [`Scene`] root.
///
/// The scene from `scene` will be spawned as a child of the entity with this component.
/// Once it's spawned, the entity will have a [`SceneInstance`] component.
#[derive(Default, Bundle, Clone)]
pub struct SceneBundle {
    /// Handle to the scene to spawn.
    pub scene: SceneHandle,
    /// Transform of the scene root entity.
    pub transform: Transform,
    /// Global transform of the scene root entity.
    pub global_transform: GlobalTransform,

    /// User-driven visibility of the scene root entity.
    #[cfg(feature = "bevy_render")]
    pub visibility: Visibility,
    /// Inherited visibility of the scene root entity.
    #[cfg(feature = "bevy_render")]
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed visibility of the scene root entity for rendering.
    #[cfg(feature = "bevy_render")]
    pub view_visibility: ViewVisibility,
}

/// A component bundle for a [`DynamicScene`] root.
///
/// The dynamic scene from `scene` will be spawn as a child of the entity with this component.
/// Once it's spawned, the entity will have a [`SceneInstance`] component.
#[derive(Default, Bundle, Clone)]
pub struct DynamicSceneBundle {
    /// Handle to the scene to spawn.
    pub scene: DynamicSceneHandle,
    /// Transform of the scene root entity.
    pub transform: Transform,
    /// Global transform of the scene root entity.
    pub global_transform: GlobalTransform,

    /// User-driven visibility of the scene root entity.
    #[cfg(feature = "bevy_render")]
    pub visibility: Visibility,
    /// Inherited visibility of the scene root entity.
    #[cfg(feature = "bevy_render")]
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed visibility of the scene root entity for rendering.
    #[cfg(feature = "bevy_render")]
    pub view_visibility: ViewVisibility,
}

/// System that will spawn scenes from [`SceneBundle`].
pub fn scene_spawner(
    mut commands: Commands,
    mut scene_to_spawn: Query<
        (Entity, &SceneHandle, Option<&mut SceneInstance>),
        (Changed<SceneHandle>, Without<DynamicSceneHandle>),
    >,
    mut dynamic_scene_to_spawn: Query<
        (Entity, &DynamicSceneHandle, Option<&mut SceneInstance>),
        (Changed<DynamicSceneHandle>, Without<SceneHandle>),
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

#[cfg(test)]
mod tests {
    use crate::{DynamicScene, DynamicSceneBundle, ScenePlugin, SceneSpawner};
    use bevy_app::{App, ScheduleRunnerPlugin};
    use bevy_asset::{AssetPlugin, Assets};
    use bevy_ecs::component::Component;
    use bevy_ecs::entity::Entity;
    use bevy_ecs::prelude::{AppTypeRegistry, ReflectComponent, World};
    use bevy_hierarchy::{Children, HierarchyPlugin};
    use bevy_reflect::Reflect;
    use bevy_utils::default;

    #[derive(Component, Reflect, Default)]
    #[reflect(Component)]
    struct ComponentA {
        pub x: f32,
        pub y: f32,
    }

    #[test]
    fn spawn_and_delete() {
        let mut app = App::new();

        app.add_plugins(ScheduleRunnerPlugin::default())
            .add_plugins(HierarchyPlugin)
            .add_plugins(AssetPlugin::default())
            .add_plugins(ScenePlugin)
            .register_type::<ComponentA>();
        app.update();

        let mut scene_world = World::new();

        // create a new DynamicScene manually
        let type_registry = app.world().resource::<AppTypeRegistry>().clone();
        scene_world.insert_resource(type_registry);
        scene_world.spawn(ComponentA { x: 3.0, y: 4.0 });
        let scene = DynamicScene::from_world(&scene_world);
        let scene_handle = app
            .world_mut()
            .resource_mut::<Assets<DynamicScene>>()
            .add(scene);

        // spawn the scene as a child of `entity` using the `DynamicSceneBundle`
        let entity = app
            .world_mut()
            .spawn(DynamicSceneBundle {
                scene: scene_handle.clone().into(),
                ..default()
            })
            .id();

        // run the app's schedule once, so that the scene gets spawned
        app.update();

        // make sure that the scene was added as a child of the root entity
        let (scene_entity, scene_component_a) = app
            .world_mut()
            .query::<(Entity, &ComponentA)>()
            .single(app.world());
        assert_eq!(scene_component_a.x, 3.0);
        assert_eq!(scene_component_a.y, 4.0);
        assert_eq!(
            app.world().entity(entity).get::<Children>().unwrap().len(),
            1
        );

        // let's try to delete the scene
        let mut scene_spawner = app.world_mut().resource_mut::<SceneSpawner>();
        scene_spawner.despawn(&scene_handle);

        // run the scene spawner system to despawn the scene
        app.update();

        // the scene entity does not exist anymore
        assert!(app.world().get_entity(scene_entity).is_none());

        // the root entity does not have any children anymore
        assert!(app.world().entity(entity).get::<Children>().is_none());
    }
}
