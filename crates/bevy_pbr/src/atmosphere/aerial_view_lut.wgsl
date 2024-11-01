#import bevy_pbr::{
    mesh_view_types::{Lights, DirectionalLight},
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        bindings::{atmosphere, settings, view, lights, aerial_view_lut_out},
        functions::{
            sample_transmittance_lut, sample_atmosphere, rayleigh, henyey_greenstein,
            sample_multiscattering_lut, AtmosphereSample, sample_local_inscattering,
            get_local_r, get_local_up
        },
        bruneton_functions::{distance_to_top_atmosphere_boundary, distance_to_bottom_atmosphere_boundary,ray_intersects_ground}
    }
}


@group(0) @binding(13) var aerial_view_lut_out: texture_storage_3d<rgba16float, write>;

@compute
@workgroup_size(16, 16, 1) //TODO: this approach makes it so closer slices get fewer samples. But we also expect those to have less scattering. So win/win?
fn main(@builtin(global_invocation_id) idx: vec3<u32>) {
    if any(idx.xy > settings.aerial_view_lut_size.xy) { return; }
    var optical_depth: vec3<f32> = vec3(0.0);

    let uv = (vec2<f32>(idx.xy) + 0.5) / vec2<f32>(settings.aerial_view_lut_size.xy);
    let view_dir = uv_to_ray_direction(uv); //TODO: negate for lighting calcs?
    let r = view.world_position.y / 1000.0 + atmosphere.bottom_radius;
    let mu = view_dir.y;

    var prev_view_z = 0.0;
    var total_inscattering = vec3(0.0);
    for (var slice_i: i32 = i32(settings.aerial_view_lut_size.z - 1); slice_i >= 0; slice_i--) { //reversed loop to iterate depth near->far 
        var sum_transmittance = 0.0;
        for (var step_i: i32 = i32(settings.aerial_view_lut_samples - 1); step_i >= 0; step_i--) { //same here
            let ndc_z = (f32(slice_i) + ((f32(step_i) + 0.5) / f32(settings.aerial_view_lut_samples))) / f32(settings.aerial_view_lut_size.z);
            //view_dir.w is the cosine of the angle between the view vector and the camera forward vector, used to correct the step length.            
            let view_z = depth_ndc_to_view_z(ndc_z) / view_dir.w / 1000.0;

            //subtraction is flipped because z values in front of the camera are negative
            let step_length = (prev_view_z - view_z) ;
            prev_view_z = view_z;

            let local_r = get_local_r(r, mu, view_z);
            if local_r > atmosphere.top_radius { break; }
            let local_up = get_local_up(r, view_z * view_dir.xyz);

            let local_atmosphere = sample_atmosphere(local_r);
            optical_depth += local_atmosphere.extinction * step_length; //TODO: units between step_length and atmosphere

            let transmittance_to_sample = exp(-optical_depth);

            var local_inscattering = sample_local_inscattering(local_atmosphere, transmittance_to_sample, view_dir.xyz, local_r, local_up);
            total_inscattering += local_inscattering * step_length;
            sum_transmittance += transmittance_to_sample.r + transmittance_to_sample.g + transmittance_to_sample.b;
        }
        let mean_transmittance = sum_transmittance / (f32(settings.aerial_view_lut_samples) * 3.0);
        textureStore(aerial_view_lut_out, vec3(vec2<i32>(idx.xy), slice_i), vec4(total_inscattering, mean_transmittance));
    }
}

/// Convert uv [0.0 .. 1.0] coordinate to ndc space xy [-1.0 .. 1.0]
fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
}

/// Convert a ndc space position to world space
fn position_ndc_to_world(ndc_pos: vec3<f32>) -> vec3<f32> {
    let world_pos = view.world_from_clip * vec4(ndc_pos, 1.0);
    return world_pos.xyz / world_pos.w;
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

    let view_ray_direction = view_position_homogeneous.xyz / view_position_homogeneous.w; //TODO: remove this step and just use position_ndc_to_world? we didn't need to transform in view space

    // Transforming the view space ray direction by the inverse view matrix, transforms the
    // direction to world space. Note that the w element is set to 0.0, as this is a
    // vector direction, not a position, That causes the matrix multiplication to ignore
    // the translations from the view matrix.
    let ray_direction = (view.world_from_view * vec4(view_ray_direction, 0.0)).xyz;

    return vec4(normalize(ray_direction), -view_ray_direction.z); //TODO: correct sign?
}


/// Convert ndc depth to linear view z. 
/// Note: Depth values in front of the camera will be negative as -z is forward
fn depth_ndc_to_view_z(ndc_depth: f32) -> f32 {
    let view_pos = view.view_from_clip * vec4(0.0, 0.0, ndc_depth, 1.0);
    return view_pos.z / view_pos.w;
}
