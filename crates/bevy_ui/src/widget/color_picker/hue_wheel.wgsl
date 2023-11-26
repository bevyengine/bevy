#import bevy_core_pipeline::tonemapping::powsafe
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_pbr::utils::{PI, hsv2rgb}

struct HueWheelMaterial {
    @location(0) inner_radius: f32
    // padding?
}

@group(1) @binding(0)
var<uniform> material: HueWheelMaterial;


@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // remap to (-1, 1) range
    let uv = in.uv * 2.0 - 1.0;
    let d = length(uv);

    // circle with smoothed edge
    let alpha = 1.-pow(d, 100.0);
    // cut out inner part
    let cutout = step(material.inner_radius, d);

    // normalized hue angle
    let hue = (atan2(uv.y, uv.x) + PI) / (2. * PI);
    let rgb = hsv2rgb(hue, 1., 1.);

    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    let output_rgb = powsafe(rgb, 2.2);
    return vec4<f32>(output_rgb, alpha * cutout);
}

