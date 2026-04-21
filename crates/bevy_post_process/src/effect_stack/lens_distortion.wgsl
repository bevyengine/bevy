// The lens distortion postprocessing effect.

#define_import_path bevy_post_process::effect_stack::lens_distortion

// See `bevy_post_process::effect_stack::LensDistortion` for more
// information on these fields.
struct LensDistortionSettings {
    intensity: f32,
    scale: f32,
    multiplier: vec2<f32>,
    center: vec2<f32>,
    edge_curvature: f32,
    unused: u32,
}

const VISUAL_THRESHOLD: f32 = 1e-4;
const MATH_EPSILON: f32 = 1e-6;

// The settings supplied by the developer.
@group(0) @binding(6) var<uniform> lens_distortion_settings: LensDistortionSettings;

fn lens_distortion(uv: vec2<f32>) -> vec2<f32>{
    let intensity = lens_distortion_settings.intensity;
    if (abs(intensity) < VISUAL_THRESHOLD) {
        return uv;
    }
    let multiplier = lens_distortion_settings.multiplier;
    let center = lens_distortion_settings.center;

    let uv_centered = uv - center;
    // Prevent division by zero.
    let radius = max(length(uv_centered), MATH_EPSILON);

    let direction = uv_centered / radius;
    let adjust = dot(abs(direction), multiplier);

    // Maintains the correlation between k2 and k1, while ensuring the sign of k2
    // is determined solely by `edge_curvature` rather than being influenced by intensity.
    let k1 = intensity * adjust;
    let k2 = k1 * intensity * lens_distortion_settings.edge_curvature;

    let r2 = radius * radius;
    let r_distorted = radius * (1.0 + (k1 + k2 * r2) * r2);

    let uv_distorted = direction * r_distorted + center;

    // Compensates for the distortion pushing pixels outside the [0,1] UV bounds.
    let uv_scaled = (uv_distorted - center) / lens_distortion_settings.scale + center;

    // Discard out-of-bounds pixels to prevent edge bleeding artifacts.
    let uv_safe = clamp(uv_scaled, vec2<f32>(0.0), vec2<f32>(1.0));

    return uv_safe;
}
