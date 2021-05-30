use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Assets};
use bevy_ecs::{
    schedule::{ParallelSystemDescriptorCoercion, StageLabel, SystemLabel, SystemStage},
    system::IntoSystem,
};
//use bevy_reflect::TypeRegistration;

mod animator;
mod app;
mod help;
mod reflect;
//mod experimental;
mod bench;
mod skinned_mesh;

pub mod blending;

pub use crate::{
    animator::*, app::*, bench::*, blending::AnimatorBlending, reflect::AnimatorPropertyRegistry,
};

pub mod prelude {
    pub use crate::{
        animator::{Animator, Clip},
        app::AddAnimated,
        blending::AnimatorBlending,
        reflect::AnimatorPropertyRegistry,
        skinned_mesh::{SkinAsset, SkinComponent, SkinDebugger},
    };
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum AnimationStage {
    Animate,
    Skinning,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum AnimationSystem {
    Animate,
    ClipEvents,
    Skinning,
}

pub struct AnimationPlugin {
    /// Enables or disables the built in skinning
    pub skinning: bool,
}

impl Default for AnimationPlugin {
    fn default() -> Self {
        Self { skinning: true }
    }
}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_after(
            CoreStage::Update,
            AnimationStage::Animate,
            SystemStage::parallel(),
        );
        app.add_stage_after(
            CoreStage::PostUpdate,
            AnimationStage::Skinning,
            SystemStage::parallel(),
        );

        // Generic animation
        app.insert_resource(animator::AnimatorRegistry::default())
            .add_asset::<Clip>()
            //.add_asset_loader(ClipLoader)
            .register_type::<Animator>()
            .add_system_to_stage(
                AnimationStage::Animate,
                Assets::<Clip>::asset_event_system
                    .system()
                    .label(AnimationSystem::ClipEvents),
            ) // ? NOTE: Fix asset event handle
            .add_system_to_stage(
                AnimationStage::Animate,
                animator::animator_update_system
                    .system()
                    .label(AnimationSystem::Animate)
                    .after(AnimationSystem::ClipEvents),
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

        if self.skinning {
            app.add_startup_system(skinned_mesh::skinning_setup.system())
                .add_system_to_stage(
                    AnimationStage::Skinning,
                    skinned_mesh::skinning_update
                        .system()
                        .label(AnimationSystem::Skinning),
                )
                .add_system_to_stage(
                    AnimationStage::Skinning,
                    skinned_mesh::skinning_debugger_update
                        .system()
                        .after(AnimationSystem::Skinning),
                );
        }
    }
}

// TODO: AppBuilder trait to add animated components and assets
