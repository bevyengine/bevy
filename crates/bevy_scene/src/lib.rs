#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Provides scene definition, instantiation and serialization/deserialization.
//!
//! Scenes are collections of entities and their associated components that can be
//! instantiated or removed from a world to allow composition. Scenes can be serialized/deserialized,
//! for example to save part of the world state to a file.

extern crate alloc;

mod components;
mod dynamic_scene;
mod dynamic_scene_builder;
mod reflect_utils;
mod scene;
mod scene_filter;
mod scene_loader;
mod scene_spawner;

#[cfg(feature = "serialize")]
pub mod serde;

/// Rusty Object Notation, a crate used to serialize and deserialize bevy scenes.
pub use bevy_asset::ron;

pub use components::*;
pub use dynamic_scene::*;
pub use dynamic_scene_builder::*;
pub use scene::*;
pub use scene_filter::*;
pub use scene_loader::*;
pub use scene_spawner::*;

/// The scene prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        DynamicScene, DynamicSceneBuilder, DynamicSceneRoot, Scene, SceneFilter, SceneRoot,
        SceneSpawner,
    };
}

use bevy_app::prelude::*;

#[cfg(feature = "serialize")]
use {bevy_asset::AssetApp, bevy_ecs::schedule::IntoScheduleConfigs};

/// Plugin that provides scene functionality to an [`App`].
#[derive(Default)]
pub struct ScenePlugin;

#[cfg(feature = "serialize")]
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<DynamicScene>()
            .init_asset::<Scene>()
            .init_asset_loader::<SceneLoader>()
            .init_resource::<SceneSpawner>()
            .add_systems(SpawnScene, (scene_spawner, scene_spawner_system).chain());

        // Register component hooks for DynamicSceneRoot
        app.world_mut()
            .register_component_hooks::<DynamicSceneRoot>()
            .on_remove(|mut world, context| {
                let Some(handle) = world.get::<DynamicSceneRoot>(context.entity) else {
                    return;
                };
                let id = handle.id();
                if let Some(&SceneInstance(scene_instance)) =
                    world.get::<SceneInstance>(context.entity)
                {
                    let Some(mut scene_spawner) = world.get_resource_mut::<SceneSpawner>() else {
                        return;
                    };
                    if let Some(instance_ids) = scene_spawner.spawned_dynamic_scenes.get_mut(&id) {
                        instance_ids.remove(&scene_instance);
                    }
                    scene_spawner.unregister_instance(scene_instance);
                }
            });

        // Register component hooks for SceneRoot
        app.world_mut()
            .register_component_hooks::<SceneRoot>()
            .on_remove(|mut world, context| {
                let Some(handle) = world.get::<SceneRoot>(context.entity) else {
                    return;
                };
                let id = handle.id();
                if let Some(&SceneInstance(scene_instance)) =
                    world.get::<SceneInstance>(context.entity)
                {
                    let Some(mut scene_spawner) = world.get_resource_mut::<SceneSpawner>() else {
                        return;
                    };
                    if let Some(instance_ids) = scene_spawner.spawned_scenes.get_mut(&id) {
                        instance_ids.remove(&scene_instance);
                    }
                    scene_spawner.unregister_instance(scene_instance);
                }
            });
    }
}

#[cfg(not(feature = "serialize"))]
impl Plugin for ScenePlugin {
    fn build(&self, _: &mut App) {}
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_asset::{AssetPlugin, Assets};
    use bevy_ecs::{
        component::Component,
        entity::Entity,
        entity_disabling::Internal,
        hierarchy::{ChildOf, Children},
        query::Allows,
        reflect::{AppTypeRegistry, ReflectComponent},
        world::World,
    };
    use bevy_reflect::Reflect;

    use crate::{
        DynamicScene, DynamicSceneBuilder, DynamicSceneRoot, Scene, ScenePlugin, SceneRoot,
    };

    #[derive(Component, Reflect, PartialEq, Debug)]
    #[reflect(Component)]
    struct Circle {
        radius: f32,
    }

    #[derive(Component, Reflect, PartialEq, Debug)]
    #[reflect(Component)]
    struct Rectangle {
        width: f32,
        height: f32,
    }

    #[derive(Component, Reflect, PartialEq, Debug)]
    #[reflect(Component)]
    struct Triangle {
        base: f32,
        height: f32,
    }

    #[derive(Component, Reflect)]
    #[reflect(Component)]
    struct FinishLine;

    #[test]
    fn scene_spawns_and_respawns_after_change() {
        let mut app = App::new();

        app.add_plugins((AssetPlugin::default(), ScenePlugin))
            .register_type::<Circle>()
            .register_type::<Rectangle>()
            .register_type::<Triangle>()
            .register_type::<FinishLine>();

        let scene_handle = app
            .world_mut()
            .resource_mut::<Assets<Scene>>()
            .reserve_handle();

        let scene_entity = app.world_mut().spawn(SceneRoot(scene_handle.clone())).id();
        app.update();

        assert!(app.world().entity(scene_entity).get::<Children>().is_none());

        let mut scene_1 = Scene {
            world: World::new(),
        };
        let root = scene_1.world.spawn_empty().id();
        scene_1.world.spawn((
            Rectangle {
                width: 10.0,
                height: 5.0,
            },
            FinishLine,
            ChildOf(root),
        ));
        scene_1.world.spawn((Circle { radius: 7.0 }, ChildOf(root)));

        app.world_mut()
            .resource_mut::<Assets<Scene>>()
            .insert(&scene_handle, scene_1)
            .unwrap();

        app.update();

        let child_root = app
            .world()
            .entity(scene_entity)
            .get::<Children>()
            .and_then(|children| children.first().cloned())
            .expect("There should be exactly one child on the scene root");
        let children = app
            .world()
            .entity(child_root)
            .get::<Children>()
            .expect("The child of the scene root should itself have 2 children");
        assert_eq!(children.len(), 2);

        let finish_line = app.world().entity(children[0]);
        assert_eq!(finish_line.archetype().component_count(), 3);
        let (rectangle, _, child_of) =
            finish_line.components::<(&Rectangle, &FinishLine, &ChildOf)>();
        assert_eq!(
            rectangle,
            &Rectangle {
                width: 10.0,
                height: 5.0,
            }
        );
        assert_eq!(child_of.0, child_root);

        let circle = app.world().entity(children[1]);
        assert_eq!(circle.archetype().component_count(), 2);
        let (circle, child_of) = circle.components::<(&Circle, &ChildOf)>();
        assert_eq!(circle, &Circle { radius: 7.0 });
        assert_eq!(child_of.0, child_root);

        // Now that we know our scene contains exactly what we expect, we will change the scene
        // asset and ensure it contains the new scene results.

        let mut scene_2 = Scene {
            world: World::new(),
        };
        let root = scene_2.world.spawn_empty().id();
        scene_2.world.spawn((
            Triangle {
                base: 1.0,
                height: 2.0,
            },
            ChildOf(root),
        ));

        app.world_mut()
            .resource_mut::<Assets<Scene>>()
            .insert(&scene_handle, scene_2)
            .unwrap();

        app.update();
        app.update();

        let child_root = app
            .world()
            .entity(scene_entity)
            .get::<Children>()
            .and_then(|children| children.first().cloned())
            .expect("There should be exactly one child on the scene root");
        let children = app
            .world()
            .entity(child_root)
            .get::<Children>()
            .expect("The child of the scene root should itself have 2 children");
        assert_eq!(children.len(), 1);

        let triangle = app.world().entity(children[0]);
        assert_eq!(triangle.archetype().component_count(), 2);
        let (triangle, child_of) = triangle.components::<(&Triangle, &ChildOf)>();
        assert_eq!(
            triangle,
            &Triangle {
                base: 1.0,
                height: 2.0,
            }
        );
        assert_eq!(child_of.0, child_root);
    }

    #[test]
    fn dynamic_scene_spawns_and_respawns_after_change() {
        let mut app = App::new();

        app.add_plugins((AssetPlugin::default(), ScenePlugin))
            .register_type::<Circle>()
            .register_type::<Rectangle>()
            .register_type::<Triangle>()
            .register_type::<FinishLine>();

        let scene_handle = app
            .world_mut()
            .resource_mut::<Assets<DynamicScene>>()
            .reserve_handle();

        let scene_entity = app
            .world_mut()
            .spawn(DynamicSceneRoot(scene_handle.clone()))
            .id();
        app.update();

        assert!(app.world().entity(scene_entity).get::<Children>().is_none());

        let create_dynamic_scene = |mut scene: Scene, world: &World| {
            scene
                .world
                .insert_resource(world.resource::<AppTypeRegistry>().clone());
            let entities: Vec<Entity> = scene
                .world
                .query_filtered::<Entity, Allows<Internal>>()
                .iter(&scene.world)
                .collect();
            DynamicSceneBuilder::from_world(&scene.world)
                .extract_entities(entities.into_iter())
                .build()
        };

        let mut scene_1 = Scene {
            world: World::new(),
        };
        let root = scene_1.world.spawn_empty().id();
        scene_1.world.spawn((
            Rectangle {
                width: 10.0,
                height: 5.0,
            },
            FinishLine,
            ChildOf(root),
        ));
        scene_1.world.spawn((Circle { radius: 7.0 }, ChildOf(root)));

        let scene_1 = create_dynamic_scene(scene_1, app.world());
        app.world_mut()
            .resource_mut::<Assets<DynamicScene>>()
            .insert(&scene_handle, scene_1)
            .unwrap();

        app.update();

        let child_root = app
            .world()
            .entity(scene_entity)
            .get::<Children>()
            .and_then(|children| children.first().cloned())
            .expect("There should be exactly one child on the scene root");
        let children = app
            .world()
            .entity(child_root)
            .get::<Children>()
            .expect("The child of the scene root should itself have 2 children");
        assert_eq!(children.len(), 2);

        let finish_line = app.world().entity(children[0]);
        assert_eq!(finish_line.archetype().component_count(), 3);
        let (rectangle, _, child_of) =
            finish_line.components::<(&Rectangle, &FinishLine, &ChildOf)>();
        assert_eq!(
            rectangle,
            &Rectangle {
                width: 10.0,
                height: 5.0,
            }
        );
        assert_eq!(child_of.0, child_root);

        let circle = app.world().entity(children[1]);
        assert_eq!(circle.archetype().component_count(), 2);
        let (circle, child_of) = circle.components::<(&Circle, &ChildOf)>();
        assert_eq!(circle, &Circle { radius: 7.0 });
        assert_eq!(child_of.0, child_root);

        // Now that we know our scene contains exactly what we expect, we will change the scene
        // asset and ensure it contains the new scene results.

        let mut scene_2 = Scene {
            world: World::new(),
        };
        let root = scene_2.world.spawn_empty().id();
        scene_2.world.spawn((
            Triangle {
                base: 1.0,
                height: 2.0,
            },
            ChildOf(root),
        ));

        let scene_2 = create_dynamic_scene(scene_2, app.world());

        app.world_mut()
            .resource_mut::<Assets<DynamicScene>>()
            .insert(&scene_handle, scene_2)
            .unwrap();

        app.update();
        app.update();

        let child_root = app
            .world()
            .entity(scene_entity)
            .get::<Children>()
            .and_then(|children| children.first().cloned())
            .expect("There should be exactly one child on the scene root");
        let children = app
            .world()
            .entity(child_root)
            .get::<Children>()
            .expect("The child of the scene root should itself have 2 children");
        assert_eq!(children.len(), 1);

        let triangle = app.world().entity(children[0]);
        assert_eq!(triangle.archetype().component_count(), 2);
        let (triangle, child_of) = triangle.components::<(&Triangle, &ChildOf)>();
        assert_eq!(
            triangle,
            &Triangle {
                base: 1.0,
                height: 2.0,
            }
        );
        assert_eq!(child_of.0, child_root);
    }
}
