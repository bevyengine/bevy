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
}

// The settings supplied by the developer.
@group(0) @binding(5) var<uniform> vignette_settings: VignetteSettings;

fn vignette(uv: vec2<f32>, color: vec3<f32>) -> vec3<f32> {
    let intensity = clamp(vignette_settings.intensity, 0.0, 1.0);
    let radius = max(vignette_settings.radius, 0.0);
    let smoothness = max(vignette_settings.smoothness, 0.0);
    let roundness = max(vignette_settings.roundness, 0.001);

    // Correct for the screen aspect ratio so the vignette remains circular, not oval.
    let dims = textureDimensions(source_texture);
    let resolution = vec2<f32>(f32(dims.x), f32(dims.y));
    let aspect_ratio = resolution / min(resolution.x, resolution.y);
    // Center the UVs at (0,0) and apply aspect correction.
    let centered_uv = (uv - 0.5) * aspect_ratio;
    let final_uv = centered_uv * vec2<f32>(1.0, 1.0 / roundness);

    // Calculate distance from center.
    let dist = length(final_uv) * 2.0;

    let inner_edge = radius;
    let outer_edge = inner_edge + smoothness;

    // Generate a 0.0 to 1.0 factor based on distance within the edges.
    let factor = smoothstep(inner_edge, outer_edge, dist);

    return mix(color, vec3<f32>(0.0), factor * intensity);
}
