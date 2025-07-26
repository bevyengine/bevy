mod binder;
mod blas;
mod extract;
mod types;

pub use binder::RaytracingSceneBindings;
pub use types::RaytracingMesh3d;

use crate::SolariPlugins;
use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_render::{
    extract_resource::ExtractResourcePlugin,
    load_shader_library,
    mesh::{
        allocator::{allocate_and_free_meshes, MeshAllocator},
        RenderMesh,
    },
    render_asset::prepare_assets,
    render_resource::BufferUsages,
    renderer::RenderDevice,
    ExtractSchedule, Render, RenderApp, RenderSystems,
};
use binder::prepare_raytracing_scene_bindings;
use blas::{prepare_raytracing_blas, BlasManager};
use extract::{extract_raytracing_scene, StandardMaterialAssets};
use tracing::warn;

/// Creates acceleration structures and binding arrays of resources for raytracing.
pub struct RaytracingScenePlugin;

impl Plugin for RaytracingScenePlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "raytracing_scene_bindings.wgsl");
        load_shader_library!(app, "sampling.wgsl");

        app.register_type::<RaytracingMesh3d>();
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        let render_device = render_app.world().resource::<RenderDevice>();
        let features = render_device.features();
        if !features.contains(SolariPlugins::required_wgpu_features()) {
            warn!(
                "RaytracingScenePlugin not loaded. GPU lacks support for required features: {:?}.",
                SolariPlugins::required_wgpu_features().difference(features)
            );
            return;
        }

        app.add_plugins(ExtractResourcePlugin::<StandardMaterialAssets>::default());

        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .world_mut()
            .resource_mut::<MeshAllocator>()
            .extra_buffer_usages |= BufferUsages::BLAS_INPUT | BufferUsages::STORAGE;

        render_app
            .init_resource::<BlasManager>()
            .init_resource::<StandardMaterialAssets>()
            .init_resource::<RaytracingSceneBindings>()
            .add_systems(ExtractSchedule, extract_raytracing_scene)
            .add_systems(
                Render,
                (
                    prepare_raytracing_blas
                        .in_set(RenderSystems::PrepareAssets)
                        .before(prepare_assets::<RenderMesh>)
                        .after(allocate_and_free_meshes),
                    prepare_raytracing_scene_bindings.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}
