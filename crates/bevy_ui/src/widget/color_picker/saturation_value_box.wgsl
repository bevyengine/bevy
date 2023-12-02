#import bevy_core_pipeline::tonemapping::powsafe
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_pbr::utils::{PI, hsv2rgb}

struct SaturationValueUniform {
    hue: f32,
    saturation: f32,
    value: f32,
}

struct SaturationValueMaterial {
    @location(0) values: SaturationValueUniform,
    // padding?
}

@group(1) @binding(0)
var<uniform> material: SaturationValueMaterial;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // NOTE: We want "value" to increase vertically which looks most natural hence the flip
    let uvflip = vec2(in.uv.x, 1. - in.uv.y);

    let rgb = hsv2rgb(material.values.hue, uvflip.x, uvflip.y);

    // add a marker to the selected saturation, value coordinate
    let dist = length(-uvflip + vec2(material.values.saturation, material.values.value));

    // the radius and thickness of the white marker
    let radius = 0.02;
    let th = 0.01;

    let marker_signal = smoothstep(radius, radius+th, dist) - smoothstep(radius+th, radius+(2. * th), dist);

    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    var output_rgb = powsafe(rgb, 2.2);

    output_rgb += marker_signal;
    return vec4<f32>(saturate(output_rgb), 1.);
}

