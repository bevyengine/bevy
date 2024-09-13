// This shader draws a circle with a given input color
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> color: vec4<f32>;
@group(1) @binding(1) var<uniform> slider: f32;
@group(1) @binding(2) var material_color_texture: texture_2d<f32>;
@group(1) @binding(3) var material_color_sampler: sampler;
@group(1) @binding(4) var<uniform> border_color: vec4<f32>;
@group(1) @binding(5) var<uniform> corner_color: vec4<f32>;


@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let point = (in.uv - 0.5) * in.size;

    let b = vec2(
        select(in.border_widths.x, in.border_widths.y, point.x < 0.),
        select(in.border_widths.z, in.border_widths.w, point.y < 0.)
    );

    let rx = select(in.border_radius.xw, in.border_radius.yz, point.x < 0.);
    let r = select(rx.x, rx.y, point.y < 0.);

    let d = 0.5 * in.size - abs(point);

    if any(d < b) {
        if all(d < vec2(r)) {
            return corner_color;
        } else {
            return border_color;
        }
    }



    if in.uv.x < slider {
        let output_color = textureSample(material_color_texture, material_color_sampler, in.uv) * color;
        return output_color;
    } else {
        return vec4(0.0);
    }
}
