#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    functions::{sample_transmittance_lut, sample_sky_view_lut, uv_to_ray_direction},
};
#import bevy_render::view::View;

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let ray_dir = uv_to_ray_direction(in.uv).xyz;
    return vec4(sample_sky_view_lut(ray_dir), 0.0);
}
