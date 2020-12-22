use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Assets};
use bevy_reflect::RegisterTypeBuilder;

mod app;
mod custom;
mod experimental;
mod hierarchy;
mod impls;
mod skinned_mesh;

pub mod blending;
pub mod curve;
pub mod lerping;

pub use crate::app::*;
pub use crate::custom::*;
pub use crate::hierarchy::Hierarchy;
pub use crate::skinned_mesh::*;

pub mod prelude {
    pub use crate::app::AddAnimated;
    pub use crate::blending::AnimatorBlending;
    pub use crate::curve::{Curve, CurveUntyped};
    pub use crate::custom::Clip;
    pub use crate::custom::{AnimatedAsset, AnimatedComponent, Animator};
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
        app.add_asset::<Clip>()
            //.add_asset_loader(ClipLoader)
            .register_type::<Animator>()
            .add_system_to_stage(stage::ANIMATE, Assets::<Clip>::asset_event_system) // ? NOTE: Fix asset event handle
            .add_system_to_stage(stage::ANIMATE, animator_update_system);

        app.register_animated_component::<bevy_transform::prelude::Transform>();

        // Skinning
        app.add_asset::<SkinAsset>()
            .add_asset::<SkinInstance>()
            .register_type::<SkinComponent>()
            .register_type::<SkinDebugger>()
            .add_startup_system(skinning_setup)
            .add_system_to_stage(stage::POST_UPDATE, skinning_update)
            .add_system_to_stage(stage::POST_UPDATE, skinning_debugger_update);
    }
}

// TODO: AppBuilder trait to add animated components and assets
