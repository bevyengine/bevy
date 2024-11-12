#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{atmosphere, view},
    functions::{sample_transmittance_lut, sample_sky_view_lut, direction_view_to_world, uv_to_ndc, sample_aerial_view_lut, view_radius, sample_sun_disk},
};
#import bevy_render::view::View;

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(12) var depth_texture: texture_depth_2d;

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let depth = textureLoad(depth_texture, vec2<i32>(in.position.xy), 0);
    if depth == 0.0 {
        let view_ray_dir = uv_to_ray_direction(in.uv).xyz;
        let world_ray_dir = direction_view_to_world(view_ray_dir);
        let r = view_radius();
        let mu = world_ray_dir.y;
        let sky_view = sample_sky_view_lut(view_ray_dir);
        let transmittance = sample_transmittance_lut(r, mu);
        let sun_disk = sample_sun_disk(world_ray_dir, transmittance);
        return vec4(sky_view + sun_disk, (transmittance.r + transmittance.g + transmittance.b) / 3.0);
    } else {
        let ndc_xy = uv_to_ndc(in.uv);
        let ndc = vec3(ndc_xy, depth);
        let inscattering = sample_aerial_view_lut(ndc);
        return inscattering;
    }
}

//Modified from skybox.wgsl. For this pass we don't need to apply a separate sky transform or consider camera viewport.
//w component is the cosine of the view direction with the view forward vector, to correct step distance at the edges of the viewport
fn uv_to_ray_direction(uv: vec2<f32>) -> vec4<f32> {
    // Using world positions of the fragment and camera to calculate a ray direction
    // breaks down at large translations. This code only needs to know the ray direction.
    // The ray direction is along the direction from the camera to the fragment position.
    // In view space, the camera is at the origin, so the view space ray direction is
    // along the direction of the fragment position - (0,0,0) which is just the
    // fragment position.
    // Use the position on the near clipping plane to avoid -inf world position
    // because the far plane of an infinite reverse projection is at infinity.
    let view_position_homogeneous = view.view_from_clip * vec4(
        uv_to_ndc(uv),
        1.0,
        1.0,
    );

    // Transforming the view space ray direction by the skybox transform matrix, it is 
    // equivalent to rotating the skybox itself.
    let view_ray_direction = view_position_homogeneous.xyz / view_position_homogeneous.w; //TODO: remove this step and just use position_ndc_to_world? we didn't need to transform in view space

    return vec4(normalize(view_ray_direction), -view_ray_direction.z);
}
