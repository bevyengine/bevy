#import bevy_core_pipeline::tonemapping::powsafe
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_pbr::utils::{PI, hsv2rgb}

struct HueUniform {
    hue: f32,
    inner_radius: f32
}

struct HueWheelMaterial {
    @location(0) values: HueUniform
    // padding?
}

@group(1) @binding(0)
var<uniform> material: HueWheelMaterial;


@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // remap to (-1, 1) range
    let uv = in.uv * 2.0 - 1.0;
    let wh_dist = length(uv);

    // normalized hue angle
    let hue = (atan2(uv.y, uv.x) + PI) / (2. * PI);
    let rgb = hsv2rgb(hue, 1., 1.);

    // make ring signal for wheel alpha
    let wh_radius = material.values.inner_radius;
    let wh_slope = 0.017;
    let wh_w = 0.13;

    let wheel_signal = smoothstep(wh_radius, wh_radius+wh_slope, wh_dist)-smoothstep(wh_radius+wh_w, wh_radius+wh_w+wh_slope, wh_dist);

    // make ring signal for white marker on wheel
    let m_radius = 0.025;
    let m_slope = 0.008;
    let m_w = 0.012;

    // hue angle in radians
    let hue_rad = material.values.hue * 2. * PI + PI;
    let m_pos = (wh_radius + wh_w * 0.55) * vec2(cos(hue_rad), sin(hue_rad));
    let m_dist = length(-uv + m_pos);

    let marker_signal = smoothstep(m_radius, m_radius+m_slope, m_dist)-smoothstep(m_radius+m_w, m_radius+m_w+m_slope, m_dist);

    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    var output_rgb = powsafe(rgb, 2.2);

    output_rgb += marker_signal;
    return vec4<f32>(saturate(output_rgb), wheel_signal);
}

