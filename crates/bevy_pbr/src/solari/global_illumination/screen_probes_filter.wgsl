#import bevy_solari::scene_bindings
#import bevy_solari::view_bindings
#import bevy_solari::utils

var<workgroup> spherical_harmonics_coefficents: array<array<vec3<f32>, 9>, 64>;

// TODO: Validate neighbor probe exists
// TODO: Angle weight
fn add_probe_contribution(
    irradiance_total: ptr<function, vec3<f32>>,
    weight_total: ptr<function, f32>,
    center_probe_depth: f32,
    cell_id: vec2<i32>,
    probe_id: vec2<i32>,
    probe_thread_id: vec2<i32>,
) {
    let probe_pixel_id = probe_thread_id + (8i * probe_id);
    let probe_depth = decode_g_buffer_depth(textureLoad(g_buffer, probe_pixel_id));

    let probe_irradiance = textureLoad(screen_probes_unfiltered, cell_id).rgb;

    let depth_weight = smoothstep(0.03, 0.0, abs(probe_depth - center_probe_depth));

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
    let probe_index = workgroup_id.x + workgroup_id.y * workgroup_count.x;
    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    let frame_index = uniforms.frame_count * 5782582u;
    var rng = pixel_index + frame_index;
    var rng2 = frame_index;

    let probe_thread_index = u32(floor(rand_f(&rng2) * 63.0));
    let probe_thread_x = probe_thread_index % 8u;
    let probe_thread_y = (probe_thread_index - probe_thread_x) / 8u;
    let probe_thread_id = vec2<i32>(vec2(probe_thread_x, probe_thread_y));

    let center_probe_id = vec2<i32>(workgroup_id.xy);
    let center_probe_pixel_id = probe_thread_id + (center_probe_id * 8i);
    let center_probe_depth = decode_g_buffer_depth(textureLoad(g_buffer, center_probe_pixel_id));

    var irradiance = vec3(0.0);
    var weight = 0.0;
    add_probe_contribution(&irradiance, &weight, center_probe_depth, vec2<i32>(global_id.xy) + vec2(-8i, 8i), center_probe_id + vec2(-1i, 1i), probe_thread_id);
    add_probe_contribution(&irradiance, &weight, center_probe_depth, vec2<i32>(global_id.xy) + vec2(0i, 8i), center_probe_id + vec2(0i, 1i), probe_thread_id);
    add_probe_contribution(&irradiance, &weight, center_probe_depth, vec2<i32>(global_id.xy) + vec2(8i, 8i), center_probe_id + vec2(1i, 1i), probe_thread_id);
    add_probe_contribution(&irradiance, &weight, center_probe_depth, vec2<i32>(global_id.xy) + vec2(-8i, 0i), center_probe_id + vec2(-1i, 0i), probe_thread_id);
    add_probe_contribution(&irradiance, &weight, center_probe_depth, vec2<i32>(global_id.xy) + vec2(0i, 0i), center_probe_id + vec2(0i, 0i), probe_thread_id);
    add_probe_contribution(&irradiance, &weight, center_probe_depth, vec2<i32>(global_id.xy) + vec2(8i, 0i), center_probe_id + vec2(1i, 0i), probe_thread_id);
    add_probe_contribution(&irradiance, &weight, center_probe_depth, vec2<i32>(global_id.xy) + vec2(-8i, -8i), center_probe_id + vec2(-1i, -1i), probe_thread_id);
    add_probe_contribution(&irradiance, &weight, center_probe_depth, vec2<i32>(global_id.xy) + vec2(0i, -8i), center_probe_id + vec2(0i, -1i), probe_thread_id);
    add_probe_contribution(&irradiance, &weight, center_probe_depth, vec2<i32>(global_id.xy) + vec2(8i, -8i), center_probe_id + vec2(1i, -1i), probe_thread_id);
    irradiance /= weight;

    // TODO: Remove unnecessary texture write + texture allocation #ifndef DEBUG_VIEW_SCREEN_PROBES_FILTERED
    textureStore(screen_probes_filtered, global_id.xy, vec4(irradiance, 1.0));

    let octahedral_pixel_center = vec2<f32>(local_id.xy) + rand_vec2(&rng);
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
    spherical_harmonics_coefficents[local_index][0] = L00;
    spherical_harmonics_coefficents[local_index][1] = L11;
    spherical_harmonics_coefficents[local_index][2] = L10;
    spherical_harmonics_coefficents[local_index][3] = L1_1;
    spherical_harmonics_coefficents[local_index][4] = L21;
    spherical_harmonics_coefficents[local_index][5] = L2_1;
    spherical_harmonics_coefficents[local_index][6] = L2_2;
    spherical_harmonics_coefficents[local_index][7] = L20;
    spherical_harmonics_coefficents[local_index][8] = L22;
    workgroupBarrier();
    for (var t = 32u; t > 0u; t >>= 1u) {
        if local_index < t {
            spherical_harmonics_coefficents[local_index][0] += spherical_harmonics_coefficents[local_index + t][0];
            spherical_harmonics_coefficents[local_index][1] += spherical_harmonics_coefficents[local_index + t][1];
            spherical_harmonics_coefficents[local_index][2] += spherical_harmonics_coefficents[local_index + t][2];
            spherical_harmonics_coefficents[local_index][3] += spherical_harmonics_coefficents[local_index + t][3];
            spherical_harmonics_coefficents[local_index][4] += spherical_harmonics_coefficents[local_index + t][4];
            spherical_harmonics_coefficents[local_index][5] += spherical_harmonics_coefficents[local_index + t][5];
            spherical_harmonics_coefficents[local_index][6] += spherical_harmonics_coefficents[local_index + t][6];
            spherical_harmonics_coefficents[local_index][7] += spherical_harmonics_coefficents[local_index + t][7];
            spherical_harmonics_coefficents[local_index][8] += spherical_harmonics_coefficents[local_index + t][8];
        }
        workgroupBarrier();
    }
    if local_index == 0u {
        L00 = spherical_harmonics_coefficents[0][0] / 64.0;
        L11 = spherical_harmonics_coefficents[0][1] / 64.0;
        L10 = spherical_harmonics_coefficents[0][2] / 64.0;
        L1_1 = spherical_harmonics_coefficents[0][3] / 64.0;
        L21 = spherical_harmonics_coefficents[0][4] / 64.0;
        L2_1 = spherical_harmonics_coefficents[0][5] / 64.0;
        L2_2 = spherical_harmonics_coefficents[0][6] / 64.0;
        L20 = spherical_harmonics_coefficents[0][7] / 64.0;
        L22 = spherical_harmonics_coefficents[0][8] / 64.0;
        screen_probes_spherical_harmonics[probe_index] = SphericalHarmonicsPacked(
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
