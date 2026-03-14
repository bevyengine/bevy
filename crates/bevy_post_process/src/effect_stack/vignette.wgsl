// The vignette postprocessing effect.

#define_import_path bevy_post_process::effect_stack::vignette

#import bevy_post_process::effect_stack::chromatic_aberration::source_texture

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

const EPSILON: f32 = 1.19209290e-07;

// The settings supplied by the developer.
@group(0) @binding(5) var<uniform> vignette_settings: VignetteSettings;

fn vignette(uv: vec2<f32>, color: vec3<f32>) -> vec3<f32> {
    if (vignette_settings.intensity < EPSILON) {
        return color;
    }

    let intensity = saturate(vignette_settings.intensity);
    let radius = max(vignette_settings.radius, 0.0);
    let smoothness = max(vignette_settings.smoothness, 0.0);
    let roundness = clamp(vignette_settings.roundness, EPSILON, 2.0-EPSILON);
    let edge_comp = saturate(vignette_settings.edge_compensation);

    // Get the screen resolution.
    let dims = textureDimensions(source_texture);
    let resolution = vec2<f32>(dims.xy);
    let screen_aspect = resolution.x / resolution.y;

    // Calculate the aspect ratio.
    //
    // We divide by the smallest dimension to normalize the scale.
    // This will be used later to force the vignette to be circular, not oval.
    let aspect_ratio = resolution / min(resolution.x, resolution.y);

    // Center the UV coordinates at (0,0).
    let centered_uv = uv - 0.5;

    // Calculate the normalized offset from the center.
    //
    // (vignette_settings.center - 0.5) maps the 0.0-1.0 input to -0.5-0.5.
    // Multiplying by (1.0, y/x) compensates for the screen's aspect ratio.
    // This ensures that a movement of 0.1 looks the same distance horizontally and vertically.
    let offset = (vignette_settings.center - 0.5) * vec2<f32>(1.0, resolution.y / resolution.x);

    let uv_from_center = centered_uv - offset;
    var scale_vec = aspect_ratio * vec2<f32>(1.0, 1.0 / roundness);

    // Apply edge compensation to make the vignette fit the screen better.
    if (screen_aspect >= 1.0) {
        let compensation_factor = mix(1.0, 1.0 / screen_aspect, edge_comp);
        scale_vec.x *= compensation_factor;
    } else {
        let compensation_factor = mix(1.0, screen_aspect, edge_comp);
        scale_vec.y *= compensation_factor;
    }

    let final_uv = uv_from_center * scale_vec;

    // Calculate distance from center.
    let dist = length(final_uv) * (1.0 / radius);
    let base_curve = 1.0 - (dist * dist);
    let clamped_factor = clamp(base_curve, 0.0, 1.0);
    let factor = pow(clamped_factor, smoothness);

    // Blend the original color with the vignette color.
    return mix(color, vignette_settings.color.rgb, (1.0 - factor) * intensity);
}
