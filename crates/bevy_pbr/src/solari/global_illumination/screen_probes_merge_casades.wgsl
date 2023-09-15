#import bevy_solari::global_illumination::view_bindings screen_probes_a, screen_probes_b

var<push_constant> lower_cascade: u32;

@compute @workgroup_size(8, 8, 1)
fn merge_screen_probe_cascades(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let lower_probe_size = u32(pow(2.0, f32(lower_cascade) + 3.0));
    let upper_probe_size = lower_probe_size * 2u;

    let adjusted_pixel_id = clamp(global_id.xy - lower_probe_size - 1u, vec2(0u), textureDimensions(screen_probes_a));
    let upper_probe_pixel_uv = (vec2<f32>(adjusted_pixel_id) + 0.5) / f32(upper_probe_size);

    let tl_probe_sample = sample_upper_probe(vec2<u32>(upper_probe_pixel_uv) * upper_probe_size);
    let tr_probe_sample = sample_upper_probe(vec2<u32>(upper_probe_pixel_uv + vec2(0.0, 1.0)) * upper_probe_size);
    let bl_probe_sample = sample_upper_probe(vec2<u32>(upper_probe_pixel_uv + vec2(1.0, 0.0)) * upper_probe_size);
    let br_probe_sample = sample_upper_probe(vec2<u32>(upper_probe_pixel_uv + vec2(1.0, 1.0)) * upper_probe_size);

    let r = fract(upper_probe_pixel_uv * f32(upper_probe_size) - 0.5);
    // TODO: Multiply weights by depth weights per probe
    let tl_probe_weight = (1.0 - r.x) * (1.0 - r.y);
    let tr_probe_weight = r.x * (1.0 - r.y);
    let bl_probe_weight = r.x * r.y;
    let br_probe_weight = (1. - r.x) * r.y;

    var upper_cascade_interpolated = (tl_probe_sample * tl_probe_weight) + (tr_probe_sample * tr_probe_weight) + (bl_probe_sample * bl_probe_weight) + (br_probe_sample * br_probe_weight) ;
    upper_cascade_interpolated /= tl_probe_weight + tr_probe_weight + bl_probe_weight + br_probe_weight;

    let lower_cascade_sample = textureLoad(screen_probes_a, global_id.xy, lower_cascade);
    let merged_sample_rgb = lower_cascade_sample.rgb + (lower_cascade_sample.a * upper_cascade_interpolated.rgb);
    let merged_sample_a = lower_cascade_sample.a * upper_cascade_interpolated.a;
    let merged_sample = vec4(merged_sample_rgb, merged_sample_a);

    if lower_cascade != 0u {
        textureStore(screen_probes_a, global_id.xy, 0i, merged_sample);
    } else {
        textureStore(screen_probes_b, global_id.xy, merged_sample);
    }
}

fn sample_upper_probe(pixel_id: vec2<u32>) -> vec4<f32> {
    let tl_direction_sample = textureLoad(screen_probes_a, pixel_id, lower_cascade + 1u);
    let tr_direction_sample = textureLoad(screen_probes_a, pixel_id + vec2(0u, 1u), lower_cascade + 1u);
    let bl_direction_sample = textureLoad(screen_probes_a, pixel_id + vec2(1u, 0u), lower_cascade + 1u);
    let br_direction_sample = textureLoad(screen_probes_a, pixel_id + vec2(1u, 1u), lower_cascade + 1u);
    return (tl_direction_sample + tr_direction_sample + bl_direction_sample + br_direction_sample) / 4.0;
}
