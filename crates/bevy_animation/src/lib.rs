use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Assets};
use bevy_ecs::{IntoSystem, SystemStage};
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
pub mod interpolate;
pub mod tracks;

pub use crate::{
    animator::*, app::*, bench::*, blending::AnimatorBlending, hierarchy::Hierarchy,
    reflect::AnimatorPropertyRegistry,
};

pub mod prelude {
    pub use crate::{
        animator::{Animator, Clip},
        app::AddAnimated,
        blending::AnimatorBlending,
        hierarchy::Hierarchy,
        interpolate::Lerp,
        reflect::AnimatorPropertyRegistry,
        skinned_mesh::{SkinAsset, SkinComponent, SkinDebugger},
    };
}

/// Exports wide types
pub mod wide {
    pub use crate::interpolate::utils::{Quatx4, Quatx8};
    pub use ultraviolet::vec::{Vec2x4, Vec2x8, Vec3x4, Vec3x8, Vec4x4, Vec4x8};
    pub use wide::*;
}

pub mod stage {
    pub const ANIMATE: &'static str = "animate";
    pub use bevy_app::stage::{POST_UPDATE, UPDATE};
}

use bevy_ecs::ParallelSystemDescriptorCoercion;

#[derive(Default)]
pub struct AnimationPlugin {
    /// Headless mode (no skinning)
    pub headless: bool,
}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_after(stage::UPDATE, stage::ANIMATE, SystemStage::parallel());

        // Generic animation
        app.insert_resource(animator::AnimatorRegistry::default())
            .add_asset::<Clip>()
            //.add_asset_loader(ClipLoader)
            .register_type::<Animator>()
            .add_system_to_stage(
                stage::ANIMATE,
                Assets::<Clip>::asset_event_system
                    .system()
                    .label("clip_event_system"),
            ) // ? NOTE: Fix asset event handle
            .add_system_to_stage(
                stage::ANIMATE,
                animator::animator_update_system
                    .system()
                    .label("animator_update")
                    .after("clip_event_system"),
            );

        // ! FIXME: Each added animated component or asset will add a bit of overhead in the animation
        // ! system, I have no idea how big this is but I would like to make it pay only for what you use
        app.insert_resource(reflect::AnimatorPropertyRegistry::default());
        app.register_animated_component::<bevy_transform::prelude::Transform>();

        // Skinning
        app.add_asset::<skinned_mesh::SkinAsset>()
            .add_asset::<skinned_mesh::SkinInstance>()
            .register_type::<skinned_mesh::SkinComponent>()
            .register_type::<skinned_mesh::SkinDebugger>();

        if !self.headless {
            app.add_startup_system(skinned_mesh::skinning_setup.system())
                .add_system_to_stage(stage::POST_UPDATE, skinned_mesh::skinning_update.system())
                .add_system_to_stage(
                    stage::POST_UPDATE,
                    skinned_mesh::skinning_debugger_update.system(),
                );
        }
    }
}

// TODO: AppBuilder trait to add animated components and assets
