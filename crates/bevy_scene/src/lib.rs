#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Provides scene definition, instantiation and serialization/deserialization.
//!
//! Scenes are collections of entities and their associated components that can be
//! instantiated or removed from a world to allow composition. Scenes can be serialized/deserialized,
//! for example to save part of the world state to a file.

mod bundle;
mod dynamic_scene;
mod dynamic_scene_builder;
mod scene;
mod scene_filter;
mod scene_loader;
mod scene_spawner;

#[cfg(feature = "serialize")]
pub mod serde;

/// Rusty Object Notation, a crate used to serialize and deserialize bevy scenes.
pub use bevy_asset::ron;

use bevy_ecs::schedule::IntoSystemConfigs;
pub use bundle::*;
pub use dynamic_scene::*;
pub use dynamic_scene_builder::*;
pub use scene::*;
pub use scene_filter::*;
pub use scene_loader::*;
pub use scene_spawner::*;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        DynamicScene, DynamicSceneBuilder, DynamicSceneBundle, Scene, SceneBundle, SceneFilter,
        SceneSpawner,
    };
}

use bevy_app::prelude::*;
use bevy_asset::{AssetApp, Handle};

/// Plugin that provides scene functionality to an [`App`].
#[derive(Default)]
pub struct ScenePlugin;

#[cfg(feature = "serialize")]
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<DynamicScene>()
            .init_asset::<Scene>()
            .init_asset_loader::<SceneLoader>()
            .add_event::<SceneInstanceReady>()
            .init_resource::<SceneSpawner>()
            .add_systems(SpawnScene, (scene_spawner, scene_spawner_system).chain());

        // Register component hooks for DynamicScene
        app.world_mut()
            .register_component_hooks::<Handle<DynamicScene>>()
            .on_remove(|mut world, entity, _| {
                let Some(handle) = world.get::<Handle<DynamicScene>>(entity) else {
                    return;
                };
                let id = handle.id();
                if let Some(&SceneInstance(scene_instance)) = world.get::<SceneInstance>(entity) {
                    let Some(mut scene_spawner) = world.get_resource_mut::<SceneSpawner>() else {
                        return;
                    };
                    if let Some(instance_ids) = scene_spawner.spawned_dynamic_scenes.get_mut(&id) {
                        instance_ids.remove(&scene_instance);
                    }
                    scene_spawner.despawn_instance(scene_instance);
                }
            });

        // Register component hooks for Scene
        app.world_mut()
            .register_component_hooks::<Handle<Scene>>()
            .on_remove(|mut world, entity, _| {
                if let Some(&SceneInstance(scene_instance)) = world.get::<SceneInstance>(entity) {
                    let Some(mut scene_spawner) = world.get_resource_mut::<SceneSpawner>() else {
                        return;
                    };
                    scene_spawner.despawn_instance(scene_instance);
                }
            });
    }
}

#[cfg(not(feature = "serialize"))]
impl Plugin for ScenePlugin {
    fn build(&self, _: &mut App) {}
}
