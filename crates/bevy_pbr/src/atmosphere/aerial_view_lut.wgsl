#import bevy_pbr::atmosphere::types::{Atmosphere, AtmosphereSettings};

@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) var<uniform> lut_settings: AtmosphereSettings;
@group(0) @binding(2) var<uniform> lights: Lights;
@group(0) @binding(3) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(4) var tranmittance_lut_sampler: sampler;
@group(0) @binding(5) var multiscattering_lut: texture_2d<f32>;
@group(0) @binding(6) var tranmittance_lut_sampler: sampler;
@group(0) @binding(7) var aerial_view_lut: texture_storage_3d<rgba16float, write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) idx: vec3<u32>) {
}

