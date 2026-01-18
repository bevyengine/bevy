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
    unused_a: u32,
    unused_b: u32,
    color: vec4<f32>
}

// The settings supplied by the developer.
@group(0) @binding(5) var<uniform> vignette_settings: VignetteSettings;

fn vignette(uv: vec2<f32>, color: vec3<f32>) -> vec3<f32> {
    let intensity = clamp(vignette_settings.intensity, 0.0, 1.0);
    let radius = max(vignette_settings.radius, 0.0);
    let smoothness = max(vignette_settings.smoothness, 0.0);
    let roundness = max(vignette_settings.roundness, 0.001);

    // Get the screen resolution.
    let dims = textureDimensions(source_texture);
    let resolution = vec2<f32>(f32(dims.x), f32(dims.y));

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
    let final_uv = uv_from_center * aspect_ratio * vec2<f32>(1.0, 1.0 / roundness);

    // Calculate distance from center.
    let dist = length(final_uv) * 2.0;

    let inner_edge = radius;
    let outer_edge = inner_edge + smoothness;

    // Generate a 0.0 to 1.0 factor based on distance within the edges.
    let factor = smoothstep(inner_edge, outer_edge, dist);

    // Blend the original color with the vignette color.
    return mix(color, vignette_settings.color.rgb, factor * intensity);
}
