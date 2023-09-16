#import bevy_solari::scene_bindings uniforms
#import bevy_solari::global_illumination::view_bindings depth_buffer, screen_probes_a, screen_probes_b, screen_probes_spherical_harmonics, view, SphericalHarmonicsPacked
#import bevy_solari::utils rand_f, rand_vec2f
#import bevy_pbr::utils octahedral_decode

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
var<workgroup> spherical_harmonics_coefficents: array<array<vec3<f32>, 9>, 64>;

fn convert_to_spherical_harmonics(irradiance: vec3<f32>, cell_id: vec2<u32>, cell_index: u32, center_probe_index: u32) {
    let octahedral_pixel_center = vec2<f32>(cell_id) + 0.5;
    let octahedral_normal = octahedral_decode(octahedral_pixel_center / 8.0);
    let x = octahedral_normal.x;
    let y = octahedral_normal.y;
    let z = octahedral_normal.z;
    let xz = x * z;
    let yz = y * z;
    let xy = x * y;
    let zz = z * z;
    let xx_yy = x * x - y * y;

    var L00 = (0.282095) * irradiance;
    var L11 = (0.488603 * x) * irradiance;
    var L10 = (0.488603 * z) * irradiance;
    var L1_1 = (0.488603 * y) * irradiance;
    var L21 = (1.092548 * xz) * irradiance;
    var L2_1 = (1.092548 * yz) * irradiance;
    var L2_2 = (1.092548 * xy) * irradiance;
    var L20 = (0.946176 * zz - 0.315392) * irradiance;
    var L22 = (0.546274 * xx_yy) * irradiance;

    // TODO: Replace with subgroup/wave ops when supported
    spherical_harmonics_coefficents[cell_index][0] = L00;
    spherical_harmonics_coefficents[cell_index][1] = L11;
    spherical_harmonics_coefficents[cell_index][2] = L10;
    spherical_harmonics_coefficents[cell_index][3] = L1_1;
    spherical_harmonics_coefficents[cell_index][4] = L21;
    spherical_harmonics_coefficents[cell_index][5] = L2_1;
    spherical_harmonics_coefficents[cell_index][6] = L2_2;
    spherical_harmonics_coefficents[cell_index][7] = L20;
    spherical_harmonics_coefficents[cell_index][8] = L22;
    workgroupBarrier();
    for (var t = 32u; t > 0u; t >>= 1u) {
        if cell_index < t {
            spherical_harmonics_coefficents[cell_index][0] += spherical_harmonics_coefficents[cell_index + t][0];
            spherical_harmonics_coefficents[cell_index][1] += spherical_harmonics_coefficents[cell_index + t][1];
            spherical_harmonics_coefficents[cell_index][2] += spherical_harmonics_coefficents[cell_index + t][2];
            spherical_harmonics_coefficents[cell_index][3] += spherical_harmonics_coefficents[cell_index + t][3];
            spherical_harmonics_coefficents[cell_index][4] += spherical_harmonics_coefficents[cell_index + t][4];
            spherical_harmonics_coefficents[cell_index][5] += spherical_harmonics_coefficents[cell_index + t][5];
            spherical_harmonics_coefficents[cell_index][6] += spherical_harmonics_coefficents[cell_index + t][6];
            spherical_harmonics_coefficents[cell_index][7] += spherical_harmonics_coefficents[cell_index + t][7];
            spherical_harmonics_coefficents[cell_index][8] += spherical_harmonics_coefficents[cell_index + t][8];
        }
        workgroupBarrier();
    }

    if cell_index == 0u {
        L00 = spherical_harmonics_coefficents[0][0] / 64.0;
        L11 = spherical_harmonics_coefficents[0][1] / 64.0;
        L10 = spherical_harmonics_coefficents[0][2] / 64.0;
        L1_1 = spherical_harmonics_coefficents[0][3] / 64.0;
        L21 = spherical_harmonics_coefficents[0][4] / 64.0;
        L2_1 = spherical_harmonics_coefficents[0][5] / 64.0;
        L2_2 = spherical_harmonics_coefficents[0][6] / 64.0;
        L20 = spherical_harmonics_coefficents[0][7] / 64.0;
        L22 = spherical_harmonics_coefficents[0][8] / 64.0;
        screen_probes_spherical_harmonics[center_probe_index] = SphericalHarmonicsPacked(
            vec4(L00, L11.x),
            vec4(L11.yz, L10.xy),
            vec4(L10.z, L1_1),
            vec4(L21, L2_1.x),
            vec4(L2_1.yz, L2_2.xy),
            vec4(L2_2.z, L20),
            L22,
        );
    }
}
#endif
