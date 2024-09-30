#import bevy_pbr::{
    mesh_view_types::{Lights, DirectionalLight},
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        functions::{
            sample_transmittance_lut, sample_atmosphere, rayleigh, henyey_greenstein,
            sample_multiscattering_lut, distance_to_top_atmosphere_boundary, distance_to_bottom_atmosphere_boundary,
            ray_intersects_ground
        },
    }
}

#import bevy_render::view::View;

@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) var<uniform> settings: AtmosphereSettings;
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(3) var<uniform> lights: Lights;

@group(0) @binding(4) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(5) var transmittance_lut_sampler: sampler;

@group(0) @binding(6) var multiscattering_lut: texture_2d<f32>;
@group(0) @binding(7) var multiscattering_lut_sampler: sampler;

@group(0) @binding(8) var aerial_view_lut: texture_storage_3d<rgba16float, write>;

@compute
@workgroup_size(16, 16, 1) //TODO: this approach makes it so closer slices get fewer samples. But we also expect those to have less scattering. So win/win?
fn main(@builtin(global_invocation_id) idx: vec3<u32>) {
    if any(idx.xy > settings.aerial_view_lut_size.xy) { return; }
    var optical_depth: vec3<f32> = vec3(0.0);

    let uv = (vec2<f32>(idx.xy) + 0.5) / vec2<f32>(settings.aerial_view_lut_size.xy);
    let ndc_xy = uv_to_ndc(uv);
    let view_dir = uv_to_ray_direction(uv);

    var inscattered_illuminance = vec3(0.0);
    var prev_depth = 0.0;
    for (var slice_i: i32 = settings.aerial_view_lut_size.z - 1; slice_i >= 0; slice_i--) { //reversed loop to iterate depth near->far 
        for (var step_i: i32 = settings.aerial_view_lut_samples - 1; step_i >= 0; step_i--) { //same here
            let ndc_z = (f32(slice_i) + ((f32(step_i) + 0.5) / f32(settings.aerial_view_lut_samples))) / f32(settings.aerial_view_lut_size.z);
            let ndc_pos = vec3(ndc_xy, ndc_z);
            let world_pos = position_ndc_to_world(ndc_pos);

            let depth = depth_ndc_to_view_z(ndc_z); //TODO: incorrect bc edges of view will have longer step length

            //subtraction is flipped because z values in front of the camera are negative
            //see uv_to_ray_direction regarding view_dir.w
            let step_length = (prev_depth - depth) / view_dir.w;
            prev_depth = depth;

            let altitude = world_pos.y;
            let local_atmosphere = sample_atmosphere(atmosphere, altitude);
            optical_depth += local_atmosphere.extinction * step_length; //TODO: units between step_length and atmosphere

            let transmittance_to_sample = exp(-optical_depth);

            var local_illuminance = sample_local_illuminance(view_dir.xyz, altitude);
            inscattered_illuminance += local_atmosphere.scattering * local_illuminance * step_length;
            let mean_transmittance = (transmittance_to_sample.r + transmittance_to_sample.g + transmittance_to_sample.b) / 3.0;

            textureStore(aerial_view_lut, vec3(idx.xy, slice_i), vec4(inscattered_illuminance, mean_transmittance));
            //textureStore(aerial_view_lut, vec3(idx.xy, slice_i), vec4(vec3<f32>(vec3(idx.xy)) / vec3<f32>(settings.aerial_view_lut_size), 1.0));
        }
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

    // Transforming the view space ray direction by the skybox transform matrix, it is 
    // equivalent to rotating the skybox itself.
    let view_ray_direction = view_position_homogeneous.xyz / view_position_homogeneous.w; //TODO: remove this step and just use position_ndc_to_world? we didn't need to transform in view space

    // Transforming the view space ray direction by the inverse view matrix, transforms the
    // direction to world space. Note that the w element is set to 0.0, as this is a
    // vector direction, not a position, That causes the matrix multiplication to ignore
    // the translations from the view matrix.
    let ray_direction = (view.world_from_view * vec4(view_ray_direction, 0.0)).xyz;

    return vec4(normalize(ray_direction), view_ray_direction.z);
}


/// Convert ndc depth to linear view z. 
/// Note: Depth values in front of the camera will be negative as -z is forward
fn depth_ndc_to_view_z(ndc_depth: f32) -> f32 {
    let view_pos = view.view_from_clip * vec4(0.0, 0.0, ndc_depth, 1.0);
    return view_pos.z / view_pos.w;
}


fn sample_local_illuminance(view_dir: vec3<f32>, altitude: f32) -> vec3<f32> {
    var local_illuminance = vec3(0.0);
    for (var light_i: u32 = 0u; light_i < lights.n_directional_lights; light_i++) {
        let light = &lights.directional_lights[light_i];
        let mu_light = (*light).direction_to_light.y;
        let neg_LdotV = dot(view_dir, (*light).direction_to_light);
        let rayleigh_phase = rayleigh(neg_LdotV);
        let mie_phase = henyey_greenstein(neg_LdotV, atmosphere.mie_asymmetry);
        let phase = rayleigh_phase + mie_phase; //TODO: check this

        let transmittance_to_light = sample_transmittance_lut(atmosphere, transmittance_lut, transmittance_lut_sampler, altitude, mu_light);
        let shadow_factor = transmittance_to_light * f32(!ray_intersects_ground(atmosphere, altitude, mu_light));

        let psi_ms = sample_multiscattering_lut(atmosphere, multiscattering_lut, multiscattering_lut_sampler, altitude, mu_light);

        local_illuminance += (transmittance_to_sample * shadow_factor * phase + psi_ms) * (*light).color.rgb; //TODO: what is color.a?
    }
}
