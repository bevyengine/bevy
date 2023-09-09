#![allow(clippy::type_complexity)]

pub mod wireframe;

mod alpha;
mod bundle;
mod environment_map;
mod fog;
mod light;
mod material;
mod parallax;
mod pbr_material;
mod prepass;
mod render;
mod ssao;

pub use alpha::*;
pub use bundle::*;
pub use environment_map::EnvironmentMapLight;
pub use fog::*;
pub use light::*;
pub use material::*;
pub use parallax::*;
pub use pbr_material::*;
pub use prepass::*;
pub use render::*;
pub use ssao::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        alpha::AlphaMode,
        bundle::{
            DirectionalLightBundle, MaterialMeshBundle, PbrBundle, PointLightBundle,
            SpotLightBundle,
        },
        environment_map::EnvironmentMapLight,
        fog::{FogFalloff, FogSettings},
        light::{AmbientLight, DirectionalLight, PointLight, SpotLight},
        material::{Material, MaterialPlugin},
        parallax::ParallaxMappingMethod,
        pbr_material::StandardMaterial,
        ssao::ScreenSpaceAmbientOcclusionPlugin,
    };
}

pub mod draw_3d_graph {
    pub mod node {
        /// Label for the shadow pass node.
        pub const SHADOW_PASS: &str = "shadow_pass";
    }
}

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, AssetApp, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::CameraUpdateSystem, extract_resource::ExtractResourcePlugin, prelude::Color,
    render_asset::prepare_assets, render_graph::RenderGraph, render_phase::sort_phase_system,
    render_resource::Shader, texture::Image, view::VisibilitySystems, ExtractSchedule, Render,
    RenderApp, RenderSet,
};
use bevy_transform::TransformSystem;
use environment_map::EnvironmentMapPlugin;

pub const PBR_TYPES_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1708015359337029744);
pub const PBR_BINDINGS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(5635987986427308186);
pub const UTILS_HANDLE: Handle<Shader> = Handle::weak_from_u128(1900548483293416725);
pub const CLUSTERED_FORWARD_HANDLE: Handle<Shader> = Handle::weak_from_u128(166852093121196815);
pub const PBR_LIGHTING_HANDLE: Handle<Shader> = Handle::weak_from_u128(14170772752254856967);
pub const SHADOWS_HANDLE: Handle<Shader> = Handle::weak_from_u128(11350275143789590502);
pub const PBR_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(4805239651767701046);
pub const PBR_PREPASS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(9407115064344201137);
pub const PBR_FUNCTIONS_HANDLE: Handle<Shader> = Handle::weak_from_u128(16550102964439850292);
pub const PBR_AMBIENT_HANDLE: Handle<Shader> = Handle::weak_from_u128(2441520459096337034);
pub const PARALLAX_MAPPING_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(17035894873630133905);

/// Sets up the entire PBR infrastructure of bevy.
pub struct PbrPlugin {
    /// Controls if the prepass is enabled for the StandardMaterial.
    /// For more information about what a prepass is, see the [`bevy_core_pipeline::prepass`] docs.
    pub prepass_enabled: bool,
}

impl Default for PbrPlugin {
    fn default() -> Self {
        Self {
            prepass_enabled: true,
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
            SHADOWS_HANDLE,
            "render/shadows.wgsl",
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
            PBR_AMBIENT_HANDLE,
            "render/pbr_ambient.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, PBR_SHADER_HANDLE, "render/pbr.wgsl", Shader::from_wgsl);
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

        app.register_asset_reflect::<StandardMaterial>()
            .register_type::<AlphaMode>()
            .register_type::<AmbientLight>()
            .register_type::<Cascade>()
            .register_type::<CascadeShadowConfig>()
            .register_type::<Cascades>()
            .register_type::<CascadesVisibleEntities>()
            .register_type::<ClusterConfig>()
            .register_type::<ClusterFarZMode>()
            .register_type::<ClusterZConfig>()
            .register_type::<CubemapVisibleEntities>()
            .register_type::<DirectionalLight>()
            .register_type::<DirectionalLightShadowMap>()
            .register_type::<NotShadowCaster>()
            .register_type::<NotShadowReceiver>()
            .register_type::<PointLight>()
            .register_type::<PointLightShadowMap>()
            .register_type::<SpotLight>()
            .init_resource::<AmbientLight>()
            .init_resource::<GlobalVisiblePointLights>()
            .init_resource::<DirectionalLightShadowMap>()
            .init_resource::<PointLightShadowMap>()
            .add_plugins((
                MeshRenderPlugin,
                MaterialPlugin::<StandardMaterial> {
                    prepass_enabled: self.prepass_enabled,
                    ..Default::default()
                },
                ScreenSpaceAmbientOcclusionPlugin,
                EnvironmentMapPlugin,
                ExtractResourcePlugin::<AmbientLight>::default(),
                FogPlugin,
            ))
            .configure_sets(
                PostUpdate,
                (
                    SimulationLightSystems::AddClusters,
                    SimulationLightSystems::AddClustersFlush,
                    SimulationLightSystems::AssignLightsToClusters,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (
                    add_clusters.in_set(SimulationLightSystems::AddClusters),
                    apply_deferred.in_set(SimulationLightSystems::AddClustersFlush),
                    assign_lights_to_clusters
                        .in_set(SimulationLightSystems::AssignLightsToClusters)
                        .after(TransformSystem::TransformPropagate)
                        .after(VisibilitySystems::CheckVisibility)
                        .after(CameraUpdateSystem),
                    update_directional_light_cascades
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
                    check_light_mesh_visibility
                        .in_set(SimulationLightSystems::CheckLightVisibility)
                        .after(VisibilitySystems::CalculateBoundsFlush)
                        .after(TransformSystem::TransformPropagate)
                        .after(SimulationLightSystems::UpdateLightFrusta)
                        // NOTE: This MUST be scheduled AFTER the core renderer visibility check
                        // because that resets entity `ViewVisibility` for the first view
                        // which would override any results from this otherwise
                        .after(VisibilitySystems::CheckVisibility),
                ),
            );

        app.world.resource_mut::<Assets<StandardMaterial>>().insert(
            Handle::<StandardMaterial>::default(),
            StandardMaterial {
                base_color: Color::rgb(1.0, 0.0, 0.5),
                unlit: true,
                ..Default::default()
            },
        );

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        // Extract the required data from the main world
        render_app
            .add_systems(
                ExtractSchedule,
                (render::extract_clusters, render::extract_lights),
            )
            .add_systems(
                Render,
                (
                    render::prepare_lights
                        .in_set(RenderSet::ManageViews)
                        .after(prepare_assets::<Image>),
                    sort_phase_system::<Shadow>.in_set(RenderSet::PhaseSort),
                    render::prepare_clusters.in_set(RenderSet::PrepareResources),
                ),
            )
            .init_resource::<LightMeta>();

        let shadow_pass_node = ShadowPassNode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        let draw_3d_graph = graph
            .get_sub_graph_mut(bevy_core_pipeline::core_3d::graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(draw_3d_graph::node::SHADOW_PASS, shadow_pass_node);
        draw_3d_graph.add_node_edge(
            draw_3d_graph::node::SHADOW_PASS,
            bevy_core_pipeline::core_3d::graph::node::START_MAIN_PASS,
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        // Extract the required data from the main world
        render_app
            .init_resource::<ShadowSamplers>()
            .init_resource::<GlobalLightMeta>();
    }
}
