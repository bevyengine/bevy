#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

extern crate alloc;

#[cfg(feature = "meshlet")]
mod meshlet;

/// Experimental features that are not yet finished. Please report any issues you encounter!
///
/// Expect bugs, missing features, compatibility issues, low performance, and/or future breaking changes.
#[cfg(feature = "meshlet")]
pub mod experimental {
    /// Render high-poly 3d meshes using an efficient GPU-driven method.
    /// See [`MeshletPlugin`](meshlet::MeshletPlugin) and [`MeshletMesh`](meshlet::MeshletMesh) for details.
    pub mod meshlet {
        pub use crate::meshlet::*;
    }
}

mod cluster;
mod components;
pub mod decal;
pub mod deferred;
mod extended_material;
mod fog;
mod light;
mod light_probe;
mod lightmap;
mod material;
mod material_bind_groups;
mod mesh_material;
mod prepass;
mod render;
mod ssao;
mod ssr;

use bevy_color::Color;

pub use cluster::*;
pub use components::*;
pub use decal::clustered::ClusteredDecalPlugin;
pub use extended_material::*;
pub use fog::*;
pub use light::*;
pub use light_probe::*;
pub use lightmap::*;
pub use material::*;
pub use material_bind_groups::*;
pub use mesh_material::*;
pub use prepass::*;
pub use render::*;
pub use ssao::*;
pub use ssr::*;

/// The PBR prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        fog::{DistanceFog, FogFalloff},
        light::{light_consts, AmbientLight, DirectionalLight, PointLight, SpotLight},
        light_probe::{environment_map::EnvironmentMapLight, LightProbe},
        material::{Material, MaterialPlugin},
        mesh_material::MeshMaterial3d,
        ssao::ScreenSpaceAmbientOcclusionPlugin,
    };
}

pub mod graph {
    use bevy_render::render_graph::RenderLabel;

    /// Render graph nodes specific to 3D PBR rendering.
    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum NodePbr {
        /// Label for the shadow pass node that draws meshes that were visible
        /// from the light last frame.
        EarlyShadowPass,
        /// Label for the shadow pass node that draws meshes that became visible
        /// from the light this frame.
        LateShadowPass,
        /// Label for the screen space ambient occlusion render node.
        ScreenSpaceAmbientOcclusion,
        DeferredLightingPass,
        /// Label for the shader that transforms and culls meshes that were
        /// visible last frame.
        EarlyGpuPreprocess,
        /// Label for the shader that transforms and culls meshes that became
        /// visible this frame.
        LateGpuPreprocess,
        /// Label for the screen space reflections pass.
        ScreenSpaceReflections,
        /// Label for the node that builds indirect draw parameters for meshes
        /// that were visible last frame.
        EarlyPrepassBuildIndirectParameters,
        /// Label for the node that builds indirect draw parameters for meshes
        /// that became visible this frame.
        LatePrepassBuildIndirectParameters,
        /// Label for the node that builds indirect draw parameters for the main
        /// rendering pass, containing all meshes that are visible this frame.
        MainBuildIndirectParameters,
        ClearIndirectParametersMetadata,
    }
}

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, weak_handle, AssetApp, Handle};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_render::{
    alpha::AlphaMode,
    camera::{sort_cameras, CameraUpdateSystem, Projection},
    extract_component::ExtractComponentPlugin,
    extract_resource::ExtractResourcePlugin,
    render_graph::RenderGraph,
    render_resource::Shader,
    sync_component::SyncComponentPlugin,
    view::VisibilitySystems,
    ExtractSchedule, Render, RenderApp, RenderDebugFlags, RenderSet,
};
use bevy_transform::TransformSystem;

use crate::{deferred::DeferredPbrLightingPlugin, graph::NodePbr};

const UTILS_HANDLE: Handle<Shader> = weak_handle!("0a32978f-2744-4608-98b6-4c3000a0638d");
const CLUSTERED_FORWARD_HANDLE: Handle<Shader> =
    weak_handle!("f8e3b4c6-60b7-4b23-8b2e-a6b27bb4ddce");
const PBR_LIGHTING_HANDLE: Handle<Shader> = weak_handle!("de0cf697-2876-49a0-aa0f-f015216f70c2");
const PBR_TRANSMISSION_HANDLE: Handle<Shader> =
    weak_handle!("22482185-36bb-4c16-9b93-a20e6d4a2725");
const SHADOWS_HANDLE: Handle<Shader> = weak_handle!("ff758c5a-3927-4a15-94c3-3fbdfc362590");
const SHADOW_SAMPLING_HANDLE: Handle<Shader> = weak_handle!("f6bf5843-54bc-4e39-bd9d-56bfcd77b033");
const VIEW_TRANSFORMATIONS_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("ec047703-cde3-4876-94df-fed121544abb");
const PBR_DEFERRED_TYPES_HANDLE: Handle<Shader> =
    weak_handle!("43060da7-a717-4240-80a8-dbddd92bd25d");
const PBR_DEFERRED_FUNCTIONS_HANDLE: Handle<Shader> =
    weak_handle!("9dc46746-c51d-45e3-a321-6a50c3963420");
const MESHLET_VISIBILITY_BUFFER_RESOLVE_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("69187376-3dea-4d0f-b3f5-185bde63d6a2");

const TONEMAPPING_LUT_TEXTURE_BINDING_INDEX: u32 = 26;
const TONEMAPPING_LUT_SAMPLER_BINDING_INDEX: u32 = 27;

/// Sets up the entire 3d rendering infrastructure of bevy.
pub struct MeshPipelinePlugin {
    /// Controls if [`DeferredPbrLightingPlugin`] is added.
    pub add_default_deferred_lighting_plugin: bool,
    /// Controls if GPU [`MeshUniform`] building is enabled.
    ///
    /// This requires compute shader support and so will be forcibly disabled if
    /// the platform doesn't support those.
    pub use_gpu_instance_buffer_builder: bool,
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
}

impl Default for MeshPipelinePlugin {
    fn default() -> Self {
        Self {
            add_default_deferred_lighting_plugin: true,
            use_gpu_instance_buffer_builder: true,
            debug_flags: RenderDebugFlags::default(),
        }
    }
}

impl Plugin for MeshPipelinePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, UTILS_HANDLE, "render/utils.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            CLUSTERED_FORWARD_HANDLE,
            "render/clustered_forward.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_LIGHTING_HANDLE,
            "render/pbr_lighting.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_TRANSMISSION_HANDLE,
            "render/pbr_transmission.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SHADOWS_HANDLE,
            "render/shadows.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_DEFERRED_TYPES_HANDLE,
            "deferred/pbr_deferred_types.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_DEFERRED_FUNCTIONS_HANDLE,
            "deferred/pbr_deferred_functions.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SHADOW_SAMPLING_HANDLE,
            "render/shadow_sampling.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            VIEW_TRANSFORMATIONS_SHADER_HANDLE,
            "render/view_transformations.wgsl",
            Shader::from_wgsl
        );
        // Setup dummy shaders for when MeshletPlugin is not used to prevent shader import errors.
        load_internal_asset!(
            app,
            MESHLET_VISIBILITY_BUFFER_RESOLVE_SHADER_HANDLE,
            "meshlet/dummy_visibility_buffer_resolve.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<AmbientLight>()
            .register_type::<CascadeShadowConfig>()
            .register_type::<Cascades>()
            .register_type::<CascadesVisibleEntities>()
            .register_type::<VisibleMeshEntities>()
            .register_type::<ClusterConfig>()
            .register_type::<CubemapVisibleEntities>()
            .register_type::<DirectionalLight>()
            .register_type::<DirectionalLightShadowMap>()
            .register_type::<NotShadowCaster>()
            .register_type::<NotShadowReceiver>()
            .register_type::<PointLight>()
            .register_type::<PointLightShadowMap>()
            .register_type::<SpotLight>()
            .register_type::<ShadowFilteringMethod>()
            .init_resource::<AmbientLight>()
            .init_resource::<GlobalVisibleClusterableObjects>()
            .init_resource::<DirectionalLightShadowMap>()
            .init_resource::<PointLightShadowMap>()
            .register_type::<DefaultOpaqueRendererMethod>()
            .init_resource::<DefaultOpaqueRendererMethod>();
        #[cfg(feature = "volumetric_light")]
        app.register_type::<VolumetricLight>();

        app.add_plugins((
            MeshRenderPlugin {
                use_gpu_instance_buffer_builder: self.use_gpu_instance_buffer_builder,
                debug_flags: self.debug_flags,
            },
            ScreenSpaceAmbientOcclusionPlugin,
            ExtractResourcePlugin::<AmbientLight>::default(),
            FogPlugin,
            ExtractResourcePlugin::<DefaultOpaqueRendererMethod>::default(),
            ExtractComponentPlugin::<ShadowFilteringMethod>::default(),
            LightmapPlugin,
            LightProbePlugin,
            PbrProjectionPlugin,
            GpuMeshPreprocessPlugin {
                use_gpu_instance_buffer_builder: self.use_gpu_instance_buffer_builder,
            },
            ScreenSpaceReflectionsPlugin,
            ClusteredDecalPlugin,
        ))
        .add_plugins((
            SyncComponentPlugin::<DirectionalLight>::default(),
            SyncComponentPlugin::<PointLight>::default(),
            SyncComponentPlugin::<SpotLight>::default(),
            ExtractComponentPlugin::<AmbientLight>::default(),
        ))
        .add_plugins(AtmospherePlugin)
        .configure_sets(
            PostUpdate,
            (
                SimulationLightSystems::AddClusters,
                SimulationLightSystems::AssignLightsToClusters,
            )
                .chain(),
        )
        .configure_sets(
            PostUpdate,
            SimulationLightSystems::UpdateDirectionalLightCascades
                .ambiguous_with(SimulationLightSystems::UpdateDirectionalLightCascades),
        )
        .configure_sets(
            PostUpdate,
            SimulationLightSystems::CheckLightVisibility
                .ambiguous_with(SimulationLightSystems::CheckLightVisibility),
        )
        .add_systems(
            PostUpdate,
            (
                add_clusters
                    .in_set(SimulationLightSystems::AddClusters)
                    .after(CameraUpdateSystem),
                assign_objects_to_clusters
                    .in_set(SimulationLightSystems::AssignLightsToClusters)
                    .after(TransformSystem::TransformPropagate)
                    .after(VisibilitySystems::CheckVisibility)
                    .after(CameraUpdateSystem),
                clear_directional_light_cascades
                    .in_set(SimulationLightSystems::UpdateDirectionalLightCascades)
                    .after(TransformSystem::TransformPropagate)
                    .after(CameraUpdateSystem),
                update_directional_light_frusta
                    .in_set(SimulationLightSystems::UpdateLightFrusta)
                    // This must run after CheckVisibility because it relies on `ViewVisibility`
                    .after(VisibilitySystems::CheckVisibility)
                    .after(TransformSystem::TransformPropagate)
                    .after(SimulationLightSystems::UpdateDirectionalLightCascades)
                    // We assume that no entity will be both a directional light and a spot light,
                    // so these systems will run independently of one another.
                    // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                    .ambiguous_with(update_spot_light_frusta),
                update_point_light_frusta
                    .in_set(SimulationLightSystems::UpdateLightFrusta)
                    .after(TransformSystem::TransformPropagate)
                    .after(SimulationLightSystems::AssignLightsToClusters),
                update_spot_light_frusta
                    .in_set(SimulationLightSystems::UpdateLightFrusta)
                    .after(TransformSystem::TransformPropagate)
                    .after(SimulationLightSystems::AssignLightsToClusters),
                (
                    check_dir_light_mesh_visibility,
                    check_point_light_mesh_visibility,
                )
                    .in_set(SimulationLightSystems::CheckLightVisibility)
                    .after(VisibilitySystems::CalculateBounds)
                    .after(TransformSystem::TransformPropagate)
                    .after(SimulationLightSystems::UpdateLightFrusta)
                    // NOTE: This MUST be scheduled AFTER the core renderer visibility check
                    // because that resets entity `ViewVisibility` for the first view
                    // which would override any results from this otherwise
                    .after(VisibilitySystems::CheckVisibility)
                    .before(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible),
            ),
        );

        if self.add_default_deferred_lighting_plugin {
            app.add_plugins(DeferredPbrLightingPlugin);
        }

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // Extract the required data from the main world
        render_app
            .add_systems(ExtractSchedule, (extract_clusters, extract_lights))
            .add_systems(
                Render,
                (
                    prepare_lights
                        .in_set(RenderSet::ManageViews)
                        .after(sort_cameras),
                    prepare_clusters.in_set(RenderSet::PrepareResources),
                ),
            )
            .init_resource::<LightMeta>()
            .init_resource::<RenderMaterialBindings>();

        render_app.world_mut().add_observer(add_light_view_entities);
        render_app
            .world_mut()
            .add_observer(remove_light_view_entities);
        render_app.world_mut().add_observer(extracted_light_removed);

        let early_shadow_pass_node = EarlyShadowPassNode::from_world(render_app.world_mut());
        let late_shadow_pass_node = LateShadowPassNode::from_world(render_app.world_mut());
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        let draw_3d_graph = graph.get_sub_graph_mut(Core3d).unwrap();
        draw_3d_graph.add_node(NodePbr::EarlyShadowPass, early_shadow_pass_node);
        draw_3d_graph.add_node(NodePbr::LateShadowPass, late_shadow_pass_node);
        draw_3d_graph.add_node_edges((
            NodePbr::EarlyShadowPass,
            NodePbr::LateShadowPass,
            Node3d::StartMainPass,
        ));
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // Extract the required data from the main world
        render_app
            .init_resource::<ShadowSamplers>()
            .init_resource::<GlobalClusterableObjectMeta>()
            .init_resource::<FallbackBindlessResources>();
    }
}

/// Camera projection PBR functionality.
#[derive(Default)]
pub struct PbrProjectionPlugin;
impl Plugin for PbrProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            build_directional_light_cascades
                .in_set(SimulationLightSystems::UpdateDirectionalLightCascades)
                .after(clear_directional_light_cascades),
        );
    }
}
