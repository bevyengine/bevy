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

use bevy_ecs::schedule::IntoSystemConfigs;
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
use bevy_asset::AssetApp;

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
    }
}

#[cfg(not(feature = "serialize"))]
impl Plugin for ScenePlugin {
    fn build(&self, _: &mut App) {}
}
