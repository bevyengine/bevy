#import bevy_solari::scene_bindings uniforms
#import bevy_solari::global_illumination::view_bindings depth_buffer, screen_probes_a, screen_probes_b, screen_probes_spherical_harmonics, view, SphericalHarmonicsPacked
#import bevy_solari::utils rand_vec2f, get_spherical_harmonics_coefficents
#import bevy_pbr::utils octahedral_decode, PI

// TODO: Angle weight
fn add_probe_contribution(
    cell_id: vec2<i32>,
    irradiance_total: ptr<function, vec3<f32>>,
    weight_total: ptr<function, f32>,
    center_probe_depth: f32,
) {
    let probe_depth = view.projection[3][2] / textureLoad(depth_buffer, cell_id, 0i);

#ifdef FIRST_PASS
    let probe_irradiance = textureLoad(screen_probes_a, cell_id, 0i).rgb;
#else
    let probe_irradiance = textureLoad(screen_probes_b, cell_id).rgb;
#endif

    let depth_weight = pow(saturate(1.0 - abs(probe_depth - center_probe_depth) / center_probe_depth), 8.0);

    *weight_total += depth_weight;
    *irradiance_total += probe_irradiance * depth_weight;
}

@compute @workgroup_size(8, 8, 1)
fn filter_screen_probes(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(local_invocation_index) local_index: u32,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(num_workgroups) workgroup_count: vec3<u32>,
) {
    let center_probe_index = workgroup_id.x + workgroup_id.y * workgroup_count.x;
    let center_probe_id = vec2<i32>(workgroup_id.xy);
    let center_probe_center_pixel_id = (workgroup_id.xy * 8u) + 3u;
    let center_probe_depth = view.projection[3][2] / textureLoad(depth_buffer, center_probe_center_pixel_id, 0i);
    let probe_count = vec2<i32>(textureDimensions(screen_probes_a) / 8u);

#ifdef FIRST_PASS
    let direction = vec2(0i, 1i);
#else
    let direction = vec2(1i, 0i);
#endif

    var irradiance = vec3(0.0);
    var weight = 0.0;
    for (var step = -2i; step <= 2i; step++) {
        let offset = direction * step;
        let probe_id = clamp(center_probe_id + offset, vec2(0i), probe_count);
        let cell_id = (probe_id * 8i) + vec2<i32>(local_id.xy);
        add_probe_contribution(cell_id, &irradiance, &weight, center_probe_depth);
    }
    irradiance /= weight;
    irradiance = max(irradiance, vec3(0.0));

#ifdef FIRST_PASS
    textureStore(screen_probes_b, global_id.xy, vec4(irradiance, 1.0));
#else
    convert_to_spherical_harmonics(irradiance, local_id.xy, local_index, center_probe_index);
#endif
}

#ifndef FIRST_PASS
var<workgroup> sh_coefficents: array<array<vec3<f32>, 9>, 64>;

// TODO: Replace with subgroup/wave ops when supported
fn convert_to_spherical_harmonics(irradiance: vec3<f32>, cell_id: vec2<u32>, cell_index: u32, center_probe_index: u32) {
    let octahedral_pixel_center = vec2<f32>(cell_id) + 0.5;
    let octahedral_normal = octahedral_decode(octahedral_pixel_center / 8.0);

    let local_sh = get_spherical_harmonics_coefficents(octahedral_normal);
    sh_coefficents[cell_index][0] = local_sh[0] * irradiance;
    sh_coefficents[cell_index][1] = local_sh[1] * irradiance;
    sh_coefficents[cell_index][2] = local_sh[2] * irradiance;
    sh_coefficents[cell_index][3] = local_sh[3] * irradiance;
    sh_coefficents[cell_index][4] = local_sh[4] * irradiance;
    sh_coefficents[cell_index][5] = local_sh[5] * irradiance;
    sh_coefficents[cell_index][6] = local_sh[6] * irradiance;
    sh_coefficents[cell_index][7] = local_sh[7] * irradiance;
    sh_coefficents[cell_index][8] = local_sh[8] * irradiance;
    workgroupBarrier();
    for (var t = 32u; t > 0u; t >>= 1u) {
        if cell_index < t {
            for (var i = 0u; i < 9u; i++) {
                sh_coefficents[cell_index][i] += sh_coefficents[cell_index + t][i];
            }
        }
        workgroupBarrier();
    }

    if cell_index == 0u {
        var sh: array<vec3<f32>, 9>;
        for (var i = 0u; i < 9u; i++) {
            sh[i] = sh_coefficents[0][i] * (4.0 * PI) / 64.0;
        }

        screen_probes_spherical_harmonics[center_probe_index] = SphericalHarmonicsPacked(
            vec4(sh[0], sh[1].x),
            vec4(sh[1].yz, sh[2].xy),
            vec4(sh[2].z, sh[3]),
            vec4(sh[4], sh[5].x),
            vec4(sh[5].yz, sh[6].xy),
            vec4(sh[6].z, sh[7]),
            vec4(sh[8], 0.0),
        );
    }
}
#endif
