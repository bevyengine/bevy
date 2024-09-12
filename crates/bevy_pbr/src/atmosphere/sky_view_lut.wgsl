#import bevy_pbr::atmosphere::types::{Atmosphere, AtmosphereSettings};
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) var<uniform> settings: AtmosphereSettings;
@group(0) @binding(2) var<uniform> lights: Lights;
@group(0) @binding(3) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(4) var tranmittance_lut_sampler: sampler;
@group(0) @binding(5) var multiscattering_lut: texture_2d<f32>;
@group(0) @binding(6) var multiscattering_lut_sampler: sampler;

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
}
