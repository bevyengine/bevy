// This shader draws a circle with a given input color
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> color: vec4<f32>;
@group(1) @binding(1) var<uniform> slider: f32;
@group(1) @binding(2) var material_color_texture: texture_2d<f32>;
@group(1) @binding(3) var material_color_sampler: sampler;
@group(1) @binding(4) var<uniform> border_color: vec4<f32>;


@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let r = in.uv - 0.5;
    let b = vec2(
        select(in.border_widths.x, in.border_widths.y, r.x < 0.),
        select(in.border_widths.z, in.border_widths.w, r.y < 0.)
    );

    if any(0.5 - b < abs(r)) {
        return border_color;
    }

    if in.uv.x < slider {
        let output_color = textureSample(material_color_texture, material_color_sampler, in.uv) * color;
        return output_color;
    } else {
        return vec4(0.0);
    }
}
