// The vignette postprocessing effect.

#define_import_path bevy_post_process::effect_stack::vignette

#import bevy_post_process::effect_stack::chromatic_aberration::source_texture

// See `bevy_post_process::effect_stack::Vignette` for more
// information on these fields.
struct VignetteSettings {
    intensity: f32,
    inv_radius: f32,
    smoothness: f32,
    roundness: f32,
    uv_offset: vec2<f32>,
    uv_scale: vec2<f32>,
    color: vec4<f32>
}

const VISUAL_THRESHOLD: f32 = 1e-4;

// The settings supplied by the developer.
@group(0) @binding(4) var<uniform> vignette_settings: VignetteSettings;

fn vignette(uv: vec2<f32>, color: vec3<f32>) -> vec3<f32> {
    let intensity = vignette_settings.intensity;
    if (intensity < VISUAL_THRESHOLD) {
        return color;
    }

    let centered_uv = uv - 0.5;
    let uv_from_center = centered_uv - vignette_settings.uv_offset;
    let final_uv = uv_from_center * vignette_settings.uv_scale;

    // Calculate distance from center.
    let dist = length(final_uv) * vignette_settings.inv_radius;

    // Create a smooth radial gradient: 1.0 at center, fading to 0.0 at the edges
    let base_curve = 1.0 - (dist * dist);
    let clamped_factor = clamp(base_curve, 0.0, 1.0);
    let factor = pow(clamped_factor, vignette_settings.smoothness);

    return mix(color, vignette_settings.color.rgb, (1.0 - factor) * intensity);
}
