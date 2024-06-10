// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

#[cfg(feature = "meshlet")]
mod meshlet;
pub mod wireframe;

/// Experimental features that are not yet finished. Please report any issues you encounter!
///
/// Expect bugs, missing features, compatibility issues, low performance, and/or future breaking changes.
#[cfg(feature = "meshlet")]
pub mod experimental {
    /// Render high-poly 3d meshes using an efficient GPU-driven method. See [`MeshletPlugin`] and [`MeshletMesh`] for details.
    pub mod meshlet {
        pub use crate::meshlet::*;
    }
}

mod bundle;
mod cluster;
pub mod deferred;
mod extended_material;
mod fog;
mod light;
mod light_probe;
mod lightmap;
mod material;
mod parallax;
mod pbr_material;
mod prepass;
mod render;
mod ssao;
mod ssr;
mod volumetric_fog;

use bevy_color::{Color, LinearRgba};
use std::marker::PhantomData;

pub use bundle::*;
pub use cluster::*;
pub use extended_material::*;
pub use fog::*;
pub use light::*;
pub use light_probe::*;
pub use lightmap::*;
pub use material::*;
pub use parallax::*;
pub use pbr_material::*;
pub use prepass::*;
pub use render::*;
pub use ssao::*;
pub use ssr::*;
pub use volumetric_fog::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        bundle::{
            DirectionalLightBundle, MaterialMeshBundle, PbrBundle, PointLightBundle,
            SpotLightBundle,
        },
        fog::{FogFalloff, FogSettings},
        light::{light_consts, AmbientLight, DirectionalLight, PointLight, SpotLight},
        light_probe::{
            environment_map::{EnvironmentMapLight, ReflectionProbeBundle},
            LightProbe,
        },
        material::{Material, MaterialPlugin},
        parallax::ParallaxMappingMethod,
        pbr_material::StandardMaterial,
        ssao::ScreenSpaceAmbientOcclusionPlugin,
    };
}

pub mod graph {
    use bevy_render::render_graph::RenderLabel;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum NodePbr {
        /// Label for the shadow pass node.
        ShadowPass,
        /// Label for the screen space ambient occlusion render node.
        ScreenSpaceAmbientOcclusion,
        DeferredLightingPass,
        /// Label for the volumetric lighting pass.
        VolumetricFog,
        /// Label for the compute shader instance data building pass.
        GpuPreprocess,
        /// Label for the screen space reflections pass.
        ScreenSpaceReflections,
    }
}

use crate::{deferred::DeferredPbrLightingPlugin, graph::NodePbr};
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, AssetApp, Assets, Handle};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::prelude::*;
use bevy_render::{
    alpha::AlphaMode,
    camera::{
        CameraProjection, CameraUpdateSystem, OrthographicProjection, PerspectiveProjection,
        Projection,
    },
    extract_component::ExtractComponentPlugin,
    extract_resource::ExtractResourcePlugin,
    render_asset::prepare_assets,
    render_graph::RenderGraph,
    render_resource::Shader,
    texture::{GpuImage, Image},
    view::{check_visibility, VisibilitySystems},
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::TransformSystem;

pub const PBR_TYPES_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1708015359337029744);
pub const PBR_BINDINGS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(5635987986427308186);
pub const UTILS_HANDLE: Handle<Shader> = Handle::weak_from_u128(1900548483293416725);
pub const CLUSTERED_FORWARD_HANDLE: Handle<Shader> = Handle::weak_from_u128(166852093121196815);
pub const PBR_LIGHTING_HANDLE: Handle<Shader> = Handle::weak_from_u128(14170772752254856967);
pub const PBR_TRANSMISSION_HANDLE: Handle<Shader> = Handle::weak_from_u128(77319684653223658032);
pub const SHADOWS_HANDLE: Handle<Shader> = Handle::weak_from_u128(11350275143789590502);
pub const SHADOW_SAMPLING_HANDLE: Handle<Shader> = Handle::weak_from_u128(3145627513789590502);
pub const PBR_FRAGMENT_HANDLE: Handle<Shader> = Handle::weak_from_u128(2295049283805286543);
pub const PBR_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(4805239651767701046);
pub const PBR_PREPASS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(9407115064344201137);
pub const PBR_FUNCTIONS_HANDLE: Handle<Shader> = Handle::weak_from_u128(16550102964439850292);
pub const PBR_AMBIENT_HANDLE: Handle<Shader> = Handle::weak_from_u128(2441520459096337034);
pub const PARALLAX_MAPPING_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(17035894873630133905);
pub const VIEW_TRANSFORMATIONS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(2098345702398750291);
pub const PBR_PREPASS_FUNCTIONS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(73204817249182637);
pub const PBR_DEFERRED_TYPES_HANDLE: Handle<Shader> = Handle::weak_from_u128(3221241127431430599);
pub const PBR_DEFERRED_FUNCTIONS_HANDLE: Handle<Shader> = Handle::weak_from_u128(72019026415438599);
pub const RGB9E5_FUNCTIONS_HANDLE: Handle<Shader> = Handle::weak_from_u128(2659010996143919192);
const MESHLET_VISIBILITY_BUFFER_RESOLVE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(2325134235233421);

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
}

impl Default for PbrPlugin {
    fn default() -> Self {
        Self {
            prepass_enabled: true,
            add_default_deferred_lighting_plugin: true,
            use_gpu_instance_buffer_builder: true,
        }
    }
}

impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PBR_TYPES_SHADER_HANDLE,
            "render/pbr_types.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_BINDINGS_SHADER_HANDLE,
            "render/pbr_bindings.wgsl",
            Shader::from_wgsl
        );
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
            PBR_FUNCTIONS_HANDLE,
            "render/pbr_functions.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            RGB9E5_FUNCTIONS_HANDLE,
            "render/rgb9e5.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_AMBIENT_HANDLE,
            "render/pbr_ambient.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_FRAGMENT_HANDLE,
            "render/pbr_fragment.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, PBR_SHADER_HANDLE, "render/pbr.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            PBR_PREPASS_FUNCTIONS_SHADER_HANDLE,
            "render/pbr_prepass_functions.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_PREPASS_SHADER_HANDLE,
            "render/pbr_prepass.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PARALLAX_MAPPING_SHADER_HANDLE,
            "render/parallax_mapping.wgsl",
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

        app.register_asset_reflect::<StandardMaterial>()
            .register_type::<AmbientLight>()
            .register_type::<CascadeShadowConfig>()
            .register_type::<Cascades>()
            .register_type::<CascadesVisibleEntities>()
            .register_type::<ClusterConfig>()
            .register_type::<CubemapVisibleEntities>()
            .register_type::<DirectionalLight>()
            .register_type::<DirectionalLightShadowMap>()
            .register_type::<NotShadowCaster>()
            .register_type::<NotShadowReceiver>()
            .register_type::<PointLight>()
            .register_type::<PointLightShadowMap>()
            .register_type::<SpotLight>()
            .register_type::<FogSettings>()
            .register_type::<ShadowFilteringMethod>()
            .init_resource::<AmbientLight>()
            .init_resource::<GlobalVisibleClusterableObjects>()
            .init_resource::<DirectionalLightShadowMap>()
            .init_resource::<PointLightShadowMap>()
            .register_type::<DefaultOpaqueRendererMethod>()
            .init_resource::<DefaultOpaqueRendererMethod>()
            .add_plugins((
                MeshRenderPlugin {
                    use_gpu_instance_buffer_builder: self.use_gpu_instance_buffer_builder,
                },
                MaterialPlugin::<StandardMaterial> {
                    prepass_enabled: self.prepass_enabled,
                    ..Default::default()
                },
                ScreenSpaceAmbientOcclusionPlugin,
                ExtractResourcePlugin::<AmbientLight>::default(),
                FogPlugin,
                ExtractResourcePlugin::<DefaultOpaqueRendererMethod>::default(),
                ExtractComponentPlugin::<ShadowFilteringMethod>::default(),
                LightmapPlugin,
                LightProbePlugin,
                PbrProjectionPlugin::<Projection>::default(),
                PbrProjectionPlugin::<PerspectiveProjection>::default(),
                PbrProjectionPlugin::<OrthographicProjection>::default(),
                GpuMeshPreprocessPlugin {
                    use_gpu_instance_buffer_builder: self.use_gpu_instance_buffer_builder,
                },
                VolumetricFogPlugin,
                ScreenSpaceReflectionsPlugin,
            ))
            .configure_sets(
                PostUpdate,
                (
                    SimulationLightSystems::AddClusters,
                    SimulationLightSystems::AssignLightsToClusters,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (
                    add_clusters.in_set(SimulationLightSystems::AddClusters),
                    crate::assign_objects_to_clusters
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
                    check_visibility::<WithLight>.in_set(VisibilitySystems::CheckVisibility),
                    check_light_mesh_visibility
                        .in_set(SimulationLightSystems::CheckLightVisibility)
                        .after(VisibilitySystems::CalculateBounds)
                        .after(TransformSystem::TransformPropagate)
                        .after(SimulationLightSystems::UpdateLightFrusta)
                        // NOTE: This MUST be scheduled AFTER the core renderer visibility check
                        // because that resets entity `ViewVisibility` for the first view
                        // which would override any results from this otherwise
                        .after(VisibilitySystems::CheckVisibility),
                ),
            );

        if self.add_default_deferred_lighting_plugin {
            app.add_plugins(DeferredPbrLightingPlugin);
        }

        app.world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .insert(
                &Handle::<StandardMaterial>::default(),
                StandardMaterial {
                    base_color: Color::srgb(1.0, 0.0, 0.5),
                    unlit: true,
                    ..Default::default()
                },
            );

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
                        .after(prepare_assets::<GpuImage>),
                    prepare_clusters.in_set(RenderSet::PrepareResources),
                ),
            )
            .init_resource::<LightMeta>();

        let shadow_pass_node = ShadowPassNode::new(render_app.world_mut());
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        let draw_3d_graph = graph.get_sub_graph_mut(Core3d).unwrap();
        draw_3d_graph.add_node(NodePbr::ShadowPass, shadow_pass_node);
        draw_3d_graph.add_node_edge(NodePbr::ShadowPass, Node3d::StartMainPass);
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // Extract the required data from the main world
        render_app
            .init_resource::<ShadowSamplers>()
            .init_resource::<GlobalClusterableObjectMeta>();
    }
}

/// [`CameraProjection`] specific PBR functionality.
pub struct PbrProjectionPlugin<T: CameraProjection + Component>(PhantomData<T>);
impl<T: CameraProjection + Component> Plugin for PbrProjectionPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            build_directional_light_cascades::<T>
                .in_set(SimulationLightSystems::UpdateDirectionalLightCascades)
                .after(clear_directional_light_cascades),
        );
    }
}
impl<T: CameraProjection + Component> Default for PbrProjectionPlugin<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}
