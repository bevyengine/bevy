// The chromatic aberration postprocessing effect.
//
// This makes edges of objects turn into multicolored streaks.

#define_import_path bevy_core_pipeline::post_processing::chromatic_aberration

// See `bevy_core_pipeline::post_process::ChromaticAberration` for more
// information on these fields.
struct ChromaticAberrationSettings {
    intensity: f32,
    max_samples: u32,
    unused_a: u32,
    unused_b: u32,
}

// The source framebuffer texture.
@group(0) @binding(0) var chromatic_aberration_source_texture: texture_2d<f32>;
// The sampler used to sample the source framebuffer texture.
@group(0) @binding(1) var chromatic_aberration_source_sampler: sampler;
// The 1D lookup table for chromatic aberration.
@group(0) @binding(2) var chromatic_aberration_lut_texture: texture_2d<f32>;
// The sampler used to sample that lookup table.
@group(0) @binding(3) var chromatic_aberration_lut_sampler: sampler;
// The settings supplied by the developer.
@group(0) @binding(4) var<uniform> chromatic_aberration_settings: ChromaticAberrationSettings;

fn chromatic_aberration(start_pos: vec2<f32>) -> vec3<f32> {
    // Radial chromatic aberration implemented using the *Inside* technique:
    //
    // <https://github.com/playdeadgames/publications/blob/master/INSIDE/rendering_inside_gdc2016.pdf>

    let end_pos = mix(start_pos, vec2(0.5), chromatic_aberration_settings.intensity);

    // Determine the number of samples. We aim for one sample per texel, unless
    // that's higher than the developer-specified maximum number of samples, in
    // which case we choose the maximum number of samples.
    let texel_length = length((end_pos - start_pos) *
        vec2<f32>(textureDimensions(chromatic_aberration_source_texture)));
    let sample_count = min(u32(ceil(texel_length)), chromatic_aberration_settings.max_samples);

    var color: vec3<f32>;
    if (sample_count > 1u) {
        // The LUT texture is in clamp-to-edge mode, so we start at 0.5 texels
        // from the sides so that we have a nice gradient over the entire LUT
        // range.
        let lut_u_offset = 0.5 / f32(textureDimensions(chromatic_aberration_lut_texture).x);

        var sample_sum = vec3(0.0);
        var modulate_sum = vec3(0.0);

        // Start accumulating samples.
        for (var sample_index = 0u; sample_index < sample_count; sample_index += 1u) {
            let t = (f32(sample_index) + 0.5) / f32(sample_count);

            // Sample the framebuffer.
            let sample_uv = mix(start_pos, end_pos, t);
            let sample = textureSampleLevel(
                chromatic_aberration_source_texture,
                chromatic_aberration_source_sampler,
                sample_uv,
                0.0,
            ).rgb;

            // Sample the LUT.
            let lut_u = mix(lut_u_offset, 1.0 - lut_u_offset, t);
            let modulate = textureSampleLevel(
                chromatic_aberration_lut_texture,
                chromatic_aberration_lut_sampler,
                vec2(lut_u, 0.5),
                0.0,
            ).rgb;

            // Modulate the sample by the LUT value.
            sample_sum += sample * modulate;
            modulate_sum += modulate;
        }

        color = sample_sum / modulate_sum;
    } else {
        // If there's only one sample, don't do anything. If we don't do this,
        // then this shader will apply whatever tint is in the center of the LUT
        // texture to such pixels, which is wrong.
        color = textureSampleLevel(
            chromatic_aberration_source_texture,
            chromatic_aberration_source_sampler,
            start_pos,
            0.0,
        ).rgb;
    }

    return color;
}
