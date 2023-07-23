#![allow(clippy::type_complexity)]

mod bundle;
mod dynamic_scene;
mod dynamic_scene_builder;
mod scene;
mod scene_filter;
mod scene_loader;
mod scene_spawner;

#[cfg(feature = "serialize")]
pub mod serde;

pub use bundle::*;
pub use dynamic_scene::*;
pub use dynamic_scene_builder::*;
pub use scene::*;
pub use scene_filter::*;
pub use scene_loader::*;
pub use scene_spawner::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        DynamicScene, DynamicSceneBuilder, DynamicSceneBundle, Scene, SceneBundle, SceneFilter,
        SceneSpawner,
    };
}

use bevy_app::prelude::*;
use bevy_asset::AddAsset;

#[derive(Default)]
pub struct ScenePlugin;

#[cfg(feature = "serialize")]
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<DynamicScene>()
            .add_asset::<Scene>()
            .init_asset_loader::<SceneLoader>()
            .init_resource::<SceneSpawner>()
            .add_systems(Update, scene_spawner_system)
            // Systems `*_bundle_spawner` must run before `scene_spawner_system`
            .add_systems(PreUpdate, scene_spawner);
    }
}

#[cfg(not(feature = "serialize"))]
impl Plugin for ScenePlugin {
    fn build(&self, _: &mut App) {}
}
