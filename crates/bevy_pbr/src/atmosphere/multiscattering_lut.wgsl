#import bevy_pbr::{
    mesh_view_types::{Lights, DirectionalLight},
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        bindings::{atmosphere, settings},
        functions::{
            multiscattering_lut_uv_to_r_mu, sample_transmittance_lut,
            get_local_r, get_local_up, sample_atmosphere, FRAC_4_PI,
            max_atmosphere_distance, rayleigh, henyey_greenstein,
            zenith_azimuth_to_ray_dir,
        },
        bruneton_functions::{
            distance_to_top_atmosphere_boundary, distance_to_bottom_atmosphere_boundary, ray_intersects_ground
        }
    }
}

#import bevy_render::maths::{PI,PI_2}

const PHI_2: vec2<f32> = vec2(1.3247179572447460259609088, 1.7548776662466927600495087);

@group(0) @binding(13) var multiscattering_lut_out: texture_storage_2d<rgba16float, write>;

fn s2_sequence(n: u32) -> vec2<f32> {
    return fract(0.5 + f32(n) * PHI_2);
}

// Lambert equal-area projection. 
fn uv_to_sphere(uv: vec2<f32>) -> vec3<f32> {
    let phi = PI_2 * uv.y;
    let sin_lambda = 2 * uv.x - 1;
    let cos_lambda = sqrt(1 - sin_lambda * sin_lambda);

    return vec3(cos_lambda * cos(phi), cos_lambda * sin(phi), sin_lambda);
}

// Shared memory arrays for workgroup communication
var<workgroup> multi_scat_shared_mem: array<vec3<f32>, 64>;
var<workgroup> l_shared_mem: array<vec3<f32>, 64>;

@compute 
@workgroup_size(1, 1, 64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var uv = (vec2<f32>(global_id.xy) + 0.5) / vec2<f32>(settings.multiscattering_lut_size);

    let r_mu = multiscattering_lut_uv_to_r_mu(uv);
    let light_dir = normalize(vec3(0.0, r_mu.y, -1.0));

    let ray_dir = uv_to_sphere(s2_sequence(global_id.z));
    let ms_sample = sample_multiscattering_dir(r_mu.x, ray_dir, light_dir);
    
    // Calculate the contribution for this sample
    let sphere_solid_angle = 4.0 * PI;
    let sample_weight = sphere_solid_angle / 64.0;
    multi_scat_shared_mem[global_id.z] = ms_sample.f_ms * sample_weight;
    l_shared_mem[global_id.z] = ms_sample.l_2 * sample_weight;

    workgroupBarrier();

    // Parallel reduction bitshift to the right to divide by 2 each step
    for (var step = 32u; step > 0u; step >>= 1u) {
        if global_id.z < step {
            multi_scat_shared_mem[global_id.z] += multi_scat_shared_mem[global_id.z + step];
            l_shared_mem[global_id.z] += l_shared_mem[global_id.z + step];
        }
        workgroupBarrier();
    }

    if global_id.z > 0u {
        return;
    }

    // Apply isotropic phase function
    let f_ms = multi_scat_shared_mem[0] * FRAC_4_PI;
    let l_2 = l_shared_mem[0] * FRAC_4_PI;
    
    // Equation 10 from the paper: Geometric series for infinite scattering
    let psi_ms = l_2 / (1.0 - f_ms);
    textureStore(multiscattering_lut_out, global_id.xy, vec4<f32>(psi_ms, 1.0));
}

struct MultiscatteringSample {
    l_2: vec3<f32>,
    f_ms: vec3<f32>,
};

fn sample_multiscattering_dir(r: f32, ray_dir: vec3<f32>, light_dir: vec3<f32>) -> MultiscatteringSample {
    // get the cosine of the zenith angle of the view direction with respect to the light direction
    let mu_view = ray_dir.y;
    let t_max = max_atmosphere_distance(r, mu_view);

    let dt = t_max / f32(settings.multiscattering_lut_samples);
    var optical_depth = vec3<f32>(0.0);

    var l_2 = vec3(0.0);
    var f_ms = vec3(0.0);
    var throughput = vec3(1.0);
    for (var i: u32 = 0u; i < settings.multiscattering_lut_samples; i++) {
        let t_i = dt * (f32(i) + 0.5);
        let local_r = get_local_r(r, mu_view, t_i);
        let local_up = get_local_up(r, t_i, ray_dir);

        let local_atmosphere = sample_atmosphere(local_r);
        let sample_optical_depth = local_atmosphere.extinction * dt;
        let sample_transmittance = exp(-sample_optical_depth);
        optical_depth += sample_optical_depth;

        let mu_light = dot(light_dir, local_up);
        let scattering_no_phase = local_atmosphere.rayleigh_scattering + local_atmosphere.mie_scattering;

        let ms = scattering_no_phase;
        let ms_int = (ms - ms * sample_transmittance) / local_atmosphere.extinction;
        f_ms += throughput * ms_int;

        let transmittance_to_light = sample_transmittance_lut(local_r, mu_light);
        let shadow_factor = transmittance_to_light * f32(!ray_intersects_ground(local_r, mu_light));

        let s = scattering_no_phase * shadow_factor * FRAC_4_PI;
        let s_int = (s - s * sample_transmittance) / local_atmosphere.extinction;
        l_2 += throughput * s_int;

        throughput *= sample_transmittance;
        if all(throughput < vec3(0.001)) {
            break;
        }
    }

    //include reflected luminance from planet ground 
    if ray_intersects_ground(r, mu_view) {
        let transmittance_to_ground = exp(-optical_depth);
        let local_up = get_local_up(r, t_max, ray_dir);
        let mu_light = dot(light_dir, local_up);
        let transmittance_to_light = sample_transmittance_lut(0.0, mu_light);
        let ground_luminance = transmittance_to_light * transmittance_to_ground * max(mu_light, 0.0) * atmosphere.ground_albedo;
        l_2 += ground_luminance;
    }

    return MultiscatteringSample(l_2, f_ms);
}
