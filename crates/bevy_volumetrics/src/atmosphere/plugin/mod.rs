//! Sets up the rendering of atmospheric scattering

mod systems;

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, Handle};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_render::{
    extract_component::{ExtractComponentPlugin, UniformComponentPlugin},
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{
        DownlevelFlags, Shader, SpecializedRenderPipelines, TextureFormat, TextureUsages,
    },
    renderer::{RenderAdapter, RenderDevice},
    settings::WgpuFeatures,
    Render, RenderApp, RenderSet,
};

use systems::{
    configure_camera_depth_usages, prepare_atmosphere_bind_groups, prepare_atmosphere_textures,
    prepare_atmosphere_transforms, queue_render_sky_pipelines,
};
use tracing::warn;

use super::{
    node::{AtmosphereLutsNode, AtmosphereNode, RenderSkyNode},
    render::{
        AtmosphereBindGroupLayouts, AtmosphereLutPipelines, AtmosphereSamplers,
        AtmosphereTransforms, RenderSkyBindGroupLayouts,
    },
    Atmosphere, AtmosphereSettings,
};

pub(crate) const TYPES: Handle<Shader> = weak_handle!("ef7e147e-30a0-4513-bae3-ddde2a6c20c5");
pub(crate) const FUNCTIONS: Handle<Shader> = weak_handle!("7ff93872-2ee9-4598-9f88-68b02fef605f");
pub(crate) const BRUNETON_FUNCTIONS: Handle<Shader> =
    weak_handle!("e2dccbb0-7322-444a-983b-e74d0a08bcda");
pub(crate) const BINDINGS: Handle<Shader> = weak_handle!("bcc55ce5-0fc4-451e-8393-1b9efd2612c4");

pub(crate) const TRANSMITTANCE_LUT: Handle<Shader> =
    weak_handle!("a4187282-8cb1-42d3-889c-cbbfb6044183");
pub(crate) const MULTISCATTERING_LUT: Handle<Shader> =
    weak_handle!("bde3a71a-73e9-49fe-a379-a81940c67a1e");
pub(crate) const SKY_VIEW_LUT: Handle<Shader> =
    weak_handle!("f87e007a-bf4b-4f99-9ef0-ac21d369f0e5");
pub(crate) const AERIAL_VIEW_LUT: Handle<Shader> =
    weak_handle!("a3daf030-4b64-49ae-a6a7-354489597cbe");
pub(crate) const RENDER_SKY: Handle<Shader> = weak_handle!("09422f46-d0f7-41c1-be24-121c17d6e834");

#[derive(Default)]
pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, TYPES, "shaders/types.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, FUNCTIONS, "shaders/functions.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            BRUNETON_FUNCTIONS,
            "shaders/bruneton_functions.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(app, BINDINGS, "shaders/bindings.wgsl", Shader::from_wgsl);

        load_internal_asset!(
            app,
            TRANSMITTANCE_LUT,
            "shaders/transmittance_lut.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            MULTISCATTERING_LUT,
            "shaders/multiscattering_lut.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            SKY_VIEW_LUT,
            "shaders/sky_view_lut.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            AERIAL_VIEW_LUT,
            "shaders/aerial_view_lut.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            RENDER_SKY,
            "shaders/render_sky.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<Atmosphere>()
            .register_type::<AtmosphereSettings>()
            .add_plugins((
                ExtractComponentPlugin::<Atmosphere>::default(),
                ExtractComponentPlugin::<AtmosphereSettings>::default(),
                UniformComponentPlugin::<Atmosphere>::default(),
                UniformComponentPlugin::<AtmosphereSettings>::default(),
            ));
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_adapter = render_app.world().resource::<RenderAdapter>();
        let render_device = render_app.world().resource::<RenderDevice>();

        if !render_device
            .features()
            .contains(WgpuFeatures::DUAL_SOURCE_BLENDING)
        {
            warn!("AtmospherePlugin not loaded. GPU lacks support for dual-source blending.");
            return;
        }

        if !render_adapter
            .get_downlevel_capabilities()
            .flags
            .contains(DownlevelFlags::COMPUTE_SHADERS)
        {
            warn!("AtmospherePlugin not loaded. GPU lacks support for compute shaders.");
            return;
        }

        if !render_adapter
            .get_texture_format_features(TextureFormat::Rgba16Float)
            .allowed_usages
            .contains(TextureUsages::STORAGE_BINDING)
        {
            warn!("AtmospherePlugin not loaded. GPU lacks support: TextureFormat::Rgba16Float does not support TextureUsages::STORAGE_BINDING.");
            return;
        }

        render_app
            .init_resource::<AtmosphereBindGroupLayouts>()
            .init_resource::<RenderSkyBindGroupLayouts>()
            .init_resource::<AtmosphereSamplers>()
            .init_resource::<AtmosphereLutPipelines>()
            .init_resource::<AtmosphereTransforms>()
            .init_resource::<SpecializedRenderPipelines<RenderSkyBindGroupLayouts>>()
            .add_systems(
                Render,
                (
                    configure_camera_depth_usages.in_set(RenderSet::ManageViews),
                    queue_render_sky_pipelines.in_set(RenderSet::Queue),
                    prepare_atmosphere_textures.in_set(RenderSet::PrepareResources),
                    prepare_atmosphere_transforms.in_set(RenderSet::PrepareResources),
                    prepare_atmosphere_bind_groups.in_set(RenderSet::PrepareBindGroups),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<AtmosphereLutsNode>>(
                Core3d,
                AtmosphereNode::RenderLuts,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    // END_PRE_PASSES -> RENDER_LUTS -> MAIN_PASS
                    Node3d::EndPrepasses,
                    AtmosphereNode::RenderLuts,
                    Node3d::StartMainPass,
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<RenderSkyNode>>(
                Core3d,
                AtmosphereNode::RenderSky,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::MainOpaquePass,
                    AtmosphereNode::RenderSky,
                    Node3d::MainTransparentPass,
                ),
            );
    }
}
