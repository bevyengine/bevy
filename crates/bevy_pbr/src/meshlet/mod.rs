//! Render high-poly 3d meshes using an efficient GPU-driven method. See [`MeshletPlugin`] and [`MeshletMesh`] for details.

mod asset;
#[cfg(feature = "meshlet_processor")]
mod from_mesh;
mod gpu_scene;
mod material_draw_nodes;
mod material_draw_prepare;
mod persistent_buffer;
mod persistent_buffer_impls;
mod pipelines;
mod visibility_buffer_raster_node;

pub mod graph {
    use bevy_render::render_graph::RenderLabel;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum NodeMeshlet {
        VisibilityBufferRasterPass,
        Prepass,
        DeferredPrepass,
        MainOpaquePass,
    }
}

pub(crate) use self::{
    gpu_scene::{queue_material_meshlet_meshes, MeshletGpuScene},
    material_draw_prepare::{
        prepare_material_meshlet_meshes_main_opaque_pass, prepare_material_meshlet_meshes_prepass,
    },
};

pub use self::asset::{Meshlet, MeshletBoundingSphere, MeshletMesh};
#[cfg(feature = "meshlet_processor")]
pub use self::from_mesh::MeshToMeshletMeshConversionError;

use self::{
    asset::MeshletMeshSaverLoad,
    gpu_scene::{
        extract_meshlet_meshes, perform_pending_meshlet_mesh_writes,
        prepare_meshlet_per_frame_resources, prepare_meshlet_view_bind_groups,
    },
    graph::NodeMeshlet,
    material_draw_nodes::{
        MeshletDeferredGBufferPrepassNode, MeshletMainOpaquePass3dNode, MeshletPrepassNode,
    },
    material_draw_prepare::{
        MeshletViewMaterialsDeferredGBufferPrepass, MeshletViewMaterialsMainOpaquePass,
        MeshletViewMaterialsPrepass,
    },
    pipelines::{
        MeshletPipelines, MESHLET_COPY_MATERIAL_DEPTH_SHADER_HANDLE, MESHLET_CULLING_SHADER_HANDLE,
        MESHLET_DOWNSAMPLE_DEPTH_SHADER_HANDLE, MESHLET_VISIBILITY_BUFFER_RASTER_SHADER_HANDLE,
        MESHLET_WRITE_INDEX_BUFFER_SHADER_HANDLE,
    },
    visibility_buffer_raster_node::MeshletVisibilityBufferRasterPassNode,
};
use crate::{graph::NodePbr, Material};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{load_internal_asset, AssetApp, Handle};
use bevy_core_pipeline::{
    core_3d::{
        graph::{Core3d, Node3d},
        Camera3d,
    },
    prepass::{DeferredPrepass, MotionVectorPrepass, NormalPrepass},
};
use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    prelude::With,
    query::Has,
    schedule::IntoSystemConfigs,
    system::{Commands, Query},
};
use bevy_render::{
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{Shader, TextureUsages},
    view::{
        check_visibility, prepare_view_targets, InheritedVisibility, Msaa, ViewVisibility,
        Visibility,
    },
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_transform::TransformSystem;

const MESHLET_BINDINGS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1325134235233421);
const MESHLET_MESH_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(3325134235233421);

/// Provides a plugin for rendering large amounts of high-poly 3d meshes using an efficient GPU-driven method. See also [`MeshletMesh`].
///
/// Rendering dense scenes made of high-poly meshes with thousands or millions of triangles is extremely expensive in Bevy's standard renderer.
/// Once meshes are pre-processed into a [`MeshletMesh`], this plugin can render these kinds of scenes very efficiently.
///
/// In comparison to Bevy's standard renderer:
/// * Minimal rendering work is done on the CPU. All rendering is GPU-driven.
/// * Much more efficient culling. Meshlets can be culled individually, instead of all or nothing culling for entire meshes at a time.
/// Additionally, occlusion culling can eliminate meshlets that would cause overdraw.
/// * Much more efficient batching. All geometry can be rasterized in a single indirect draw.
/// * Scales better with large amounts of dense geometry and overdraw. Bevy's standard renderer will bottleneck sooner.
/// * Much greater base overhead. Rendering will be slower than Bevy's standard renderer with small amounts of geometry and overdraw.
/// * Much greater memory usage.
/// * Requires preprocessing meshes. See [`MeshletMesh`] for details.
/// * More limitations on the kinds of materials you can use. See [`MeshletMesh`] for details.
///
/// This plugin is not compatible with [`Msaa`], and adding this plugin will disable it.
///
/// This plugin does not work on the WebGL2 backend.
///
/// ![A render of the Stanford dragon as a `MeshletMesh`](https://raw.githubusercontent.com/bevyengine/bevy/meshlet/crates/bevy_pbr/src/meshlet/meshlet_preview.png)
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
            super::MESHLET_VISIBILITY_BUFFER_RESOLVE_SHADER_HANDLE,
            "visibility_buffer_resolve.wgsl",
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
            MESHLET_WRITE_INDEX_BUFFER_SHADER_HANDLE,
            "write_index_buffer.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESHLET_DOWNSAMPLE_DEPTH_SHADER_HANDLE,
            "downsample_depth.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESHLET_VISIBILITY_BUFFER_RASTER_SHADER_HANDLE,
            "visibility_buffer_raster.wgsl",
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

        app.init_asset::<MeshletMesh>()
            .register_asset_loader(MeshletMeshSaverLoad)
            .insert_resource(Msaa::Off)
            .add_systems(
                PostUpdate,
                check_visibility::<WithMeshletMesh>.after(TransformSystem::TransformPropagate),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<MeshletVisibilityBufferRasterPassNode>(
                Core3d,
                NodeMeshlet::VisibilityBufferRasterPass,
            )
            .add_render_graph_node::<ViewNodeRunner<MeshletPrepassNode>>(
                Core3d,
                NodeMeshlet::Prepass,
            )
            .add_render_graph_node::<ViewNodeRunner<MeshletDeferredGBufferPrepassNode>>(
                Core3d,
                NodeMeshlet::DeferredPrepass,
            )
            .add_render_graph_node::<ViewNodeRunner<MeshletMainOpaquePass3dNode>>(
                Core3d,
                NodeMeshlet::MainOpaquePass,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    NodeMeshlet::VisibilityBufferRasterPass,
                    NodePbr::ShadowPass,
                    NodeMeshlet::Prepass,
                    NodeMeshlet::DeferredPrepass,
                    Node3d::Prepass,
                    Node3d::DeferredPrepass,
                    Node3d::CopyDeferredLightingId,
                    Node3d::EndPrepasses,
                    Node3d::StartMainPass,
                    NodeMeshlet::MainOpaquePass,
                    Node3d::MainOpaquePass,
                    Node3d::EndMainPass,
                ),
            )
            .init_resource::<MeshletGpuScene>()
            .init_resource::<MeshletPipelines>()
            .add_systems(ExtractSchedule, extract_meshlet_meshes)
            .add_systems(
                Render,
                (
                    perform_pending_meshlet_mesh_writes.in_set(RenderSet::PrepareAssets),
                    configure_meshlet_views
                        .after(prepare_view_targets)
                        .in_set(RenderSet::ManageViews),
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

/// A convenient alias for `With<Handle<MeshletMesh>>`, for use with
/// [`bevy_render::view::VisibleEntities`].
pub type WithMeshletMesh = With<Handle<MeshletMesh>>;

fn configure_meshlet_views(
    mut views_3d: Query<(
        Entity,
        &mut Camera3d,
        Has<NormalPrepass>,
        Has<MotionVectorPrepass>,
        Has<DeferredPrepass>,
    )>,
    mut commands: Commands,
) {
    for (entity, mut camera_3d, normal_prepass, motion_vector_prepass, deferred_prepass) in
        &mut views_3d
    {
        let mut usages: TextureUsages = camera_3d.depth_texture_usages.into();
        usages |= TextureUsages::TEXTURE_BINDING;
        camera_3d.depth_texture_usages = usages.into();

        if !(normal_prepass || motion_vector_prepass || deferred_prepass) {
            commands
                .entity(entity)
                .insert(MeshletViewMaterialsMainOpaquePass::default());
        } else {
            commands.entity(entity).insert((
                MeshletViewMaterialsMainOpaquePass::default(),
                MeshletViewMaterialsPrepass::default(),
                MeshletViewMaterialsDeferredGBufferPrepass::default(),
            ));
        }
    }
}
