mod asset;
mod from_mesh;
mod gpu_scene;
mod persistent_buffer;
mod pipelines;
mod psb_impls;
mod visibility_buffer_node;

pub(crate) use self::gpu_scene::{
    prepare_material_for_meshlet_meshes, queue_material_meshlet_meshes, MeshletGpuScene,
};

pub use self::{
    asset::{Meshlet, MeshletBoundingSphere, MeshletMesh},
    from_mesh::MeshToMeshletMeshConversionError,
    visibility_buffer_node::{draw_3d_graph, MeshletVisibilityBufferPassNode},
};

use self::{
    draw_3d_graph::node::MESHLET_VISIBILITY_BUFFER_PASS,
    gpu_scene::{
        extract_meshlet_meshes, perform_pending_meshlet_mesh_writes,
        prepare_meshlet_per_frame_bind_groups, prepare_meshlet_per_frame_resources,
    },
    pipelines::{
        MeshletPipelines, MESHLET_CULLING_SHADER_HANDLE, MESHLET_VISIBILITY_BUFFER_SHADER_HANDLE,
    },
};
use crate::Material;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetApp, Handle};
use bevy_core_pipeline::core_3d::{graph::node::PREPASS, CORE_3D};
use bevy_ecs::{bundle::Bundle, schedule::IntoSystemConfigs};
use bevy_render::{
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::Shader,
    view::{InheritedVisibility, ViewVisibility, Visibility},
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::components::{GlobalTransform, Transform};

const MESHLET_BINDINGS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1325134235233421);
pub(crate) const MESHLET_MATERIAL_SHADER_HANDLE: Handle<Shader> =
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
            MESHLET_VISIBILITY_BUFFER_SHADER_HANDLE,
            "visibility_buffer.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESHLET_MATERIAL_SHADER_HANDLE,
            "meshlet_material.wgsl",
            Shader::from_wgsl
        );
        app.init_asset::<MeshletMesh>();
    }

    fn finish(&self, app: &mut App) {
        let Ok(app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        app.add_render_graph_node::<ViewNodeRunner<MeshletVisibilityBufferPassNode>>(
            CORE_3D,
            MESHLET_VISIBILITY_BUFFER_PASS,
        )
        .add_render_graph_edges(CORE_3D, &[MESHLET_VISIBILITY_BUFFER_PASS, PREPASS])
        .init_resource::<MeshletGpuScene>()
        .init_resource::<MeshletPipelines>()
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
