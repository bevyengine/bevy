mod command;
mod dynamic_scene;
mod scene;
mod scene_loader;
mod scene_spawner;
pub mod serde;

use bevy_ecs::{IntoSystem, SystemStage};
pub use command::*;
pub use dynamic_scene::*;
pub use scene::*;
pub use scene_loader::*;
pub use scene_spawner::*;

pub mod prelude {
    pub use crate::{
        DynamicScene, Scene, SceneSpawner, SpawnSceneAsChildCommands, SpawnSceneCommands,
    };
}

use bevy_app::prelude::*;
use bevy_asset::AddAsset;

#[derive(Default)]
pub struct ScenePlugin;

pub const SCENE_STAGE: &str = "scene";

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<DynamicScene>()
            .add_asset::<Scene>()
            .init_asset_loader::<SceneLoader>()
            .init_resource::<SceneSpawner>()
            .add_stage_after(stage::EVENT, SCENE_STAGE, SystemStage::parallel())
            .add_system_to_stage(SCENE_STAGE, scene_spawner_system.system());
    }
}
