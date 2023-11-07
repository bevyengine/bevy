mod asset;
mod from_mesh;
mod gpu_scene;
mod material_draw_nodes;
mod persistent_buffer;
mod persistent_buffer_impls;
mod pipelines;
mod prepare_materials;
mod visibility_buffer_node;

pub(crate) use self::{
    gpu_scene::{queue_material_meshlet_meshes, MeshletGpuScene},
    prepare_materials::prepare_material_meshlet_meshes,
};

pub use self::{
    asset::{Meshlet, MeshletBoundingSphere, MeshletMesh},
    from_mesh::MeshToMeshletMeshConversionError,
};

use self::{
    gpu_scene::{
        extract_meshlet_meshes, perform_pending_meshlet_mesh_writes,
        prepare_meshlet_per_frame_resources, prepare_meshlet_view_bind_groups,
    },
    material_draw_nodes::{
        draw_3d_graph::node::MESHLET_MAIN_OPAQUE_PASS_3D, MeshletMainOpaquePass3dNode,
    },
    pipelines::{
        MeshletPipelines, MESHLET_COPY_MATERIAL_DEPTH_SHADER_HANDLE, MESHLET_CULLING_SHADER_HANDLE,
        MESHLET_VISIBILITY_BUFFER_SHADER_HANDLE,
    },
    visibility_buffer_node::{
        draw_3d_graph::node::MESHLET_VISIBILITY_BUFFER_PASS, MeshletVisibilityBufferPassNode,
    },
};
use crate::Material;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetApp, Handle};
use bevy_core_pipeline::core_3d::{graph::node::*, CORE_3D};
use bevy_ecs::{bundle::Bundle, schedule::IntoSystemConfigs};
use bevy_render::{
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::Shader,
    view::{InheritedVisibility, Msaa, ViewVisibility, Visibility},
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::components::{GlobalTransform, Transform};

const MESHLET_BINDINGS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1325134235233421);
pub(crate) const MESHLET_MESH_MATERIAL_SHADER_HANDLE: Handle<Shader> =
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
            MESHLET_MESH_MATERIAL_SHADER_HANDLE,
            "meshlet_mesh_material.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESHLET_COPY_MATERIAL_DEPTH_SHADER_HANDLE,
            "copy_material_depth.wgsl",
            Shader::from_wgsl
        );

        app.init_asset::<MeshletMesh>().insert_resource(Msaa::Off);
    }

    fn finish(&self, app: &mut App) {
        let Ok(app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        app.add_render_graph_node::<ViewNodeRunner<MeshletVisibilityBufferPassNode>>(
            CORE_3D,
            MESHLET_VISIBILITY_BUFFER_PASS,
        )
        .add_render_graph_node::<ViewNodeRunner<MeshletMainOpaquePass3dNode>>(
            CORE_3D,
            MESHLET_MAIN_OPAQUE_PASS_3D,
        )
        .add_render_graph_edges(
            CORE_3D,
            &[
                MESHLET_VISIBILITY_BUFFER_PASS,
                PREPASS,
                DEFERRED_PREPASS,
                COPY_DEFERRED_LIGHTING_ID,
                END_PREPASSES,
                START_MAIN_PASS,
                MESHLET_MAIN_OPAQUE_PASS_3D,
                MAIN_OPAQUE_PASS,
                END_MAIN_PASS,
            ],
        )
        .init_resource::<MeshletGpuScene>()
        .init_resource::<MeshletPipelines>()
        .add_systems(ExtractSchedule, extract_meshlet_meshes)
        .add_systems(
            Render,
            (
                perform_pending_meshlet_mesh_writes.in_set(RenderSet::PrepareAssets),
                prepare_meshlet_per_frame_resources.in_set(RenderSet::PrepareResources),
                prepare_meshlet_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
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
