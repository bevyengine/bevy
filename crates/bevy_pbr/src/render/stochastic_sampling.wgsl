#define_import_path bevy_pbr::stochastic_sampling

// https://developer.nvidia.com/blog/rendering-in-real-time-with-spatiotemporal-blue-noise-textures-part-1
// https://developer.nvidia.com/blog/rendering-in-real-time-with-spatiotemporal-blue-noise-textures-part-2
// https://tellusim.com/improved-blue-noise

// Given a texture you want to sample stochastically from by computing an average:
// sample_uv           = texture coordinates of center sampling point
// sample_i            = current sample number
// sample_offset_scale = sampling_radius / texture_size
// returns             = texture coordinates to sample at
fn stochastic_uv(sample_uv: vec2<f32>, sample_i: u32, sample_offset_scale: vec2<f32>) -> vec2<f32> {
    var noise = textureSampleLevel(
        stochastic_noise,
        dt_lut_sampler, // TODO
        sample_uv + r2(sample_i),
        i32(globals.frame_count % 64u), // Naga bug: need to use i32
        0.0,
    ).rg;

    noise = (noise * 2.0) - 1.0;
    let offset = noise * sample_offset_scale;

    return sample_uv + offset;
}

// http://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences
fn r2(i: u32) -> vec2<f32> {
    let vec_i = vec2(f32(i));
    return fract(vec_i * vec2(0.754877666247, 0.569840290998) + 0.5);
}
