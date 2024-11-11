mod node;
pub mod resources;

use bevy_app::{App, Plugin};
use bevy_asset::load_internal_asset;
use bevy_core_pipeline::core_3d::graph::Node3d;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryItem, With},
    schedule::IntoSystemConfigs,
    system::{lifetimeless::Read, Commands, Query},
};
use bevy_math::{UVec2, UVec3, Vec3};
use bevy_reflect::Reflect;
use bevy_render::{
    camera::Camera,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{Shader, TextureFormat, TextureUsages},
    renderer::RenderAdapter,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_render::{extract_component::UniformComponentPlugin, render_resource::ShaderType};
use bevy_utils::tracing::warn;

use bevy_core_pipeline::core_3d::{graph::Core3d, Camera3d};

use self::{
    node::{AtmosphereLutsNode, AtmosphereNode, RenderSkyNode},
    resources::{
        prepare_atmosphere_bind_groups, prepare_atmosphere_textures, AtmosphereBindGroupLayouts,
        AtmospherePipelines, AtmosphereSamplers,
    },
};

mod shaders {
    use bevy_asset::Handle;
    use bevy_render::render_resource::Shader;

    pub const TYPES: Handle<Shader> = Handle::weak_from_u128(0xB4CA686B10FA592B508580CCC2F9558C);
    pub const FUNCTIONS: Handle<Shader> =
        Handle::weak_from_u128(0xD5524FD88BDC153FBF256B7F2C21906F);
    pub const BRUNETON_FUNCTIONS: Handle<Shader> =
        Handle::weak_from_u128(0x7E896F48B707555DD11985F9C1594459);
    pub const BINDINGS: Handle<Shader> = Handle::weak_from_u128(0x140EFD89B5D4C8490AB895010DFC42FE);

    pub const TRANSMITTANCE_LUT: Handle<Shader> =
        Handle::weak_from_u128(0xEECBDEDFEED7F4EAFBD401BFAA5E0EFB);
    pub const MULTISCATTERING_LUT: Handle<Shader> =
        Handle::weak_from_u128(0x65915B32C44B6287C0CCE1E70AF2936A);
    pub const SKY_VIEW_LUT: Handle<Shader> =
        Handle::weak_from_u128(0x54136D7E6FFCD45BE38399A4E5ED7186);
    pub const AERIAL_VIEW_LUT: Handle<Shader> =
        Handle::weak_from_u128(0x6FDEC284AD356B78C3A4D8ED4CBA0BC5);
    pub const RENDER_SKY: Handle<Shader> =
        Handle::weak_from_u128(0x1951EB87C8A6129F0B541B1E4B3D4962);
}

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, shaders::TYPES, "types.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, shaders::FUNCTIONS, "functions.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            shaders::BRUNETON_FUNCTIONS,
            "bruneton_functions.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(app, shaders::BINDINGS, "bindings.wgsl", Shader::from_wgsl);

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

        load_internal_asset!(
            app,
            shaders::RENDER_SKY,
            "render_sky.wgsl",
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

        if !render_app
            .world()
            .resource::<RenderAdapter>()
            .get_texture_format_features(TextureFormat::Rgba16Float)
            .allowed_usages
            .contains(TextureUsages::STORAGE_BINDING)
        {
            warn!("AtmospherePlugin not loaded. GPU lacks support: TextureFormat::Rgba16Float does not support TextureUsages::STORAGE_BINDING.");
            return;
        }

        render_app
            .init_resource::<AtmosphereBindGroupLayouts>()
            .init_resource::<AtmosphereSamplers>()
            .init_resource::<AtmospherePipelines>()
            .add_systems(
                Render,
                (
                    prepare_atmosphere_textures.in_set(RenderSet::PrepareResources),
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

/// This component describes the atmosphere of a planet
//TODO: padding/alignment?
#[derive(Clone, Component, Reflect, ShaderType)]
#[require(AtmosphereSettings)]
pub struct Atmosphere {
    /// Radius of the planet
    ///
    /// units: km
    bottom_radius: f32,

    /// Radius at which we consider the atmosphere to 'end' for out calculations (from center of planet)
    /// units: km
    top_radius: f32,

    ground_albedo: Vec3, //used for estimating multiscattering

    rayleigh_density_exp_scale: f32,
    rayleigh_scattering: Vec3,

    mie_density_exp_scale: f32,
    mie_scattering: f32, //units: km^-1
    mie_absorption: f32, //units: km^-1
    mie_asymmetry: f32,  //the "asymmetry" value of the phase function, unitless. Domain: (-1, 1)

    ozone_layer_center_altitude: f32, //units: km
    ozone_layer_half_width: f32,      //units: km
    ozone_absorption: Vec3,           //ozone absorption. units: km^-1
}

impl Atmosphere {
    //TODO: check all these values before merge
    //TODO: UNITS
    pub const EARTH: Atmosphere = Atmosphere {
        bottom_radius: 6360.0,
        top_radius: 6460.0,
        ground_albedo: Vec3::splat(0.3),
        rayleigh_density_exp_scale: -1.0 / 8.0,
        rayleigh_scattering: Vec3::new(0.005802, 0.013558, 0.033100),
        mie_density_exp_scale: -1.0 / 1.2,
        mie_scattering: 0.003996,
        mie_absorption: 0.004440,
        mie_asymmetry: 0.8,
        ozone_layer_center_altitude: 25.0,
        ozone_layer_half_width: 15.0,
        ozone_absorption: Vec3::new(0.000650, 0.001881, 0.000085),
    };
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self::EARTH
    }
}

impl ExtractComponent for Atmosphere {
    type QueryData = Read<Atmosphere>;

    type QueryFilter = With<Camera3d>;

    type Out = Atmosphere;

    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

#[derive(Clone, Component, Reflect, ShaderType)]
pub struct AtmosphereSettings {
    pub transmittance_lut_size: UVec2,
    pub multiscattering_lut_size: UVec2,
    pub sky_view_lut_size: UVec2,
    pub multiscattering_lut_dirs: u32,
    pub transmittance_lut_samples: u32,
    pub aerial_view_lut_size: UVec3,
    pub multiscattering_lut_samples: u32,
    pub sky_view_lut_samples: u32,
    pub aerial_view_lut_samples: u32,
    pub scene_units_to_km: f32,
}

impl Default for AtmosphereSettings {
    fn default() -> Self {
        Self {
            transmittance_lut_size: UVec2::new(256, 128),
            transmittance_lut_samples: 40,
            multiscattering_lut_size: UVec2::new(32, 32),
            multiscattering_lut_dirs: 64,
            multiscattering_lut_samples: 20,
            sky_view_lut_size: UVec2::new(200, 100),
            sky_view_lut_samples: 30,
            aerial_view_lut_size: UVec3::new(32, 32, 32),
            aerial_view_lut_samples: 10,
            scene_units_to_km: 1.0e-3,
        }
    }
}

impl ExtractComponent for AtmosphereSettings {
    type QueryData = Read<AtmosphereSettings>;

    type QueryFilter = (With<Camera3d>, With<Atmosphere>);

    type Out = AtmosphereSettings;

    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}
