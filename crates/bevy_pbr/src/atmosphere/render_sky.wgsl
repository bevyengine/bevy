#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{atmosphere, view},
    functions::{sample_transmittance_lut, sample_sky_view_lut, uv_to_ray_direction, uv_to_ndc, sample_aerial_view_lut, view_radius, sample_sun_disk},
};
#import bevy_render::view::View;

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(12) var depth_texture: texture_depth_2d;

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let depth = textureLoad(depth_texture, vec2<i32>(in.position.xy), 0);
    if depth == 0.0 {
        let ray_dir = uv_to_ray_direction(in.uv).xyz;
        let r = view_radius();
        let mu = ray_dir.y;
        let sky_view = sample_sky_view_lut(ray_dir);
        let transmittance = sample_transmittance_lut(r, mu);
        let sun_disk = sample_sun_disk(ray_dir, transmittance);
        return vec4(sky_view + sun_disk, 1.0);
    } else {
        let ndc_xy = uv_to_ndc(in.uv);
        let ndc = vec3(ndc_xy, depth);
        let inscattering = sample_aerial_view_lut(ndc);
        return inscattering;
    }
}
