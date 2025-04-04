mod binder;
mod blas;
mod extract;
mod types;

pub use types::RaytracingMesh3d;

use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_render::{
    mesh::{
        allocator::{allocate_and_free_meshes, MeshAllocator},
        RenderMesh,
    },
    render_asset::prepare_assets,
    render_resource::BufferUsages,
    renderer::RenderDevice,
    settings::WgpuFeatures,
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use binder::{prepare_raytracing_scene_bindings, RaytracingSceneBindings};
use blas::{manage_blas, BlasManager};
use extract::extract_raytracing_scene;
use tracing::warn;

pub struct RaytracingScenePlugin;

impl Plugin for RaytracingScenePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RaytracingMesh3d>();
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        let render_device = render_app.world().resource::<RenderDevice>();
        let features = render_device.features();
        if !features.contains(Self::required_wgpu_features()) {
            warn!(
                "SolariScenePlugin not loaded. GPU lacks support for required feature: {:?}.",
                Self::required_wgpu_features().difference(features)
            );
            return;
        }

        render_app
            .world_mut()
            .resource_mut::<MeshAllocator>()
            .extra_buffer_usages |= BufferUsages::BLAS_INPUT | BufferUsages::STORAGE;

        render_app
            .init_resource::<BlasManager>()
            .init_resource::<RaytracingSceneBindings>()
            .add_systems(ExtractSchedule, extract_raytracing_scene)
            .add_systems(
                Render,
                (
                    manage_blas
                        .in_set(RenderSet::PrepareAssets)
                        .before(prepare_assets::<RenderMesh>)
                        .after(allocate_and_free_meshes),
                    prepare_raytracing_scene_bindings.in_set(RenderSet::PrepareBindGroups),
                ),
            );
    }
}

impl RaytracingScenePlugin {
    /// [`WgpuFeatures`] required for this plugin to function.
    pub fn required_wgpu_features() -> WgpuFeatures {
        WgpuFeatures::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE
            | WgpuFeatures::BUFFER_BINDING_ARRAY
            | WgpuFeatures::TEXTURE_BINDING_ARRAY
            | WgpuFeatures::UNIFORM_BUFFER_AND_STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING
            | WgpuFeatures::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
            | WgpuFeatures::PARTIALLY_BOUND_BINDING_ARRAY
    }
}
