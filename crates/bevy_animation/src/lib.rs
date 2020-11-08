use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::prelude::*;
use bevy_type_registry::RegisterType;

use crate::generic::*;
use crate::skined_mesh::*;

pub mod generic;
mod lerping;
pub mod skined_mesh;

pub mod prelude {
    pub use crate::generic::*;
    pub use crate::lerping::LerpValue;
    pub use crate::skined_mesh::{MeshSkin, MeshSkinner};
}

pub mod stage {
    pub const ANIMATE: &'static str = "animate";
    pub use bevy_app::stage::UPDATE;
}

#[derive(Default)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<Clip>()
            .add_stage_after(stage::UPDATE, stage::ANIMATE)
            //.add_asset_loader(ClipLoader)
            .add_system_to_stage(stage::ANIMATE, animator_fetch.thread_local_system())
            //.add_system_to_stage(stage::ANIMATE, animator_update.system())
            .add_asset::<MeshSkin>()
            .register_component_with::<MeshSkinner>(|reg| reg.map_entities())
            .add_system(mesh_skinner_startup.system());
    }
}
