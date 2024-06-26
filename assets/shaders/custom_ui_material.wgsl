// This shader draws a circle with a given input color
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> color: vec4<f32>;
@group(1) @binding(1) var<uniform> slider: f32;
@group(1) @binding(2) var material_color_texture: texture_2d<f32>;
@group(1) @binding(3) var material_color_sampler: sampler;


@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    if in.uv.x < slider {
        let output_rgb = textureSample(material_color_texture, material_color_sampler, in.uv).rgb * color.rgb;
        return vec4(output_rgb, 1.0);
    } else {
        return vec4(0.0);
    }
}
