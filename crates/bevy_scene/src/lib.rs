mod loaded_scenes;
mod scene;
mod scene_spawner;
pub mod serde;

pub use loaded_scenes::*;
pub use scene::*;
pub use scene_spawner::*;

use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::IntoThreadLocalSystem;

#[derive(Default)]
pub struct ScenePlugin;

pub const SCENE_STAGE: &str = "scene";

impl AppPlugin for ScenePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<Scene>()
            .add_asset_loader::<Scene, SceneLoader>()
            .init_resource::<SceneSpawner>()
            .add_stage_after(stage::EVENT_UPDATE, SCENE_STAGE)
            .add_system_to_stage(SCENE_STAGE, scene_spawner_system.thread_local_system());
    }
}
