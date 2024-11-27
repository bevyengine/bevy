#import bevy_pbr::{
    mesh_view_types::{Lights, DirectionalLight},
    atmosphere::{
        types::{Atmosphere, AtmosphereSettings},
        bindings::{atmosphere, settings},
        functions::{
            multiscattering_lut_uv_to_r_mu, sample_transmittance_lut, isotropic, 
            get_local_r, get_local_up, sample_atmosphere, FRAC_4_PI, TAU,
            max_atmosphere_distance
        },
        bruneton_functions::{
            distance_to_top_atmosphere_boundary, distance_to_bottom_atmosphere_boundary, ray_intersects_ground
        }
    }
}


const PHI_2: vec2<f32> = vec2(1.3247179572447460259609088, 1.7548776662466927600495087);

@group(0) @binding(13) var multiscattering_lut_out: texture_storage_2d<rgba16float, write>;

fn s2_sequence(n: u32) -> vec2<f32> {
    return fract(0.5 + f32(n) * PHI_2);
}

//Lambert equal-area projection. 
fn map_to_sphere(uv: vec2<f32>) -> vec3<f32> {
    let phi = TAU * uv.y;
    let sin_lambda = 2 * uv.x - 1;
    let cos_lambda = cos(asin(sin_lambda));

    return vec3(cos_lambda * cos(phi), sin_lambda, cos_lambda * sin(phi));
}

@compute 
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let uv: vec2<f32> = (vec2<f32>(global_id.xy) + 0.5) / vec2<f32>(settings.multiscattering_lut_size);

    //See Multiscattering LUT parametrization
    let r_mu = multiscattering_lut_uv_to_r_mu(uv);

    //single directional light is oriented exactly along the z axis, 
    //with an zenith angle corresponding to mu
    let light_dir = normalize(vec3(0.0, r_mu.y, -1.0));

    var l_2 = vec3(0.0);
    var f_ms = vec3(0.0);

    for (var i: u32 = 0u; i < settings.multiscattering_lut_dirs; i++) {
        let ray_dir = map_to_sphere(s2_sequence(i));
        let mu = ray_dir.y; // cos(zenith_angle) = dot(vec3::up, dir);

        let ms_sample = sample_multiscattering_dir(r_mu.x, r_mu.y, ray_dir, light_dir);
        l_2 += ms_sample.l_2;
        f_ms += ms_sample.f_ms;
    }

    // xFRAC_4_PI since we're integrating over a sphere
    l_2 *= FRAC_4_PI;
    //l_2 /= f32(settings.multiscattering_lut_dirs);
    f_ms *= FRAC_4_PI;
    //f_ms /= f32(settings.multiscattering_lut_dirs);

    let F_ms = 1 / (1 - f_ms);
    let phi_ms = l_2 * F_ms;

    textureStore(multiscattering_lut_out, global_id.xy, vec4(phi_ms, 1.0));
}

struct MultiscatteringSample {
    l_2: vec3<f32>,
    f_ms: vec3<f32>,
};

fn sample_multiscattering_dir(r: f32, mu: f32, ray_dir: vec3<f32>, light_dir: vec3<f32>) -> MultiscatteringSample {
    let t_max = max_atmosphere_distance(r, mu);

    let dt = t_max / f32(settings.multiscattering_lut_samples);
    var optical_depth = vec3<f32>(0.0);

    var l_2 = vec3(0.0);
    var f_ms = vec3(0.0);

    for (var i: u32 = 0u; i < settings.multiscattering_lut_samples; i++) {
        let t_i = dt * (f32(i) + 0.5);
        let local_r = get_local_r(r, mu, t_i);
        let local_up = get_local_up(r, t_i, ray_dir);

        let local_atmosphere = sample_atmosphere(local_r);
        optical_depth += local_atmosphere.extinction * dt;
        let transmittance_to_sample = exp(-optical_depth);

        let mu_light = dot(light_dir, local_up);
        let scattering = local_atmosphere.rayleigh_scattering + local_atmosphere.mie_scattering;
        let isotropic_phase = FRAC_4_PI;

        let transmittance_to_light = sample_transmittance_lut(local_r, mu_light);
        let shadow_factor = transmittance_to_light * f32(!ray_intersects_ground(local_r, mu_light));

        let energy_transfer = transmittance_to_sample * scattering;

        l_2 += energy_transfer * isotropic_phase * shadow_factor * dt;
        f_ms += energy_transfer * isotropic_phase * dt;
    }

    return MultiscatteringSample(l_2, f_ms);
}
