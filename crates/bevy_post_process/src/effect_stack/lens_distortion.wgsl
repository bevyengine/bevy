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

const EPSILON: f32 = 1.19209290e-07;

// The settings supplied by the developer.
@group(0) @binding(6) var<uniform> lens_distortion_settings: LensDistortionSettings;

fn lens_distortion(uv: vec2<f32>) -> vec2<f32>{
    let intensity = lens_distortion_settings.intensity;
    if (abs(intensity) < EPSILON) {
        return uv;
    }
    let multiplier = lens_distortion_settings.multiplier;
    let center = lens_distortion_settings.center;

    let uv_centered = uv - center;
    // Prevent division by zero.
    let radius = max(length(uv_centered), EPSILON);

    let direction = uv_centered / radius;
    let weight_x = abs(direction.x);
    let weight_y = abs(direction.y);
    let adjust = weight_x * multiplier.x + weight_y * multiplier.y;

    // Maintains the correlation between k2 and k1, while ensuring the sign of k2
    // is determinedsolely by `edge_curvature` rather than being influenced by intensity.
    let k1 = intensity * adjust;
    let k2 = k1 * intensity * lens_distortion_settings.edge_curvature;

    // Simplify version: r' = r(1 + k1*r^2 + k2*r^4)
    //
    // k1 dominates the overall distortion shape, while k2 provides subtle
    // edge refinement.Using only k1 and k2 offers an optimal balance between
    // visual fidelity and performance for real-time rendering, making higher-order
    // terms (k3) generally unnecessary.
    let r2 = radius*radius;
    let r_distorted = radius * (1.0 + k1 * r2 + k2 * r2 * r2);

    let uv_distorted = direction * r_distorted + center;
    let uv_scaled = (uv_distorted - center) / lens_distortion_settings.scale + center;

    let uv_safe = clamp(uv_scaled, vec2<f32>(0.0), vec2<f32>(1.0));

    return uv_safe;
}
