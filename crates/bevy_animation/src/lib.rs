use bevy_app::prelude::*;
use bevy_asset::AddAsset;
//use bevy_ecs::prelude::*;
use bevy_type_registry::RegisterType;

pub mod generic;
pub mod lerping;
mod skined_mesh;

pub use crate::generic::*;
pub use crate::skined_mesh::*;
//mod util;

pub mod prelude {
    pub use crate::generic::{Animator, Clip, Curve, CurveUntyped};
    pub use crate::lerping::LerpValue;
    pub use crate::skined_mesh::{MeshSkin, MeshSkinBinder, MeshSkinnerDebuger};
}

pub mod stage {
    pub const ANIMATE: &'static str = "animate";
    // ? NOTE: At this stage the Children component should be in a valid state
    // sadly this will add a one frame startup delay for each animator
    pub(crate) const WARM_ANIMATE: &'static str = "warm_animate";
    pub use bevy_app::stage::POST_UPDATE;
    pub use bevy_app::stage::UPDATE;
}

#[derive(Default)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_after(stage::UPDATE, stage::ANIMATE);
        app.add_stage_after(stage::POST_UPDATE, stage::WARM_ANIMATE);

        // Generic animation
        app.add_asset::<Clip>()
            //.add_asset_loader(ClipLoader)
            .register_component::<Animator>()
            .add_system_to_stage(stage::ANIMATE, animator_update)
            .add_system_to_stage(stage::WARM_ANIMATE, animator_fetch);

        // Skinning
        app.add_asset::<MeshSkin>()
            .register_component_with::<MeshSkinBinder>(|reg| reg.map_entities())
            .register_component::<MeshSkinnerDebuger>()
            .add_system_to_stage(stage::ANIMATE, mesh_skinner_debugger_update)
            .add_system_to_stage(stage::WARM_ANIMATE, mesh_skinner_startup);
    }
}
