use bevy_app::prelude::*;
use bevy_asset::AddAsset;
//use bevy_ecs::prelude::*;
use bevy_type_registry::RegisterType;

mod custom;
mod hierarchy;
mod skinned_mesh;

pub mod blending;
pub mod curve;
pub mod lerping;

pub use crate::custom::*;
pub use crate::hierarchy::Hierarchy;
pub use crate::skinned_mesh::*;

pub mod prelude {
    pub use crate::blending::AnimatorBlending;
    pub use crate::curve::{Curve, CurveUntyped};
    pub use crate::custom::Animator;
    pub use crate::custom::Clip;
    pub use crate::hierarchy::Hierarchy;
    pub use crate::lerping::Lerp;
    pub use crate::skinned_mesh::{SkinAsset, SkinComponent, SkinDebugger};
}

pub mod stage {
    pub const ANIMATE: &'static str = "animate";
    pub use bevy_app::stage::POST_UPDATE;
    pub use bevy_app::stage::UPDATE;
}

#[derive(Default)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_after(stage::UPDATE, stage::ANIMATE);

        // Generic animation
        app.add_asset::<prelude::Clip>()
            //.add_asset_loader(ClipLoader)
            .register_component::<prelude::Animator>()
            // .add_system_to_stage(stage::ANIMATE, animator_binding_system)
            // .add_system_to_stage(stage::ANIMATE, animator_update_system);
            .add_system_to_stage(stage::ANIMATE, animator_update_system)
            .add_system_to_stage(stage::ANIMATE, animator_transform_update_system);

        // Skinning
        app.add_asset::<SkinAsset>()
            .add_asset::<SkinInstance>()
            .register_component_with::<SkinComponent>(|reg| reg.map_entities())
            .register_component::<SkinDebugger>()
            .add_startup_system(skinning_setup)
            .add_system_to_stage(stage::POST_UPDATE, skinning_update)
            .add_system_to_stage(stage::POST_UPDATE, skinning_debugger_update);
    }
}

// TODO: AppBuilder trait to add animated components and assets
