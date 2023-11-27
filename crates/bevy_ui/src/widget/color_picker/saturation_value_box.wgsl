#import bevy_core_pipeline::tonemapping::powsafe
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_pbr::utils::{PI, hsv2rgb}

struct SaturationValueUniform {
    hue: f32,
    marker: vec2<f32>,
}

struct SaturationValueBoxMaterial {
    @location(0) values: SaturationValueUniform,
    // padding?
}

@group(1) @binding(0)
var<uniform> material: SaturationValueBoxMaterial;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // NOTE: We want "value" to increase vertically which looks most natural hence the flip
    let rgb = hsv2rgb(material.values.hue, in.uv.x, 1.-in.uv.y);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    var output_rgb = powsafe(rgb, 2.2);

    // add a marker to the selected saturation, value coordinate

    // uv in (-1., 1.) range, +x right, +y up
    let uv = (in.uv * 2. - 1.) * vec2(1., -1.);

    let dist = length(-material.values.marker+uv);

    // the radius and thickness of the white marker
    let radius = 0.03;
    let th = 0.02;

    let ring_signal = smoothstep(radius, radius+th, dist) - smoothstep(radius+th, radius+(2. * th), dist);

    output_rgb += ring_signal;

    return vec4<f32>(saturate(output_rgb), 1.);
}

