mod node;
mod resources;

use bevy_app::{App, Plugin};
use bevy_asset::load_internal_asset;
use bevy_core_pipeline::core_3d::graph::Node3d;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    schedule::IntoSystemConfigs,
    system::{Commands, Query},
};
use bevy_math::Vec3;
use bevy_reflect::Reflect;
use bevy_render::{
    camera::Camera,
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{Shader, TextureFormat, TextureUsages},
    renderer::RenderAdapter,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_render::{extract_component::UniformComponentPlugin, render_resource::ShaderType};
use bevy_utils::tracing::warn;

use bevy_core_pipeline::core_3d::{graph::Core3d, Camera3d};

use self::{
    node::{SkyLabel, SkyNode},
    resources::{
        prepare_atmosphere_bind_groups, prepare_atmosphere_textures, AtmosphereBindGroupLayouts,
        AtmospherePipelines,
    },
};

mod shaders {
    use bevy_asset::Handle;
    use bevy_render::render_resource::Shader;

    pub const TYPES: Handle<Shader> = Handle::weak_from_u128(0xB4CA686B10FA592B508580CCC2F9558C);
    pub const COMMON: Handle<Shader> = Handle::weak_from_u128(0xD5524FD88BDC153FBF256B7F2C21906F);

    pub const TRANSMITTANCE_LUT: Handle<Shader> =
        Handle::weak_from_u128(0xEECBDEDFEED7F4EAFBD401BFAA5E0EFB);
    pub const MULTISCATTERING_LUT: Handle<Shader> =
        Handle::weak_from_u128(0x65915B32C44B6287C0CCE1E70AF2936A);
    pub const SKY_VIEW_LUT: Handle<Shader> =
        Handle::weak_from_u128(0x54136D7E6FFCD45BE38399A4E5ED7186);
    pub const AERIAL_VIEW_LUT: Handle<Shader> =
        Handle::weak_from_u128(0x6FDEC284AD356B78C3A4D8ED4CBA0BC5);
}

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, shaders::TYPES, "types.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, shaders::COMMON, "common.wgsl", Shader::from_wgsl);

        load_internal_asset!(
            app,
            shaders::TRANSMITTANCE_LUT,
            "transmittance_lut.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            shaders::MULTISCATTERING_LUT,
            "multiscattering_lut.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            shaders::SKY_VIEW_LUT,
            "sky_view_lut.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            shaders::AERIAL_VIEW_LUT,
            "aerial_view_lut.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<Atmosphere>();
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        if !render_app
            .world()
            .resource::<RenderAdapter>()
            .get_texture_format_features(TextureFormat::Rgba16Float)
            .allowed_usages
            .contains(TextureUsages::STORAGE_BINDING)
        {
            warn!("SkyPlugin not loaded. GPU lacks support: TextureFormat::Rgba16Float does not support TextureUsages::STORAGE_BINDING.");
            return;
        }

        render_app
            .init_resource::<AtmosphereBindGroupLayouts>()
            .init_resource::<AtmospherePipelines>()
            .add_systems(ExtractSchedule, extract_sky_settings)
            .add_plugins(UniformComponentPlugin::<Atmosphere>::default())
            .add_systems(
                Render,
                (
                    prepare_atmosphere_textures.in_set(RenderSet::PrepareResources),
                    prepare_atmosphere_bind_groups.in_set(RenderSet::PrepareBindGroups),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<SkyNode>>(Core3d, SkyLabel)
            .add_render_graph_edges(
                Core3d,
                (
                    // END_PRE_PASSES -> PREPARE_SKY -> MAIN_PASS
                    Node3d::EndPrepasses,
                    SkyLabel,
                    Node3d::StartMainPass,
                ),
            );
    }
}

//TODO: padding/alignment?
#[derive(Clone, Component, Default, Reflect, ShaderType)]
pub struct Atmosphere {
    /// Radius of the planet
    ///
    /// units: km
    bottom_radius: f32,

    // Radius at which we consider the atmosphere to 'end' for out calculations (from center of planet)
    top_radius: f32,

    rayleigh_density_exp_scale: f32,
    rayleigh_scattering: Vec3,

    mie_density_exp_scale: f32,
    mie_scattering: f32,       //units: km^-1
    mie_absorption: f32,       //units: km^-1
    mie_phase_function_g: f32, //the "asymmetry" value of the phase function, unitless. Domain: (-1, 1)

    ozone_layer_center_altitude: f32, //units: km
    ozone_layer_half_width: f32,      //units: km
    ozone_absorption: Vec3,           //ozone absorption. units: km^-1
}

fn extract_sky_settings(
    mut commands: Commands,
    cameras: Extract<Query<(Entity, &Camera, &Atmosphere), With<Camera3d>>>,
) {
    for (entity, camera, sky_settings) in &cameras {
        if camera.is_active {
            commands.get_or_spawn(entity).insert(sky_settings.clone());
        }
    }
}
