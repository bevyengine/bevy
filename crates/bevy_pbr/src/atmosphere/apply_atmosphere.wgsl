#import bevy_pbr::{
    mesh_view_types::View,
    atmosphere::types::{Atmosphere, AtmosphereSettings},
}

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var depth_sampler: sampler;
@group(0) @binding(3) var sky_view_lut: texture_2d<f32>;
@group(0) @binding(4) var sky_view_lut_sampler: sampler;
@group(0) @binding(5) var aerial_view_lut: texture_3d<f32>;
@group(0) @binding(6) var aerial_view_lut_sampler: sampler;


@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec3<f32> {
}
