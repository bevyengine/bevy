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

pub use self::asset::{
    MeshletMesh, MeshletMeshLoader, MeshletMeshSaver, MESHLET_MESH_ASSET_VERSION,
};
#[cfg(feature = "meshlet_processor")]
pub use self::from_mesh::{
    MeshToMeshletMeshConversionError, MESHLET_DEFAULT_VERTEX_POSITION_QUANTIZATION_FACTOR,
};
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
    meshlet_mesh_manager::perform_pending_meshlet_mesh_writes,
    pipelines::*,
    resource_manager::{
        prepare_meshlet_per_frame_resources, prepare_meshlet_view_bind_groups, ResourceManager,
    },
    visibility_buffer_raster_node::MeshletVisibilityBufferRasterPassNode,
};
use crate::{
    graph::NodePbr, meshlet::meshlet_mesh_manager::init_meshlet_mesh_manager,
    PreviousGlobalTransform,
};
use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, AssetApp, AssetId, Handle};
use bevy_camera::visibility::{self, Visibility, VisibilityClass};
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    prepass::{DeferredPrepass, MotionVectorPrepass, NormalPrepass},
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Has,
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    render_graph::{RenderGraphExt, ViewNodeRunner},
    renderer::RenderDevice,
    settings::WgpuFeatures,
    view::{prepare_view_targets, Msaa},
    ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::load_shader_library;
use bevy_transform::components::Transform;
use derive_more::From;
use tracing::error;

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
/// This plugin requires a fairly recent GPU that supports [`WgpuFeatures::TEXTURE_INT64_ATOMIC`].
///
/// This plugin currently works only on the Vulkan and Metal backends.
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
    ///
    /// Must not be greater than 2^25.
    pub cluster_buffer_slots: u32,
}

impl MeshletPlugin {
    /// [`WgpuFeatures`] required for this plugin to function.
    pub fn required_wgpu_features() -> WgpuFeatures {
        WgpuFeatures::TEXTURE_INT64_ATOMIC
            | WgpuFeatures::TEXTURE_ATOMIC
            | WgpuFeatures::SHADER_INT64
            | WgpuFeatures::SUBGROUP
            | WgpuFeatures::DEPTH_CLIP_CONTROL
            | WgpuFeatures::PUSH_CONSTANTS
    }
}

impl Plugin for MeshletPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_endian = "big")]
        compile_error!("MeshletPlugin is only supported on little-endian processors.");

        if self.cluster_buffer_slots > 2_u32.pow(25) {
            error!("MeshletPlugin::cluster_buffer_slots must not be greater than 2^25.");
            std::process::exit(1);
        }

        load_shader_library!(app, "meshlet_bindings.wgsl");
        load_shader_library!(app, "visibility_buffer_resolve.wgsl");
        load_shader_library!(app, "meshlet_cull_shared.wgsl");
        embedded_asset!(app, "clear_visibility_buffer.wgsl");
        embedded_asset!(app, "cull_instances.wgsl");
        embedded_asset!(app, "cull_bvh.wgsl");
        embedded_asset!(app, "cull_clusters.wgsl");
        embedded_asset!(app, "visibility_buffer_software_raster.wgsl");
        embedded_asset!(app, "visibility_buffer_hardware_raster.wgsl");
        embedded_asset!(app, "meshlet_mesh_material.wgsl");
        embedded_asset!(app, "resolve_render_targets.wgsl");
        embedded_asset!(app, "remap_1d_to_2d_dispatch.wgsl");
        embedded_asset!(app, "fill_counts.wgsl");

        app.init_asset::<MeshletMesh>()
            .register_asset_loader(MeshletMeshLoader);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // Create a variable here so we can move-capture it.
        let cluster_buffer_slots = self.cluster_buffer_slots;
        let init_resource_manager_system =
            move |mut commands: Commands, render_device: Res<RenderDevice>| {
                commands
                    .insert_resource(ResourceManager::new(cluster_buffer_slots, &render_device));
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
                    NodePbr::EarlyShadowPass,
                    //
                    NodeMeshlet::Prepass,
                    //
                    NodeMeshlet::DeferredPrepass,
                    Node3d::EndPrepasses,
                    //
                    Node3d::StartMainPass,
                    NodeMeshlet::MainOpaquePass,
                    Node3d::MainOpaquePass,
                    Node3d::EndMainPass,
                ),
            )
            .insert_resource(InstanceManager::new())
            .add_systems(
                RenderStartup,
                (
                    check_meshlet_features,
                    (
                        (init_resource_manager_system, init_meshlet_pipelines).chain(),
                        init_meshlet_mesh_manager,
                    ),
                )
                    .chain(),
            )
            .add_systems(ExtractSchedule, extract_meshlet_mesh_entities)
            .add_systems(
                Render,
                (
                    perform_pending_meshlet_mesh_writes.in_set(RenderSystems::PrepareAssets),
                    configure_meshlet_views
                        .after(prepare_view_targets)
                        .in_set(RenderSystems::ManageViews),
                    prepare_meshlet_per_frame_resources.in_set(RenderSystems::PrepareResources),
                    prepare_meshlet_view_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                    queue_material_meshlet_meshes.in_set(RenderSystems::QueueMeshes),
                    prepare_material_meshlet_meshes_main_opaque_pass
                        .in_set(RenderSystems::QueueMeshes)
                        .before(queue_material_meshlet_meshes),
                ),
            );
    }
}

fn check_meshlet_features(render_device: Res<RenderDevice>) {
    let features = render_device.features();
    if !features.contains(MeshletPlugin::required_wgpu_features()) {
        error!(
            "MeshletPlugin can't be used. GPU lacks support for required features: {:?}.",
            MeshletPlugin::required_wgpu_features().difference(features)
        );
        std::process::exit(1);
    }
}

/// The meshlet mesh equivalent of [`bevy_mesh::Mesh3d`].
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default, Clone, PartialEq)]
#[require(Transform, PreviousGlobalTransform, Visibility, VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<MeshletMesh3d>)]
pub struct MeshletMesh3d(pub Handle<MeshletMesh>);

impl From<MeshletMesh3d> for AssetId<MeshletMesh> {
    fn from(mesh: MeshletMesh3d) -> Self {
        mesh.id()
    }
}

impl From<&MeshletMesh3d> for AssetId<MeshletMesh> {
    fn from(mesh: &MeshletMesh3d) -> Self {
        mesh.id()
    }
}

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
