#import bevy_pbr::atmosphere::types::{Atmosphere, AtmosphereSettings};
#import bevy_render::view::View;

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var sky_view_lut: texture_2d<f32>;
@group(0) @binding(2) var sky_view_lut_sampler: sampler;

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(sky_view_lut, sky_view_lut_sampler, in.uv);
}
