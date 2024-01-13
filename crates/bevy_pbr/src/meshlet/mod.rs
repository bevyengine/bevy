//! Render high-poly 3d meshes using an efficient GPU-driven method. See [`MeshletPlugin`] and [`MeshletMesh`] for details.

mod asset;
#[cfg(feature = "meshopt")]
mod from_mesh;
mod gpu_scene;
mod material_draw_nodes;
mod material_draw_prepare;
mod persistent_buffer;
mod persistent_buffer_impls;
mod pipelines;
mod visibility_buffer_raster_node;

pub(crate) use self::{
    gpu_scene::{queue_material_meshlet_meshes, MeshletGpuScene},
    material_draw_prepare::{
        prepare_material_meshlet_meshes_main_opaque_pass, prepare_material_meshlet_meshes_prepass,
    },
};

pub use self::asset::{Meshlet, MeshletBoundingSphere, MeshletMesh};
#[cfg(feature = "meshopt")]
pub use self::from_mesh::MeshToMeshletMeshConversionError;

use self::{
    asset::MeshletMeshSaverLoad,
    gpu_scene::{
        extract_meshlet_meshes, perform_pending_meshlet_mesh_writes,
        prepare_meshlet_per_frame_resources, prepare_meshlet_view_bind_groups,
    },
    material_draw_nodes::{
        draw_3d_graph::node::{
            MESHLET_DEFERRED_PREPASS, MESHLET_MAIN_OPAQUE_PASS_3D, MESHLET_PREPASS,
        },
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
    visibility_buffer_raster_node::{
        draw_3d_graph::node::MESHLET_VISIBILITY_BUFFER_RASTER_PASS,
        MeshletVisibilityBufferRasterPassNode,
    },
};
use crate::{draw_3d_graph::node::SHADOW_PASS, Material};
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetApp, Handle};
use bevy_core_pipeline::{
    core_3d::{graph::node::*, Camera3d, CORE_3D},
    prepass::{DeferredPrepass, MotionVectorPrepass, NormalPrepass},
};
use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    query::Has,
    schedule::IntoSystemConfigs,
    system::{Commands, Query},
};
use bevy_render::{
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{Shader, TextureUsages},
    renderer::RenderDevice,
    settings::WgpuFeatures,
    view::{prepare_view_targets, InheritedVisibility, Msaa, ViewVisibility, Visibility},
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_utils::tracing::warn;

const MESHLET_BINDINGS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1325134235233421);
const MESHLET_VISIBILITY_BUFFER_RESOLVE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(2325134235233421);
const MESHLET_MESH_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(3325134235233421);

/// Provides a plugin for rendering high-poly 3d meshes using an efficient GPU-driven method. See also [`MeshletMesh`].
///
/// Rendering high-poly meshes with thousands or millions of triangles is extremely expensive in Bevy's standard renderer.
/// Once pre-processed into a [`MeshletMesh`], this plugin can render these kinds of meshes very efficently via the following method:
/// * All work is done on the GPU. Minimal CPU processing is required, unlike Bevy's standard renderer.
/// * Individual meshlets outside of the camera's frustum are culled (unlike Bevy's standard renderer, which can only cull entire meshes).
/// * All meshlets that were visible last frame (and are in the camea's frustum this frame) get rendered to a depth buffer.
/// * The depth buffer is then downsampled to form a hierarchical depth buffer.
/// * All meshlets that were not _not_ visible last frame get frustum culled and tested against the depth buffer, and culled if they would not be visible (occlusion culling, which Bevy's standard renderer does not have).
/// * A visibility buffer is then rendered for the surviving frustum and occlusion culled meshlets. Each pixel of the texture encodes the visible meshlet and triangle ID.
/// * For the opaque and prepass phases, one draw per [`Material`] batch (unique pipeline + bind group) is performed, regardless of the amount of entities using that material.
///   The material's fragment shader reads the meshlet and triangle IDs from the visibility buffer to reconstruct the rendered point on the mesh and shade it.
///
/// This plugin is not compatible with [`Msaa`], and adding this plugin will disable it.
///
/// This plugin requires support for [push constants](WgpuFeatures#associatedconstant.PUSH_CONSTANTS), and will not work on web platforms.
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
            MESHLET_VISIBILITY_BUFFER_RESOLVE_SHADER_HANDLE,
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
            .insert_resource(Msaa::Off);
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        if !render_app
            .world
            .resource::<RenderDevice>()
            .features()
            .contains(WgpuFeatures::PUSH_CONSTANTS)
        {
            warn!("MeshletPlugin not loaded. GPU lacks support for WgpuFeatures::PUSH_CONSTANTS.");
            return;
        }

        render_app
            .add_render_graph_node::<MeshletVisibilityBufferRasterPassNode>(
                CORE_3D,
                MESHLET_VISIBILITY_BUFFER_RASTER_PASS,
            )
            .add_render_graph_node::<ViewNodeRunner<MeshletPrepassNode>>(CORE_3D, MESHLET_PREPASS)
            .add_render_graph_node::<ViewNodeRunner<MeshletDeferredGBufferPrepassNode>>(
                CORE_3D,
                MESHLET_DEFERRED_PREPASS,
            )
            .add_render_graph_node::<ViewNodeRunner<MeshletMainOpaquePass3dNode>>(
                CORE_3D,
                MESHLET_MAIN_OPAQUE_PASS_3D,
            )
            .add_render_graph_edges(
                CORE_3D,
                &[
                    MESHLET_VISIBILITY_BUFFER_RASTER_PASS,
                    SHADOW_PASS,
                    MESHLET_PREPASS,
                    MESHLET_DEFERRED_PREPASS,
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
                    configure_meshlet_views
                        .after(prepare_view_targets)
                        .in_set(RenderSet::ManageViews),
                    prepare_meshlet_per_frame_resources.in_set(RenderSet::PrepareResources),
                    prepare_meshlet_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
                ),
            );
    }
}

/// Sets up dummy shaders for when [`MeshletPlugin`] is not used to prevent shader import errors.
pub struct MeshletDummyShaderPlugin;

impl Plugin for MeshletDummyShaderPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            MESHLET_VISIBILITY_BUFFER_RESOLVE_SHADER_HANDLE,
            "dummy_visibility_buffer_resolve.wgsl",
            Shader::from_wgsl
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
