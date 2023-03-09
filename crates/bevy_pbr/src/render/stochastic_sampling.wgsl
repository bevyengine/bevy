#define_import_path bevy_pbr::stochastic_sampling

// https://developer.nvidia.com/blog/rendering-in-real-time-with-spatiotemporal-blue-noise-textures-part-1
// https://developer.nvidia.com/blog/rendering-in-real-time-with-spatiotemporal-blue-noise-textures-part-2
// https://tellusim.com/improved-blue-noise

const NOISE_SIZE = 64.0;
const NOISE_LAYERS = 64u;

// Given a texture you want to sample stochastically from by computing an average:
// sample_uv             = texture coordinates of center sampling point
// sample_i              = current sample number
// sampling_radius       = size of sampling radius around the center point in pixels
// sampling_texture_size = size of the texture you intend to sample
// returns               = texture coordinates to sample at
fn stochastic_uv(
    sample_uv: vec2<f32>,
    sample_i: u32,
    sampling_radius: f32,
    texture_size: vec2<i32>
) -> vec2<f32> {
    let texture_size = vec2<f32>(texture_size);

    // Convert center sampling coordinates to pixel units
    let sample_coords = sample_uv * texture_size;

    // Calculate what offset to sample the noise from in pixels [0, 64]
    let noise_offset = r2(sample_i) * NOISE_SIZE;

    // Calculate what UV coordinates to sample the noise at (wrapping sampler)
    let noise_uv = (sample_coords + noise_offset) / NOISE_SIZE;

    // Sample 2d noise based on sample_uv, sample_i, and frame_count
    let noise = textureSampleLevel(
        stochastic_noise,
        stochastic_noise_sampler,
        noise_uv,
        i32(globals.frame_count % NOISE_LAYERS), // Naga bug: need to use i32
        0.0,
    ).rg;

    // Map [0, 1] noise to [-radius, +radius] (in pixels)
    let sampling_offset_pixels = ((noise * 2.0) - 1.0) * f32(sampling_radius);

    // Convert sampling offset to UV coordinates
    let sample_offset = noise / texture_size;

    // Offset the initial sampling point
    return sample_uv + sample_offset;
}

// http://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences
// Returns a quasi random number in [0, 1]
fn r2(i: u32) -> vec2<f32> {
    let vec_i = vec2(f32(i));
    return fract(vec_i * vec2(0.754877666247, 0.569840290998) + 0.5);
}
