use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Assets};
use bevy_reflect::RegisterTypeBuilder;

mod animator;
mod app;
mod help;
mod reflect;
//mod experimental;
mod bench;
mod hierarchy;
mod skinned_mesh;

pub mod blending;
pub mod curve;
pub mod lerping;

pub use crate::animator::*;
pub use crate::app::*;
pub use crate::bench::*;
pub use crate::blending::AnimatorBlending;
pub use crate::hierarchy::Hierarchy;
pub use crate::reflect::AnimatorPropertyRegistry;

pub mod prelude {
    pub use crate::animator::{Animator, Clip};
    pub use crate::app::AddAnimated;
    pub use crate::blending::AnimatorBlending;
    pub use crate::curve::Curve;
    pub use crate::hierarchy::Hierarchy;
    pub use crate::lerping::Lerp;
    pub use crate::reflect::AnimatorPropertyRegistry;
    pub use crate::skinned_mesh::{SkinAsset, SkinComponent, SkinDebugger};
}

pub mod stage {
    pub const ANIMATE: &'static str = "animate";
    pub use bevy_app::stage::POST_UPDATE;
    pub use bevy_app::stage::UPDATE;
}

#[derive(Default)]
pub struct AnimationPlugin {
    /// Headless mode (no skinning)
    pub headless: bool,
}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_after(stage::UPDATE, stage::ANIMATE);

        // Generic animation
        app.add_resource(animator::AnimatorRegistry::default())
            .add_asset::<Clip>()
            //.add_asset_loader(ClipLoader)
            .register_type::<Animator>()
            .add_system_to_stage(stage::ANIMATE, Assets::<Clip>::asset_event_system) // ? NOTE: Fix asset event handle
            .add_system_to_stage(stage::ANIMATE, animator::animator_update_system);

        // ! FIXME: Each added animated component or asset will add a bit of overhead in the animation
        // ! system, I have no idea how big this is but I would like to make it pay only for what you use
        app.add_resource(reflect::AnimatorPropertyRegistry::default());
        app.register_animated_component::<bevy_transform::prelude::Transform>();

        // Skinning
        app.add_asset::<skinned_mesh::SkinAsset>()
            .add_asset::<skinned_mesh::SkinInstance>()
            .register_type::<skinned_mesh::SkinComponent>()
            .register_type::<skinned_mesh::SkinDebugger>();

        if !self.headless {
            app.add_startup_system(skinned_mesh::skinning_setup)
                .add_system_to_stage(stage::POST_UPDATE, skinned_mesh::skinning_update)
                .add_system_to_stage(stage::POST_UPDATE, skinned_mesh::skinning_debugger_update);
        }
    }
}

// TODO: AppBuilder trait to add animated components and assets
