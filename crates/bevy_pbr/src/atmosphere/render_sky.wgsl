#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{atmosphere, view, atmosphere_transforms},
    functions::{sample_transmittance_lut, sample_sky_view_lut, direction_atmosphere_to_world, uv_to_ndc, sample_aerial_view_lut, view_radius, sample_sun_illuminance},
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
        let ray_dir_as = uv_to_ray_direction_as(in.uv);
        let ray_dir_ws = direction_atmosphere_to_world(ray_dir_as);

        let r = view_radius();
        let mu = ray_dir_ws.y;
        let transmittance = sample_transmittance_lut(r, mu);
        let inscattering = sample_sky_view_lut(correct_sampling_dir(r, ray_dir_as));

        let sun_illuminance = sample_sun_illuminance(ray_dir_ws, transmittance);
        return vec4(inscattering + sun_illuminance, (transmittance.r + transmittance.g + transmittance.b) / 3.0);
    } else {
        let ndc_xy = uv_to_ndc(in.uv);
        let ndc = vec3(ndc_xy, depth);
        let inscattering_and_transmittance = sample_aerial_view_lut(ndc);
        return inscattering_and_transmittance;
    }
}

//approximates sampling direction from angle to horizon at the current radius
fn correct_sampling_dir(r: f32, ray_dir_as: vec3<f32>) -> vec3<f32> {
    let altitude_ratio = atmosphere.bottom_radius / r;
    let neg_mu_horizon = sqrt(1 - altitude_ratio * altitude_ratio);
    return normalize(ray_dir_as + vec3(0.0, neg_mu_horizon, 0.0));
}

//Modified from skybox.wgsl. For this pass we don't need to apply a separate sky transform or consider camera viewport.
//w component is the cosine of the view direction with the view forward vector, to correct step distance at the edges of the viewport
fn uv_to_ray_direction_as(uv: vec2<f32>) -> vec3<f32> {
    // Using world positions of the fragment and camera to calculate a ray direction
    // breaks down at large translations. This code only needs to know the ray direction.
    // The ray direction is along the direction from the camera to the fragment position.
    // In view space, the camera is at the origin, so the view space ray direction is
    // along the direction of the fragment position - (0,0,0) which is just the
    // fragment position.
    // Use the position on the near clipping plane to avoid -inf world position
    // because the far plane of an infinite reverse projection is at infinity.
    let atmosphere_position_homogeneous = atmosphere_transforms.atmosphere_from_clip * vec4(
        uv_to_ndc(uv),
        1.0,
        1.0,
    );

    return atmosphere_position_homogeneous.xyz / atmosphere_position_homogeneous.w;
}
