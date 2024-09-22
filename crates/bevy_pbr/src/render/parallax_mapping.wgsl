#define_import_path bevy_pbr::parallax_mapping

#import bevy_pbr::pbr_bindings::{depth_map_texture, depth_map_sampler}

fn sample_depth_map(uv: vec2<f32>) -> f32 {
    // We use `textureSampleLevel` over `textureSample` because the wgpu DX12
    // backend (Fxc) panics when using "gradient instructions" inside a loop.
    // It results in the whole loop being unrolled by the shader compiler,
    // which it can't do because the upper limit of the loop in steep parallax
    // mapping is a variable set by the user.
    // The "gradient instructions" comes from `textureSample` computing MIP level
    // based on UV derivative. With `textureSampleLevel`, we provide ourselves
    // the MIP level, so no gradient instructions are used, and we can use
    // sample_depth_map in our loop.
    // See https://stackoverflow.com/questions/56581141/direct3d11-gradient-instruction-used-in-a-loop-with-varying-iteration-forcing
    return textureSampleLevel(depth_map_texture, depth_map_sampler, uv, 0.0).r;
}

// An implementation of parallax mapping, see https://en.wikipedia.org/wiki/Parallax_mapping
// Code derived from: https://web.archive.org/web/20150419215321/http://sunandblackcat.com/tipFullView.php?l=eng&topicid=28
fn parallaxed_uv(
    depth_scale: f32,
    max_layer_count: f32,
    max_steps: u32,
    // The original interpolated uv
    original_uv: vec2<f32>,
    // The vector from the camera to the fragment at the surface in tangent space
    Vt: vec3<f32>,
) -> vec2<f32> {
    if max_layer_count < 1.0 {
        return original_uv;
    }
    var uv = original_uv;

    // Steep Parallax Mapping
    // ======================
    // Split the depth map into `layer_count` layers.
    // When Vt hits the surface of the mesh (excluding depth displacement),
    // if the depth is not below or on surface including depth displacement (textureSample), then
    // look forward (+= delta_uv) on depth texture according to
    // Vt and distance between hit surface and depth map surface,
    // repeat until below the surface.
    //
    // Where `layer_count` is interpolated between `1.0` and
    // `max_layer_count` according to the steepness of Vt.

    let view_steepness = abs(Vt.z);
    // We mix with minimum value 1.0 because otherwise,
    // with 0.0, we get a division by zero in surfaces parallel to viewport,
    // resulting in a singularity.
    let layer_count = mix(max_layer_count, 1.0, view_steepness);
    let layer_depth = 1.0 / layer_count;
    var delta_uv = depth_scale * layer_depth * Vt.xy * vec2(1.0, -1.0) / view_steepness;

    var current_layer_depth = 0.0;
    var texture_depth = sample_depth_map(uv);

    // texture_depth > current_layer_depth means the depth map depth is deeper
    // than the depth the ray would be at this UV offset so the ray has not
    // intersected the surface
    for (var i: i32 = 0; texture_depth > current_layer_depth && i <= i32(layer_count); i++) {
        current_layer_depth += layer_depth;
        uv += delta_uv;
        texture_depth = sample_depth_map(uv);
    }

#ifdef RELIEF_MAPPING
    // Relief Mapping
    // ==============
    // "Refine" the rough result from Steep Parallax Mapping
    // with a **binary search** between the layer selected by steep parallax
    // and the next one to find a point closer to the depth map surface.
    // This reduces the jaggy step artifacts from steep parallax mapping.

    delta_uv *= 0.5;
    var delta_depth = 0.5 * layer_depth;

    uv -= delta_uv;
    current_layer_depth -= delta_depth;

    for (var i: u32 = 0u; i < max_steps; i++) {
        texture_depth = sample_depth_map(uv);

        // Halve the deltas for the next step
        delta_uv *= 0.5;
        delta_depth *= 0.5;

        // Step based on whether the current depth is above or below the depth map
        if (texture_depth > current_layer_depth) {
            uv += delta_uv;
            current_layer_depth += delta_depth;
        } else {
            uv -= delta_uv;
            current_layer_depth -= delta_depth;
        }
    }
#else
    // Parallax Occlusion mapping
    // ==========================
    // "Refine" Steep Parallax Mapping by interpolating between the
    // previous layer's depth and the computed layer depth.
    // Only requires a single lookup, unlike Relief Mapping, but
    // may skip small details and result in writhing material artifacts.
    let previous_uv = uv - delta_uv;
    let next_depth = texture_depth - current_layer_depth;
    let previous_depth = sample_depth_map(previous_uv) - current_layer_depth + layer_depth;

    let weight = next_depth / (next_depth - previous_depth);

    uv = mix(uv, previous_uv, weight);

    current_layer_depth += mix(next_depth, previous_depth, weight);
#endif

    // Note: `current_layer_depth` is not returned, but may be useful
    // for light computation later on in future improvements of the pbr shader.
    return uv;
}
