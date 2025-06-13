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
use bevy_render::settings::WgpuFeatures;

/// An experimental plugin for raytraced lighting.
///
/// This plugin provides:
/// * (Coming soon) - Raytraced direct and indirect lighting.
/// * [`RaytracingScenePlugin`] - BLAS building, resource and lighting binding.
/// * [`PathtracingPlugin`] - A non-realtime pathtracer for validation purposes.
///
/// To get started, add `RaytracingMesh3d` and `MeshMaterial3d::<StandardMaterial>` to your entities.
pub struct SolariPlugin;

impl Plugin for SolariPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RaytracingScenePlugin, SolariLightingPlugin));
    }
}

impl SolariPlugin {
    /// [`WgpuFeatures`] required for this plugin to function.
    pub fn required_wgpu_features() -> WgpuFeatures {
        WgpuFeatures::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE
            | WgpuFeatures::EXPERIMENTAL_RAY_QUERY
            | WgpuFeatures::BUFFER_BINDING_ARRAY
            | WgpuFeatures::TEXTURE_BINDING_ARRAY
            | WgpuFeatures::UNIFORM_BUFFER_AND_STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING
            | WgpuFeatures::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
            | WgpuFeatures::PARTIALLY_BOUND_BINDING_ARRAY
    }
}
