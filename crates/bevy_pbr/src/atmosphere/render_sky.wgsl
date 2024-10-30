#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    functions::{sample_transmittance_lut, sample_sky_view_lut}
};
#import bevy_render::view::View;

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.uv, 0.0, 1.0);
}