mod binder;
mod blas;
mod extract;
mod types;

pub use binder::RaytracingSceneBindings;
pub use types::RaytracingMesh3d;

pub(crate) use binder::init_raytracing_scene_bindings;

use crate::SolariSystems;
use bevy_app::{App, Plugin};
use bevy_ecs::{schedule::IntoScheduleConfigs, system::ResMut};
use bevy_render::{
    extract_resource::ExtractResourcePlugin,
    load_shader_library,
    mesh::{
        allocator::{allocate_and_free_meshes, MeshAllocator},
        RenderMesh,
    },
    render_asset::prepare_assets,
    render_resource::BufferUsages,
    ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
};
use binder::prepare_raytracing_scene_bindings;
use blas::{prepare_raytracing_blas, BlasManager};
use extract::{extract_raytracing_scene, StandardMaterialAssets};

/// Creates acceleration structures and binding arrays of resources for raytracing.
pub struct RaytracingScenePlugin;

impl Plugin for RaytracingScenePlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "brdf.wgsl");
        load_shader_library!(app, "raytracing_scene_bindings.wgsl");
        load_shader_library!(app, "sampling.wgsl");

        app.register_type::<RaytracingMesh3d>()
            .add_plugins(ExtractResourcePlugin::<StandardMaterialAssets>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<BlasManager>()
            .init_resource::<StandardMaterialAssets>()
            .add_systems(
                RenderStartup,
                (
                    add_raytracing_extra_mesh_buffer_usages,
                    init_raytracing_scene_bindings,
                )
                    .in_set(SolariSystems),
            )
            .add_systems(
                ExtractSchedule,
                extract_raytracing_scene.in_set(SolariSystems),
            )
            .add_systems(
                Render,
                (
                    prepare_raytracing_blas
                        .in_set(RenderSystems::PrepareAssets)
                        .before(prepare_assets::<RenderMesh>)
                        .after(allocate_and_free_meshes),
                    prepare_raytracing_scene_bindings.in_set(RenderSystems::PrepareBindGroups),
                )
                    .in_set(SolariSystems),
            );
    }
}

fn add_raytracing_extra_mesh_buffer_usages(mut mesh_allocator: ResMut<MeshAllocator>) {
    mesh_allocator.extra_buffer_usages |= BufferUsages::BLAS_INPUT | BufferUsages::STORAGE;
}
