#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{atmosphere, view, atmosphere_transforms},
    functions::{
        sample_transmittance_lut, sample_sky_view_lut, 
        direction_world_to_atmosphere, uv_to_ray_direction,
        uv_to_ndc, sample_aerial_view_lut, view_radius,
        sample_sun_illuminance, get_horizon_zenith,
    },
};
#import bevy_render::view::View;

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

#ifdef MULTISAMPLED
@group(0) @binding(13) var depth_texture: texture_depth_multisampled_2d;
#else
@group(0) @binding(13) var depth_texture: texture_depth_2d;
#endif

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let depth = textureLoad(depth_texture, vec2<i32>(in.position.xy), 0);
    if depth == 0.0 {
        let ray_dir_ws = uv_to_ray_direction(in.uv);
        let ray_dir_as = direction_world_to_atmosphere(ray_dir_ws.xyz);

        let r = view_radius();
        let mu = ray_dir_ws.y;
        let horizon_zenith = get_horizon_zenith(r);

        let transmittance = sample_transmittance_lut(r, mu);
        let inscattering = sample_sky_view_lut(ray_dir_as, horizon_zenith);

        let sun_illuminance = sample_sun_illuminance(ray_dir_ws.xyz, transmittance);
        return vec4(inscattering + sun_illuminance, (transmittance.r + transmittance.g + transmittance.b) / 3.0);
    } else {
        let ndc_xy = uv_to_ndc(in.uv);
        let ndc = vec3(ndc_xy, depth);
        let inscattering_and_transmittance = sample_aerial_view_lut(ndc);
        return inscattering_and_transmittance;
    }
}
