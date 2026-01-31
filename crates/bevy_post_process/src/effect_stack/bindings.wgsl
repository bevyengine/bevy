// The bindings of the effects.
#define_import_path bevy_post_process::effect_stack::bindings


const EPSILON: f32 = 1.19209290e-07;

// The source framebuffer texture.
@group(0) @binding(0) var source_texture: texture_2d<f32>;
// The sampler was used to sample the source framebuffer texture.
@group(0) @binding(1) var source_sampler: sampler;
// The 1D lookup table for chromatic aberration.
@group(0) @binding(2) var chromatic_aberration_lut_texture: texture_2d<f32>;
// The sampler was used to sample that lookup table.
@group(0) @binding(3) var chromatic_aberration_lut_sampler: sampler;
// The settings were supplied by the developer.
@group(0) @binding(4) var<uniform> chromatic_aberration_settings: ChromaticAberrationSettings;
// The settings were supplied by the developer.
@group(0) @binding(5) var<uniform> vignette_settings: VignetteSettings;
// The film grain texture.
@group(0) @binding(6) var film_grain_texture: texture_2d<f32>;
// The sampler was used to sample the film grain texture.
@group(0) @binding(7) var film_grain_sampler: sampler;
// The settings were supplied by the developer.
@group(0) @binding(8) var<uniform> film_grain_settings: FilmGrainSettings;

// See `bevy_post_process::effect_stack::ChromaticAberration` for more
// information on these fields.
struct ChromaticAberrationSettings {
    intensity: f32,
    max_samples: u32,
    unused_a: u32,
    unused_b: u32,
}

// See `bevy_post_process::effect_stack::Vignette` for more
// information on these fields.
struct VignetteSettings {
    intensity: f32,
    radius: f32,
    smoothness: f32,
    roundness: f32,
    center: vec2<f32>,
    edge_compensation: f32,
    unused: u32,
    color: vec4<f32>
}

// See `bevy_post_process::effect_stack::FilmGrain` for more
// information on these fields.
struct FilmGrainSettings {
    intensity: f32,
    shadows_intensity: f32,
    midtones_intensity: f32,
    highlights_intensity: f32,
    shadows_threshold: f32,
    highlights_threshold: f32,
    grain_size: f32,
    frame:u32
}
