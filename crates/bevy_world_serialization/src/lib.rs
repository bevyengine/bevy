#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Provides dynamic world definition, instantiation, and serialization/deserialization.
//!
//! [`DynamicWorld`]s are collections of entities and their associated components that can be
//! instantiated or removed from a world to allow composition. [`DynamicWorld`]s can be serialized/deserialized,
//! for example to save part of the world state to a file.

extern crate alloc;

mod components;
mod dynamic_world;
mod dynamic_world_builder;
mod reflect_utils;
mod world_asset;
mod world_asset_loader;
mod world_asset_spawner;
mod world_filter;

#[cfg(feature = "serialize")]
pub mod serde;

pub use components::*;
pub use dynamic_world::*;
pub use dynamic_world_builder::*;
pub use world_asset::*;
pub use world_asset_loader::*;
pub use world_asset_spawner::*;
pub use world_filter::*;

/// The `bevy_world_serialization` prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        DynamicWorld, DynamicWorldBuilder, DynamicWorldRoot, WorldAsset, WorldAssetRoot,
        WorldFilter, WorldInstanceSpawner,
    };
}

use bevy_app::prelude::*;

#[cfg(feature = "serialize")]
use {
    bevy_app::SceneSpawnerSystems, bevy_asset::AssetApp, bevy_ecs::schedule::IntoScheduleConfigs,
};

/// Plugin that provides world serialization functionality to an [`App`].
#[derive(Default)]
pub struct WorldSerializationPlugin;

#[cfg(feature = "serialize")]
impl Plugin for WorldSerializationPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<DynamicWorld>()
            .init_asset::<WorldAsset>()
            .init_asset_loader::<WorldAssetLoader>()
            .init_resource::<WorldInstanceSpawner>()
            .add_systems(
                SpawnScene,
                (world_instance_spawner, world_instance_spawner_system)
                    .chain()
                    .in_set(SceneSpawnerSystems::WorldInstanceSpawn),
            );

        // Register component hooks for DynamicWorldRoot
        app.world_mut()
            .register_component_hooks::<DynamicWorldRoot>()
            .on_remove(|mut world, context| {
                let Some(handle) = world.get::<DynamicWorldRoot>(context.entity) else {
                    return;
                };
                let id = handle.id();
                if let Some(&WorldInstance(instance_id)) =
                    world.get::<WorldInstance>(context.entity)
                {
                    let Some(mut world_instance_spawner) =
                        world.get_resource_mut::<WorldInstanceSpawner>()
                    else {
                        return;
                    };
                    if let Some(instance_ids) =
                        world_instance_spawner.spawned_dynamic_worlds.get_mut(&id)
                    {
                        instance_ids.remove(&instance_id);
                    }
                    world_instance_spawner.unregister_instance(instance_id);
                }
            });

        // Register component hooks for WorldAssetRoot
        app.world_mut()
            .register_component_hooks::<WorldAssetRoot>()
            .on_remove(|mut world, context| {
                let Some(handle) = world.get::<WorldAssetRoot>(context.entity) else {
                    return;
                };
                let id = handle.id();
                if let Some(&WorldInstance(instance_id)) =
                    world.get::<WorldInstance>(context.entity)
                {
                    let Some(mut world_instance_spawner) =
                        world.get_resource_mut::<WorldInstanceSpawner>()
                    else {
                        return;
                    };
                    if let Some(instance_ids) = world_instance_spawner.spawned_worlds.get_mut(&id) {
                        instance_ids.remove(&instance_id);
                    }
                    world_instance_spawner.unregister_instance(instance_id);
                }
            });
    }
}

#[cfg(not(feature = "serialize"))]
impl Plugin for WorldSerializationPlugin {
    fn build(&self, _: &mut App) {}
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_asset::{AssetPlugin, DirectAssetAccessExt};
    use bevy_ecs::{
        component::Component,
        entity::Entity,
        hierarchy::{ChildOf, Children},
        reflect::{AppTypeRegistry, ReflectComponent},
        world::World,
    };
    use bevy_reflect::Reflect;

    use crate::{
        DynamicWorldBuilder, DynamicWorldRoot, WorldAsset, WorldAssetRoot, WorldSerializationPlugin,
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
    fn world_instance_spawns_and_respawns_after_change() {
        let mut app = App::new();

        app.add_plugins((AssetPlugin::default(), WorldSerializationPlugin))
            .register_type::<ChildOf>()
            .register_type::<Children>()
            .register_type::<Circle>()
            .register_type::<Rectangle>()
            .register_type::<Triangle>()
            .register_type::<FinishLine>();

        let handle = app.world_mut().reserve_asset_handle();

        let instance_entity = app.world_mut().spawn(WorldAssetRoot(handle.clone())).id();
        app.update();

        assert!(app
            .world()
            .entity(instance_entity)
            .get::<Children>()
            .is_none());

        let mut world_1 = WorldAsset {
            world: World::new(),
        };
        let root = world_1.world.spawn_empty().id();
        world_1.world.spawn((
            Rectangle {
                width: 10.0,
                height: 5.0,
            },
            FinishLine,
            ChildOf(root),
        ));
        world_1.world.spawn((Circle { radius: 7.0 }, ChildOf(root)));

        app.world_mut().insert_asset(&handle, world_1).unwrap();

        app.update();
        // TODO: multiple updates to avoid debounced asset events. See comment on WorldInstanceSpawner::debounced_world_asset_events
        app.update();
        app.update();
        app.update();

        let child_root = app
            .world()
            .entity(instance_entity)
            .get::<Children>()
            .and_then(|children| children.first().cloned())
            .expect("There should be exactly one child on the world asset root");
        let children = app
            .world()
            .entity(child_root)
            .get::<Children>()
            .expect("The child of the world asset root should itself have 2 children");
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

        // Now that we know our world contains exactly what we expect, we will change the world
        // asset and ensure it contains the new results.

        let mut world_2 = WorldAsset {
            world: World::new(),
        };
        let root = world_2.world.spawn_empty().id();
        world_2.world.spawn((
            Triangle {
                base: 1.0,
                height: 2.0,
            },
            ChildOf(root),
        ));

        app.world_mut().insert_asset(&handle, world_2).unwrap();

        app.update();
        app.update();

        let child_root = app
            .world()
            .entity(instance_entity)
            .get::<Children>()
            .and_then(|children| children.first().cloned())
            .expect("There should be exactly one child on the world asset root");
        let children = app
            .world()
            .entity(child_root)
            .get::<Children>()
            .expect("The child of the world asset root should itself have 2 children");
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
    fn dynamic_world_spawns_and_respawns_after_change() {
        let mut app = App::new();

        app.add_plugins((AssetPlugin::default(), WorldSerializationPlugin))
            .register_type::<ChildOf>()
            .register_type::<Children>()
            .register_type::<Circle>()
            .register_type::<Rectangle>()
            .register_type::<Triangle>()
            .register_type::<FinishLine>();

        let handle = app.world_mut().reserve_asset_handle();

        let instance_entity = app.world_mut().spawn(DynamicWorldRoot(handle.clone())).id();
        app.update();

        assert!(app
            .world()
            .entity(instance_entity)
            .get::<Children>()
            .is_none());

        let create_dynamic_world = |mut world_asset: WorldAsset, world: &World| {
            let type_registry = world.resource::<AppTypeRegistry>().read();
            let entities: Vec<Entity> = world_asset
                .world
                .query::<Entity>()
                .iter(&world_asset.world)
                .collect();
            DynamicWorldBuilder::from_world(&world_asset.world, &type_registry)
                .extract_entities(entities.into_iter())
                .build()
        };

        let mut world_1 = WorldAsset {
            world: World::new(),
        };
        let root = world_1.world.spawn_empty().id();
        world_1.world.spawn((
            Rectangle {
                width: 10.0,
                height: 5.0,
            },
            FinishLine,
            ChildOf(root),
        ));
        world_1.world.spawn((Circle { radius: 7.0 }, ChildOf(root)));

        let dynamic_world_1 = create_dynamic_world(world_1, app.world());
        app.world_mut()
            .insert_asset(&handle, dynamic_world_1)
            .unwrap();

        app.update();
        // TODO: multiple updates to avoid debounced asset events. See comment on WorldInstanceSpawner::debounced_world_asset_events
        app.update();
        app.update();
        app.update();

        let child_root = app
            .world()
            .entity(instance_entity)
            .get::<Children>()
            .and_then(|children| children.first().cloned())
            .expect("There should be exactly one child on the world asset root");
        let children = app
            .world()
            .entity(child_root)
            .get::<Children>()
            .expect("The child of the world asset root should itself have 2 children");
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

        // Now that we know our world contains exactly what we expect, we will change the world
        // asset and ensure it contains the new results.

        let mut world_2 = WorldAsset {
            world: World::new(),
        };
        let root = world_2.world.spawn_empty().id();
        world_2.world.spawn((
            Triangle {
                base: 1.0,
                height: 2.0,
            },
            ChildOf(root),
        ));

        let dynamic_world_2 = create_dynamic_world(world_2, app.world());

        app.world_mut()
            .insert_asset(&handle, dynamic_world_2)
            .unwrap();

        app.update();
        app.update();

        let child_root = app
            .world()
            .entity(instance_entity)
            .get::<Children>()
            .and_then(|children| children.first().cloned())
            .expect("There should be exactly one child on the world asset root");
        let children = app
            .world()
            .entity(child_root)
            .get::<Children>()
            .expect("The child of the world asset root should itself have 2 children");
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
