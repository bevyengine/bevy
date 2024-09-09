mod node;
mod resources;

use bevy_app::{App, Plugin};
use bevy_asset::load_internal_asset;
use bevy_core_pipeline::core_3d::graph::Node3d;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{With, Without},
    schedule::IntoSystemConfigs,
    system::{Commands, Query},
};
use bevy_math::{Vec2, Vec3};
use bevy_reflect::Reflect;
use bevy_render::{
    camera::Camera,
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{Extent3d, Shader, TextureFormat, TextureUsages},
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

        app.register_type::<Atmosphere>()
            .add_plugins(UniformComponentPlugin::<Atmosphere>::default());
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
            .add_systems(ExtractSchedule, extract_atmosphere)
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
#[derive(Clone, Component, Reflect, ShaderType)]
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

impl Default for Atmosphere {
    fn default() -> Self {
        Self::EARTH
    }
}

impl Atmosphere {
    //TODO: check all these values before merge
    pub const EARTH: Atmosphere = Atmosphere {
        bottom_radius: 6360.0,
        top_radius: 6460.0,
        rayleigh_density_exp_scale: -1.0 / 8.0,
        rayleigh_scattering: Vec3::new(0.005802, 0.013558, 0.033100),
        mie_density_exp_scale: -1.0 / 1.2,
        mie_scattering: 0.03996,
        mie_absorption: 0.000444,
        mie_phase_function_g: 0.8,
        ozone_layer_center_altitude: 25.0,
        ozone_layer_half_width: 15.0,
        ozone_absorption: Vec3::new(0.000650, 0.001881, 0.000085),
    };
}

fn extract_atmosphere(
    mut commands: Commands,
    cameras: Extract<
        Query<(Entity, &Camera, &Atmosphere, Option<&AtmosphereLutSettings>), With<Camera3d>>,
    >,
) {
    for (entity, camera, atmosphere, lut_settings) in &cameras {
        if camera.is_active {
            commands.get_or_spawn(entity).insert((
                atmosphere.clone(),
                lut_settings
                    .cloned()
                    .unwrap_or_else(|| AtmosphereLutSettings::from_camera(camera)),
            ));
        }
    }
}

#[derive(Clone, Component)]
pub struct AtmosphereLutSettings {
    pub transmittance_lut_size: Extent3d,
    pub multiscattering_lut_size: Extent3d,
    pub sky_view_lut_size: Extent3d,
    pub aerial_view_lut_size: Extent3d,
}

impl Default for AtmosphereLutSettings {
    fn default() -> Self {
        Self {
            transmittance_lut_size: Extent3d {
                width: 256,
                height: 128,
                depth_or_array_layers: 1,
            },
            multiscattering_lut_size: Extent3d {
                width: 32,
                height: 32,
                depth_or_array_layers: 1,
            },
            sky_view_lut_size: Extent3d {
                width: 192,
                height: 108,
                depth_or_array_layers: 1,
            },
            aerial_view_lut_size: Extent3d {
                width: 32,
                height: 32,
                depth_or_array_layers: 32,
            },
        }
    }
}

impl AtmosphereLutSettings {
    pub fn from_camera(camera: &Camera) -> Self {
        //TODO: correct method?
        if let Some(viewport_size) = camera.logical_viewport_size() {
            Self {
                sky_view_lut_size: Extent3d {
                    width: viewport_size.x as u32 / 10,
                    height: viewport_size.y as u32 / 10,
                    depth_or_array_layers: 1,
                },
                ..Self::default()
            }
        } else {
            Self::default()
        }
    }
}
