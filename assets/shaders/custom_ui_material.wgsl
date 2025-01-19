// Draws a progress bar with properties defined in CustomUiMaterial
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> color: vec4<f32>;
@group(1) @binding(1) var<uniform> slider: f32;
@group(1) @binding(2) var material_color_texture: texture_2d<f32>;
@group(1) @binding(3) var material_color_sampler: sampler;
@group(1) @binding(4) var<uniform> border_color: vec4<f32>;


@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // normalized position relative to the center of the UI node
    let r = in.uv - 0.5;

    // normalized size of the border closest to the current position
    let b = vec2(
        select(in.border_widths.x, in.border_widths.y, 0. < r.x),
        select(in.border_widths.z, in.border_widths.w, 0. < r.y)
    );

    // if the distance to the edge from the current position on any axis 
    // is less than the border width on that axis then the position is within 
    // the border and we return the border color
    if any(0.5 - b < abs(r)) {
        return border_color;
    }

    // sample the texture at this position if it's to the left of the slider value
    // otherwise return a fully transparent color
    if in.uv.x < slider {
        let output_color = textureSample(material_color_texture, material_color_sampler, in.uv) * color;
        return output_color;
    } else {
        return vec4(0.0);
    }
}
