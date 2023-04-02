#define_import_path bevy_pbr::parallax_mapping

fn sample_depth_map(uv: vec2<f32>) -> f32 {
    return textureSample(depth_map_texture, depth_map_sampler, uv).r;
}

// An implementation of parallax mapping, see https://en.wikipedia.org/wiki/Parallax_mapping
// Code derived from: https://web.archive.org/web/20150419215321/http://sunandblackcat.com/tipFullView.php?l=eng&topicid=28
fn parallaxed_uv(
    depth: f32,
    max_layer_count: f32,
    max_steps: u32,
    // The original uv
    uv: vec2<f32>,
    // The vector from camera to the surface of material
    V: vec3<f32>,
) -> vec2<f32> {
    var uv = uv;
    if max_layer_count < 1.0 {
        return uv;
    }

    // Steep Parallax Mapping
    // ======================
    // Split the depth map into `layer_count` layers.
    // When V hits the surface of the mesh (excluding depth displacement),
    // if the depth is not below or on surface including depth displacement (textureSample), then
    // look forward (-= delta_uv) according to V and distance between hit surface and
    // depth map surface, repeat until below the surface.
    //
    // Where `layer_count` is interpolated between `min_layer_count` and
    // `max_layer_count` according to the steepness of V.

    let view_steepness = abs(dot(vec3<f32>(0.0, 0.0, 1.0), V));
    // We mix with minimum value 1.0 because otherwise, with 0.0, we get
    // a nice division by zero in surfaces parallel to viewport, resulting
    // in a singularity.
    let layer_count = mix(max_layer_count, 1.0, view_steepness);
    let layer_height = 1.0 / layer_count;
    var delta_uv = depth * V.xy / V.z / layer_count;

    var current_layer_height = 0.0;
    var current_height = sample_depth_map(uv);

    // This at most runs layer_count times
    for (var i: i32 = 0; current_height > current_layer_height && i <= i32(layer_count); i++) {
        current_layer_height += layer_height;
        uv -= delta_uv;
        current_height = sample_depth_map(uv);
    }

#ifdef RELIEF_MAPPING
    // Relief Mapping
    // ==============
    // "Refine" the rough result from Steep Parallax Mapping
    // with a binary search between the layer selected by steep parallax
    // and the next one to find a point closer to the depth map surface.
    // This reduces the jaggy step artifacts from steep parallax mapping.

    delta_uv *= 0.5;
    var delta_height = 0.5 * layer_height;
    uv += delta_uv;
    current_layer_height -= delta_height;
    for (var i: u32 = 0u; i < max_steps; i++) {
        // Sample depth at current offset
        current_height = sample_depth_map(uv);

        // Halve the deltas for the next step
        delta_uv *= 0.5;
        delta_height *= 0.5;

        // Step based on whether the current depth is above or below the depth map
        if (current_height > current_layer_height) {
            uv -= delta_uv;
            current_layer_height += delta_height;
        } else {
            uv += delta_uv;
            current_layer_height -= delta_height;
        }
    }
#else    
    // Parallax Occlusion mapping
    // ==========================
    // "Refine" Steep Parallax Mapping by interpolating between the
    // previous layer's height and the computed layer height.
    // Only requires a single lookup, unlike Relief Mapping, but
    // may incur artifacts on very steep relief.
    let previous_uv = uv + delta_uv;
    let next_height = current_height - current_layer_height;
    let previous_height = sample_depth_map(previous_uv) - current_layer_height + layer_height;

    let weight = next_height / (next_height - previous_height);

    uv = mix(uv, previous_uv, weight);

    current_layer_height += mix(next_height, previous_height, weight);
#endif

    // Note: `current_layer_height` is not returned, but may be useful
    // for light computation later on in future improvements of the pbr shader.
    return uv;
}
