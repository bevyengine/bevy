mod command;
mod dynamic_scene;
mod scene;
mod scene_loader;
mod scene_spawner;
pub mod serde;

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
use bevy_ecs::{IntoExclusiveSystem, StageLabel, SystemStage};

#[derive(Default)]
pub struct ScenePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
enum SceneStage {
    SceneStage,
}

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<DynamicScene>()
            .add_asset::<Scene>()
            .init_asset_loader::<SceneLoader>()
            .init_resource::<SceneSpawner>()
            .add_stage_after(
                CoreStage::Event,
                SceneStage::SceneStage,
                SystemStage::parallel(),
            )
            .add_system_to_stage(
                SceneStage::SceneStage,
                scene_spawner_system.exclusive_system(),
            );
    }
}
