#define_import_path bevy_pbr::atmosphere::bindings

#import bevy_render::view::View;

#import bevy_pbr::{
    mesh_view_types::Lights,
    atmosphere::types::{Atmosphere, AtmosphereSettings, AtmosphereTransforms}
}

@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) var<uniform> settings: AtmosphereSettings;
@group(0) @binding(2) var<uniform> atmosphere_transforms: AtmosphereTransforms;
@group(0) @binding(3) var<uniform> view: View;
@group(0) @binding(4) var<uniform> lights: Lights;
@group(0) @binding(5) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(6) var transmittance_lut_sampler: sampler;
@group(0) @binding(7) var multiscattering_lut: texture_2d<f32>;
@group(0) @binding(8) var multiscattering_lut_sampler: sampler;
@group(0) @binding(9) var sky_view_lut: texture_2d<f32>;
@group(0) @binding(10) var sky_view_lut_sampler: sampler;
@group(0) @binding(11) var aerial_view_lut: texture_3d<f32>;
@group(0) @binding(12) var aerial_view_lut_sampler: sampler;
