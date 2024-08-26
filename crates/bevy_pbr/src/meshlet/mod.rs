//! Render high-poly 3d meshes using an efficient GPU-driven method. See [`MeshletPlugin`] and [`MeshletMesh`] for details.

mod asset;
#[cfg(feature = "meshlet_processor")]
mod from_mesh;
mod instance_manager;
mod material_pipeline_prepare;
mod material_shade_nodes;
mod meshlet_mesh_manager;
mod persistent_buffer;
mod persistent_buffer_impls;
mod pipelines;
mod resource_manager;
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
    instance_manager::{queue_material_meshlet_meshes, InstanceManager},
    material_pipeline_prepare::{
        prepare_material_meshlet_meshes_main_opaque_pass, prepare_material_meshlet_meshes_prepass,
    },
};

pub use self::asset::{MeshletMesh, MeshletMeshSaverLoader};
#[cfg(feature = "meshlet_processor")]
pub use self::from_mesh::MeshToMeshletMeshConversionError;

use self::{
    graph::NodeMeshlet,
    instance_manager::extract_meshlet_mesh_entities,
    material_pipeline_prepare::{
        MeshletViewMaterialsDeferredGBufferPrepass, MeshletViewMaterialsMainOpaquePass,
        MeshletViewMaterialsPrepass,
    },
    material_shade_nodes::{
        MeshletDeferredGBufferPrepassNode, MeshletMainOpaquePass3dNode, MeshletPrepassNode,
    },
    meshlet_mesh_manager::{perform_pending_meshlet_mesh_writes, MeshletMeshManager},
    pipelines::*,
    resource_manager::{
        prepare_meshlet_per_frame_resources, prepare_meshlet_view_bind_groups, ResourceManager,
    },
    visibility_buffer_raster_node::MeshletVisibilityBufferRasterPassNode,
};
use crate::{graph::NodePbr, Material};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{load_internal_asset, AssetApp, Handle};
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
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
    render_resource::Shader,
    renderer::RenderDevice,
    settings::WgpuFeatures,
    view::{
        check_visibility, prepare_view_targets, InheritedVisibility, Msaa, ViewVisibility,
        Visibility, VisibilitySystems,
    },
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_utils::tracing::error;

const MESHLET_BINDINGS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1325134235233421);
const MESHLET_MESH_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(3325134235233421);

/// Provides a plugin for rendering large amounts of high-poly 3d meshes using an efficient GPU-driven method. See also [`MeshletMesh`].
///
/// Rendering dense scenes made of high-poly meshes with thousands or millions of triangles is extremely expensive in Bevy's standard renderer.
/// Once meshes are pre-processed into a [`MeshletMesh`], this plugin can render these kinds of scenes very efficiently.
///
/// In comparison to Bevy's standard renderer:
/// * Much more efficient culling. Meshlets can be culled individually, instead of all or nothing culling for entire meshes at a time.
///   Additionally, occlusion culling can eliminate meshlets that would cause overdraw.
/// * Much more efficient batching. All geometry can be rasterized in a single draw.
/// * Scales better with large amounts of dense geometry and overdraw. Bevy's standard renderer will bottleneck sooner.
/// * Near-seamless level of detail (LOD).
/// * Much greater base overhead. Rendering will be slower and use more memory than Bevy's standard renderer
///   with small amounts of geometry and overdraw.
/// * Requires preprocessing meshes. See [`MeshletMesh`] for details.
/// * Limitations on the kinds of materials you can use. See [`MeshletMesh`] for details.
///
/// This plugin requires a fairly recent GPU that supports [`WgpuFeatures::SHADER_INT64_ATOMIC_MIN_MAX`].
///
/// This plugin currently works only on the Vulkan backend.
///
/// This plugin is not compatible with [`Msaa`]. Any camera rendering a [`MeshletMesh`] must have
/// [`Msaa`] set to [`Msaa::Off`].
///
/// Mixing forward+prepass and deferred rendering for opaque materials is not currently supported when using this plugin.
/// You must use one or the other by setting [`crate::DefaultOpaqueRendererMethod`].
/// Do not override [`crate::Material::opaque_render_method`] for any material when using this plugin.
///
/// ![A render of the Stanford dragon as a `MeshletMesh`](https://raw.githubusercontent.com/bevyengine/bevy/main/crates/bevy_pbr/src/meshlet/meshlet_preview.png)
pub struct MeshletPlugin {
    /// The maximum amount of clusters that can be processed at once,
    /// used to control the size of a pre-allocated GPU buffer.
    ///
    /// If this number is too low, you'll see rendering artifacts like missing or blinking meshes.
    ///
    /// Each cluster slot costs 4 bytes of VRAM.
    pub cluster_buffer_slots: u32,
}

impl MeshletPlugin {
    /// [`WgpuFeatures`] required for this plugin to function.
    pub fn required_wgpu_features() -> WgpuFeatures {
        WgpuFeatures::SHADER_INT64_ATOMIC_MIN_MAX
            | WgpuFeatures::SHADER_INT64
            | WgpuFeatures::SUBGROUP
            | WgpuFeatures::PUSH_CONSTANTS
    }
}

impl Plugin for MeshletPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_endian = "big")]
        compile_error!("MeshletPlugin is only supported on little-endian processors.");

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
            MESHLET_FILL_CLUSTER_BUFFERS_SHADER_HANDLE,
            "fill_cluster_buffers.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESHLET_CULLING_SHADER_HANDLE,
            "cull_clusters.wgsl",
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
            MESHLET_VISIBILITY_BUFFER_SOFTWARE_RASTER_SHADER_HANDLE,
            "visibility_buffer_software_raster.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESHLET_VISIBILITY_BUFFER_HARDWARE_RASTER_SHADER_HANDLE,
            "visibility_buffer_hardware_raster.wgsl",
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
            MESHLET_RESOLVE_RENDER_TARGETS_SHADER_HANDLE,
            "resolve_render_targets.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESHLET_REMAP_1D_TO_2D_DISPATCH_SHADER_HANDLE,
            "remap_1d_to_2d_dispatch.wgsl",
            Shader::from_wgsl
        );

        app.init_asset::<MeshletMesh>()
            .register_asset_loader(MeshletMeshSaverLoader)
            .add_systems(
                PostUpdate,
                check_visibility::<WithMeshletMesh>.in_set(VisibilitySystems::CheckVisibility),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_device = render_app.world().resource::<RenderDevice>().clone();
        let features = render_device.features();
        if !features.contains(Self::required_wgpu_features()) {
            error!(
                "MeshletPlugin can't be used. GPU lacks support for required features: {:?}.",
                Self::required_wgpu_features().difference(features)
            );
            std::process::exit(1);
        }

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
                    //
                    NodeMeshlet::Prepass,
                    Node3d::Prepass,
                    //
                    NodeMeshlet::DeferredPrepass,
                    Node3d::DeferredPrepass,
                    Node3d::CopyDeferredLightingId,
                    Node3d::EndPrepasses,
                    //
                    Node3d::StartMainPass,
                    NodeMeshlet::MainOpaquePass,
                    Node3d::MainOpaquePass,
                    Node3d::EndMainPass,
                ),
            )
            .init_resource::<MeshletMeshManager>()
            .insert_resource(InstanceManager::new())
            .insert_resource(ResourceManager::new(
                self.cluster_buffer_slots,
                &render_device,
            ))
            .init_resource::<MeshletPipelines>()
            .add_systems(ExtractSchedule, extract_meshlet_mesh_entities)
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
        &Msaa,
        Has<NormalPrepass>,
        Has<MotionVectorPrepass>,
        Has<DeferredPrepass>,
    )>,
    mut commands: Commands,
) {
    for (entity, msaa, normal_prepass, motion_vector_prepass, deferred_prepass) in &mut views_3d {
        if *msaa != Msaa::Off {
            error!("MeshletPlugin can't be used with MSAA. Add Msaa::Off to your camera to use this plugin.");
            std::process::exit(1);
        }

        if !(normal_prepass || motion_vector_prepass || deferred_prepass) {
            commands
                .entity(entity)
                .insert(MeshletViewMaterialsMainOpaquePass::default());
        } else {
            // TODO: Should we add both Prepass and DeferredGBufferPrepass materials here, and in other systems/nodes?
            commands.entity(entity).insert((
                MeshletViewMaterialsMainOpaquePass::default(),
                MeshletViewMaterialsPrepass::default(),
                MeshletViewMaterialsDeferredGBufferPrepass::default(),
            ));
        }
    }
}
