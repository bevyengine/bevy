// Draws a progress bar with properties defined in CustomUiMaterial
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> color: vec4<f32>;
@group(1) @binding(1) var<uniform> slider: vec4<f32>;
@group(1) @binding(2) var material_color_texture: texture_2d<f32>;
@group(1) @binding(3) var material_color_sampler: sampler;
@group(1) @binding(4) var<uniform> border_color: vec4<f32>;


@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let output_color = textureSample(material_color_texture, material_color_sampler, in.uv) * color;

    // half size of the UI node
    let half_size = 0.5 * in.size;

    // position relative to the center of the UI node
    let p = in.uv * in.size - half_size;

    // thickness of the border closest to the current position
    let b = vec2(
        select(in.border_widths.x, in.border_widths.z, 0. < p.x),
        select(in.border_widths.y, in.border_widths.w, 0. < p.y)
    );

    // select radius for the nearest corner
    let rs = select(in.border_radius.xy, in.border_radius.wz, 0.0 < p.y);
    let radius = select(rs.x, rs.y, 0.0 < p.x);

    // distance along each axis from the corner
    let d = half_size - abs(p);

    // if the distance to the edge from the current position on any axis 
    // is less than the border width on that axis then the position is within 
    // the border and we return the border color
    if d.x < b.x || d.y < b.y {
        // select radius for the nearest corner
        let rs = select(in.border_radius.xy, in.border_radius.wz, 0.0 < p.y);
        let radius = select(rs.x, rs.y, 0.0 < p.x);

        // determine if the point is inside the curved corner and return the corresponding color
        let q = radius - d;
        if radius < min(max(q.x, q.y), 0.0) + length(vec2(max(q.x, 0.0), max(q.y, 0.0))) {
            return vec4(0.0);
        } else {
            return border_color;
        }
    }

    // sample the texture at this position if it's to the left of the slider value
    // otherwise return a fully transparent color
    if in.uv.x < slider.x {
        let output_color = textureSample(material_color_texture, material_color_sampler, in.uv) * color;
        return output_color;
    } else {
        return vec4(0.0);
    }
}
