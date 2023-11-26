#import bevy_core_pipeline::tonemapping::powsafe
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_pbr::utils::{PI, hsv2rgb}

struct SaturationValueBoxMaterial {
    @location(0) hue: f32,
    // padding?
}

@group(1) @binding(0)
var<uniform> material: SaturationValueBoxMaterial;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // NOTE: We want "value" to increase vertically which looks most natural hence the flip
    let rgb = hsv2rgb(material.hue, in.uv.x, 1.-in.uv.y);

    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    let output_rgb = powsafe(rgb, 2.2);
    return vec4<f32>(output_rgb, 1.);
}

