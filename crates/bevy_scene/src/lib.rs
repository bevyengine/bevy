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
    #[doc(hidden)]
    pub use crate::{
        DynamicScene, Scene, SceneSpawner, SpawnSceneAsChildCommands, SpawnSceneCommands,
    };
}

use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::{
    prelude::{ExclusiveConfig, StageConfig},
    system::IntoExclusiveSystem,
};

#[derive(Default)]
pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<DynamicScene>()
            .add_asset::<Scene>()
            .init_asset_loader::<SceneLoader>()
            .init_resource::<SceneSpawner>()
            .add_exclusive(
                scene_spawner_system
                    .exclusive_system()
                    .stage(CoreStage::PreUpdate)
                    .at_end(),
            );
    }
}
