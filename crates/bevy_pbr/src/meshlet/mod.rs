mod asset;
mod culling_pipeline;
mod from_mesh;
mod gpu_scene;
mod node;
mod persistent_buffer;
mod psb_impls;

pub(crate) use self::gpu_scene::MeshletGpuScene;

pub use self::{
    asset::{Meshlet, MeshletBoundingSphere, MeshletMesh},
    from_mesh::MeshToMeshletMeshConversionError,
    node::{draw_3d_graph, MainMeshletOpaquePass3dNode},
};

use self::{
    culling_pipeline::{MeshletCullingPipeline, MESHLET_CULLING_SHADER_HANDLE},
    draw_3d_graph::node::MAIN_MESHLET_OPAQUE_PASS_3D,
    gpu_scene::{
        extract_meshlet_meshes, perform_pending_meshlet_mesh_writes,
        prepare_meshlet_per_frame_bind_groups, prepare_meshlet_per_frame_resources,
    },
};
use crate::Material;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetApp, Handle};
use bevy_core_pipeline::core_3d::{
    graph::node::{MAIN_OPAQUE_PASS, START_MAIN_PASS},
    CORE_3D,
};
use bevy_ecs::{bundle::Bundle, schedule::IntoSystemConfigs};
use bevy_render::{
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::Shader,
    view::{InheritedVisibility, ViewVisibility, Visibility},
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::components::{GlobalTransform, Transform};

const MESHLET_BINDINGS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1325134235233421);
pub(crate) const MESHLET_RASTER_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(2325134235233421);

pub struct MeshletPlugin;

impl Plugin for MeshletPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            MESHLET_BINDINGS_SHADER_HANDLE,
            "meshlet_bindings.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESHLET_CULLING_SHADER_HANDLE,
            "cull_meshlets.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESHLET_RASTER_SHADER_HANDLE,
            "meshlet_raster.wgsl",
            Shader::from_wgsl
        );
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
        .init_resource::<MeshletCullingPipeline>()
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

/// A component bundle for entities with a [`MeshletMesh`] and a [`Material`].
#[derive(Bundle, Clone)]
pub struct MaterialMeshletMeshBundle<M: Material> {
    pub meshlet_mesh: Handle<MeshletMesh>,
    pub material: Handle<M>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
}

impl<M: Material> Default for MaterialMeshletMeshBundle<M> {
    fn default() -> Self {
        Self {
            meshlet_mesh: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            inherited_visibility: Default::default(),
            view_visibility: Default::default(),
        }
    }
}
