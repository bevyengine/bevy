mod asset;
mod from_mesh;
mod gpu_scene;
mod node;
mod persistent_buffer;
mod psb_impls;

pub use self::{
    asset::{Meshlet, MeshletBoundingSphere, MeshletMesh},
    node::{draw_3d_graph::node::MAIN_MESHLET_OPAQUE_PASS_3D, MainMeshletOpaquePass3dNode},
};

use self::gpu_scene::{
    extract_meshlet_meshes, perform_pending_meshlet_mesh_writes,
    prepare_meshlet_per_frame_bind_groups, prepare_meshlet_per_frame_resources, MeshletGpuScene,
};
use bevy_app::{App, Plugin};
use bevy_asset::AssetApp;
use bevy_core_pipeline::core_3d::{
    graph::node::{MAIN_OPAQUE_PASS, START_MAIN_PASS},
    CORE_3D,
};
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_render::{
    render_graph::{RenderGraphApp, ViewNodeRunner},
    ExtractSchedule, Render, RenderApp, RenderSet,
};

// TODO: Gate plugin (and meshopt dependency) behind a cargo feature
pub struct MeshletPlugin;

impl Plugin for MeshletPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<MeshletMesh>();
    }

    fn finish(&self, app: &mut App) {
        let Ok(app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        app.add_render_graph_node::<ViewNodeRunner<MainMeshletOpaquePass3dNode>>(
            CORE_3D,
            MAIN_MESHLET_OPAQUE_PASS_3D,
        )
        .add_render_graph_edges(
            CORE_3D,
            &[
                START_MAIN_PASS,
                MAIN_MESHLET_OPAQUE_PASS_3D,
                MAIN_OPAQUE_PASS,
            ],
        )
        .init_resource::<MeshletGpuScene>()
        .add_systems(ExtractSchedule, extract_meshlet_meshes)
        .add_systems(
            Render,
            (
                perform_pending_meshlet_mesh_writes.in_set(RenderSet::PrepareAssets),
                prepare_meshlet_per_frame_resources.in_set(RenderSet::PrepareResources),
                prepare_meshlet_per_frame_bind_groups.in_set(RenderSet::PrepareBindGroups),
            ),
        );
    }
}
