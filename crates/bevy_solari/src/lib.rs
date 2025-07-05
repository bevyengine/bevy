#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]

//! Provides raytraced lighting.
//!
//! See [`SolariPlugin`] for more info.
//!
//! ![`bevy_solari` logo](https://raw.githubusercontent.com/bevyengine/bevy/assets/branding/bevy_solari.svg)
pub mod pathtracer;
pub mod realtime;
pub mod scene;

/// The solari prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    pub use super::SolariPlugin;
    pub use crate::realtime::SolariLighting;
    pub use crate::scene::RaytracingMesh3d;
}

use crate::realtime::SolariLightingPlugin;
use crate::scene::RaytracingScenePlugin;
use bevy_app::{App, Plugin};
use bevy_ecs::{
    resource::Resource,
    schedule::{common_conditions::resource_exists, IntoScheduleConfigs, SystemSet},
    system::{Commands, Res},
};
use bevy_render::{
    renderer::RenderDevice, settings::WgpuFeatures, ExtractSchedule, Render, RenderStartup,
};
use tracing::warn;

/// An experimental plugin for raytraced lighting.
///
/// This plugin provides:
/// * [`SolariLightingPlugin`] - Raytraced direct and indirect lighting (indirect lighting not yet implemented).
/// * [`RaytracingScenePlugin`] - BLAS building, resource and lighting binding.
/// * [`pathtracer::PathtracingPlugin`] - A non-realtime pathtracer for validation purposes.
///
/// To get started, add `RaytracingMesh3d` and `MeshMaterial3d::<StandardMaterial>` to your entities.
pub struct SolariPlugin;

impl Plugin for SolariPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RaytracingScenePlugin, SolariLightingPlugin))
            // Note: conditions only run once per schedule run. So even though these conditions
            // could apply to many systems, they will only be checked once for the entire group.
            .configure_sets(
                RenderStartup,
                SolariSystems
                    .after(check_solari_has_required_features)
                    .run_if(resource_exists::<HasSolariRequiredFeatures>),
            )
            .configure_sets(
                ExtractSchedule,
                SolariSystems.run_if(resource_exists::<HasSolariRequiredFeatures>),
            )
            .configure_sets(
                Render,
                SolariSystems.run_if(resource_exists::<HasSolariRequiredFeatures>),
            );
    }
}

impl SolariPlugin {
    /// [`WgpuFeatures`] required for this plugin to function.
    pub fn required_wgpu_features() -> WgpuFeatures {
        WgpuFeatures::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE
            | WgpuFeatures::EXPERIMENTAL_RAY_QUERY
            | WgpuFeatures::BUFFER_BINDING_ARRAY
            | WgpuFeatures::TEXTURE_BINDING_ARRAY
            | WgpuFeatures::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
            | WgpuFeatures::PARTIALLY_BOUND_BINDING_ARRAY
    }
}

#[derive(SystemSet, PartialEq, Eq, Debug, Clone, Hash)]
pub struct SolariSystems;

/// A resource to track whether the renderer has the required features for Solari systems.
#[derive(Resource)]
struct HasSolariRequiredFeatures;

/// Check for the Solari required features once in startup, and insert a resource if the features
/// are enabled.
///
/// Now systems can do a cheap check for if the resource exists.
fn check_solari_has_required_features(mut commands: Commands, render_device: Res<RenderDevice>) {
    let features = render_device.features();
    if !features.contains(SolariPlugin::required_wgpu_features()) {
        warn!(
            "SolariSystems disabled. GPU lacks support for required features: {:?}.",
            SolariPlugin::required_wgpu_features().difference(features)
        );
        return;
    }
    commands.insert_resource(HasSolariRequiredFeatures);
}
