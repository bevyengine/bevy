// This shader draws a circular progress bar
#import bevy_render::view View

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
};

struct CustomUiMaterial {
    @location(0) fill_amount: f32,
    @location(1) color: vec4<f32>
}

@group(0) @binding(0)
var<uniform> view: View;
@group(1) @binding(0)
var<uniform> input: CustomUiMaterial;

// How smooth the border of the gradient should be
const gradient_ease: f32 = 25.0;
// the width of the gradient
const width = 0.25;
const PI = 3.141592656;
const TAU = 6.283185312;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let fill_amount = input.fill_amount;
    let fill_angle = fill_amount * TAU;
    let uv = in.uv * 2.0 - 1.0;
    var color = vec4<f32>(0.0);
    if (atan2(uv.y, uv.x) + PI < fill_angle) {
        var inner_width = 1.0 - width;
        inner_width *= inner_width;
        let d = uv.x * uv.x + uv.y * uv.y;
        if (d <= 1.0 && d >= inner_width) {
            var w: f32 = abs((1.0 + inner_width) / 2.0 - d) / (1.0 - inner_width);
            w = 1.0 - pow(w + 0.5, gradient_ease);
            color = vec4<f32>(input.color.rgb, min(1.0, w));
        } else {
            color.a = 0.0;
        }
    } else {
        color.a = 0.0;
    }
    return color;
}
