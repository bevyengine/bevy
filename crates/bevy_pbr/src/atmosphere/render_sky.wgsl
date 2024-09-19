#import bevy_pbr::{
    mesh_view_types::View,
    atmosphere::types::{Atmosphere, AtmosphereSettings},
}

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(3) var sky_view_lut: texture_2d<f32>;
@group(0) @binding(4) var sky_view_lut_sampler: sampler;

//from core_pipeline skybox.wgsl
fn coords_to_ray_direction(position: vec2<f32>, viewport: vec4<f32>) -> vec3<f32> {
    // Using world positions of the fragment and camera to calculate a ray direction
    // breaks down at large translations. This code only needs to know the ray direction.
    // The ray direction is along the direction from the camera to the fragment position.
    // In view space, the camera is at the origin, so the view space ray direction is
    // along the direction of the fragment position - (0,0,0) which is just the
    // fragment position.
    // Use the position on the near clipping plane to avoid -inf world position
    // because the far plane of an infinite reverse projection is at infinity.
    let view_position_homogeneous = view.view_from_clip * vec4(
        coords_to_viewport_uv(position, viewport) * vec2(2.0, -2.0) + vec2(-1.0, 1.0),
        1.0,
        1.0,
    );

    // Transforming the view space ray direction by the skybox transform matrix, it is 
    // equivalent to rotating the skybox itself.
    var view_ray_direction = view_position_homogeneous.xyz / view_position_homogeneous.w;
    view_ray_direction = (uniforms.transform * vec4(view_ray_direction, 1.0)).xyz;

    // Transforming the view space ray direction by the view matrix, transforms the
    // direction to world space. Note that the w element is set to 0.0, as this is a
    // vector direction, not a position, That causes the matrix multiplication to ignore
    // the translations from the view matrix.
    let ray_direction = (view.world_from_view * vec4(view_ray_direction, 0.0)).xyz;

    return normalize(ray_direction);
}

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec3<f32> {
    let ray_direction = coords_to_ray_direction(in.uv, view.viewport);


    textureSample(sky_view_lut, sky_view_lut_sampler, in.uv)
}
