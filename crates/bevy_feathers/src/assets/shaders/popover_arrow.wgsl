#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> fill_color: vec4<f32>;
@group(1) @binding(1) var<uniform> border_color: vec4<f32>;
@group(1) @binding(2) var<uniform> border_width: vec4<f32>;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let size = in.size;
    let p = in.uv * size;
    let slope = 0.5 * size.x / size.y;
    let edge_distance =
        (slope * p.y - abs(p.x - 0.5 * size.x)) / sqrt(slope * slope + 1.0);
    let triangle_distance = min(edge_distance, p.y);
    let coverage = smoothstep(-0.75, 0.75, triangle_distance);
    let fill_mix = smoothstep(border_width.x - 0.5, border_width.x + 0.5, edge_distance);
    let color = mix(border_color, fill_color, fill_mix);

    return vec4<f32>(color.rgb, color.a * coverage);
}
