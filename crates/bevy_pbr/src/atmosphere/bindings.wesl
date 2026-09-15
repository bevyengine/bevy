import bevy_render::view::View;

import package::{
    render::mesh_view_types::Lights,
    atmosphere::types::{Atmosphere, AtmosphereSettings, AtmosphereTransforms}
};

@group(0) @binding(0) public var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) public var<uniform> settings: AtmosphereSettings;
@group(0) @binding(2) public var<uniform> atmosphere_transforms: AtmosphereTransforms;
@group(0) @binding(3) public var<uniform> view: View;
@group(0) @binding(4) public var<uniform> lights: Lights;

@group(0) @binding(5) public var medium_density_lut: texture_2d<f32>;
@group(0) @binding(6) public var medium_scattering_lut: texture_2d<f32>;
@group(0) @binding(7) public var medium_sampler: sampler;

@group(0) @binding(8) public var transmittance_lut: texture_2d<f32>;
@group(0) @binding(9) public var multiscattering_lut: texture_2d<f32>;
@group(0) @binding(10) public var sky_view_lut: texture_2d<f32>;
@group(0) @binding(11) public var aerial_view_lut: texture_3d<f32>;
@group(0) @binding(12) public var atmosphere_lut_sampler: sampler;
