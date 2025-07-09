#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

extern crate alloc;

#[cfg(feature = "meshlet")]
mod meshlet;
pub mod wireframe;

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

mod atmosphere;
mod cluster;
mod components;
pub mod decal;
pub mod deferred;
mod extended_material;
mod fog;
mod light_probe;
mod lightmap;
mod material;
mod material_bind_groups;
mod mesh_material;
mod parallax;
mod pbr_material;
mod prepass;
mod render;
mod ssao;
mod ssr;
mod volumetric_fog;

use bevy_color::{Color, LinearRgba};

pub use atmosphere::*;
use bevy_light::SimulationLightSystems;
pub use bevy_light::{
    light_consts, AmbientLight, CascadeShadowConfig, CascadeShadowConfigBuilder, Cascades,
    ClusteredDecal, DirectionalLight, DirectionalLightShadowMap, DirectionalLightTexture,
    FogVolume, IrradianceVolume, LightPlugin, LightProbe, NotShadowCaster, NotShadowReceiver,
    PointLight, PointLightShadowMap, PointLightTexture, ShadowFilteringMethod, SpotLight,
    SpotLightTexture, TransmittedShadowReceiver, VolumetricFog, VolumetricLight,
};
pub use cluster::*;
pub use components::*;
pub use decal::clustered::ClusteredDecalPlugin;
pub use extended_material::*;
pub use fog::*;
pub use light_probe::*;
pub use lightmap::*;
pub use material::*;
pub use material_bind_groups::*;
pub use mesh_material::*;
pub use parallax::*;
pub use pbr_material::*;
pub use prepass::*;
pub use render::*;
pub use ssao::*;
pub use ssr::*;
pub use volumetric_fog::VolumetricFogPlugin;

/// The PBR prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        fog::{DistanceFog, FogFalloff},
        material::{Material, MaterialPlugin},
        mesh_material::MeshMaterial3d,
        parallax::ParallaxMappingMethod,
        pbr_material::StandardMaterial,
        ssao::ScreenSpaceAmbientOcclusionPlugin,
    };
    #[doc(hidden)]
    pub use bevy_light::{
        light_consts, AmbientLight, DirectionalLight, EnvironmentMapLight, LightProbe, PointLight,
        SpotLight,
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
        /// Label for the volumetric lighting pass.
        VolumetricFog,
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

use crate::{deferred::DeferredPbrLightingPlugin, graph::NodePbr};
use bevy_app::prelude::*;
use bevy_asset::{AssetApp, AssetPath, Assets, Handle};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_render::{
    alpha::AlphaMode,
    camera::{sort_cameras, Projection},
    extract_component::ExtractComponentPlugin,
    extract_resource::ExtractResourcePlugin,
    load_shader_library,
    render_graph::RenderGraph,
    render_resource::ShaderRef,
    sync_component::SyncComponentPlugin,
    ExtractSchedule, Render, RenderApp, RenderDebugFlags, RenderSystems,
};

use std::path::PathBuf;

fn shader_ref(path: PathBuf) -> ShaderRef {
    ShaderRef::Path(AssetPath::from_path_buf(path).with_source("embedded"))
}

pub const TONEMAPPING_LUT_TEXTURE_BINDING_INDEX: u32 = 18;
pub const TONEMAPPING_LUT_SAMPLER_BINDING_INDEX: u32 = 19;

/// Sets up the entire PBR infrastructure of bevy.
pub struct PbrPlugin {
    /// Controls if the prepass is enabled for the [`StandardMaterial`].
    /// For more information about what a prepass is, see the [`bevy_core_pipeline::prepass`] docs.
    pub prepass_enabled: bool,
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

impl Default for PbrPlugin {
    fn default() -> Self {
        Self {
            prepass_enabled: true,
            add_default_deferred_lighting_plugin: true,
            use_gpu_instance_buffer_builder: true,
            debug_flags: RenderDebugFlags::default(),
        }
    }
}

impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "render/pbr_types.wgsl");
        load_shader_library!(app, "render/pbr_bindings.wgsl");
        load_shader_library!(app, "render/utils.wgsl");
        load_shader_library!(app, "render/clustered_forward.wgsl");
        load_shader_library!(app, "render/pbr_lighting.wgsl");
        load_shader_library!(app, "render/pbr_transmission.wgsl");
        load_shader_library!(app, "render/shadows.wgsl");
        load_shader_library!(app, "deferred/pbr_deferred_types.wgsl");
        load_shader_library!(app, "deferred/pbr_deferred_functions.wgsl");
        load_shader_library!(app, "render/shadow_sampling.wgsl");
        load_shader_library!(app, "render/pbr_functions.wgsl");
        load_shader_library!(app, "render/rgb9e5.wgsl");
        load_shader_library!(app, "render/pbr_ambient.wgsl");
        load_shader_library!(app, "render/pbr_fragment.wgsl");
        load_shader_library!(app, "render/pbr.wgsl");
        load_shader_library!(app, "render/pbr_prepass_functions.wgsl");
        load_shader_library!(app, "render/pbr_prepass.wgsl");
        load_shader_library!(app, "render/parallax_mapping.wgsl");
        load_shader_library!(app, "render/view_transformations.wgsl");

        // Setup dummy shaders for when MeshletPlugin is not used to prevent shader import errors.
        load_shader_library!(app, "meshlet/dummy_visibility_buffer_resolve.wgsl");

        app.register_asset_reflect::<StandardMaterial>()
            .register_type::<DefaultOpaqueRendererMethod>()
            .init_resource::<DefaultOpaqueRendererMethod>()
            .add_plugins((
                MeshRenderPlugin {
                    use_gpu_instance_buffer_builder: self.use_gpu_instance_buffer_builder,
                    debug_flags: self.debug_flags,
                },
                MaterialsPlugin {
                    debug_flags: self.debug_flags,
                },
                MaterialPlugin::<StandardMaterial> {
                    prepass_enabled: self.prepass_enabled,
                    debug_flags: self.debug_flags,
                    ..Default::default()
                },
                ScreenSpaceAmbientOcclusionPlugin,
                ExtractResourcePlugin::<AmbientLight>::default(),
                FogPlugin,
                ExtractResourcePlugin::<DefaultOpaqueRendererMethod>::default(),
                ExtractComponentPlugin::<ShadowFilteringMethod>::default(),
                LightmapPlugin,
                LightProbePlugin,
                LightPlugin,
                GpuMeshPreprocessPlugin {
                    use_gpu_instance_buffer_builder: self.use_gpu_instance_buffer_builder,
                },
                VolumetricFogPlugin,
                ScreenSpaceReflectionsPlugin,
                ClusteredDecalPlugin,
            ))
            .add_plugins((
                decal::ForwardDecalPlugin,
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
            );

        if self.add_default_deferred_lighting_plugin {
            app.add_plugins(DeferredPbrLightingPlugin);
        }

        // Initialize the default material handle.
        app.world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .insert(
                &Handle::<StandardMaterial>::default(),
                StandardMaterial {
                    base_color: Color::srgb(1.0, 0.0, 0.5),
                    ..Default::default()
                },
            );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // Extract the required data from the main world
        render_app
            .add_systems(
                ExtractSchedule,
                (
                    extract_clusters,
                    extract_lights,
                    late_sweep_material_instances,
                ),
            )
            .add_systems(
                Render,
                (
                    prepare_lights
                        .in_set(RenderSystems::ManageViews)
                        .after(sort_cameras),
                    prepare_clusters.in_set(RenderSystems::PrepareResources),
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

        let global_cluster_settings = make_global_cluster_settings(render_app.world());
        app.insert_resource(global_cluster_settings);
    }
}
