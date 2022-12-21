// numbers from https://www.shadertoy.com/view/lt3GRj

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

struct Diagnostics {
    fps: f32,
};
@group(0) @binding(0)
var<uniform> diagnostics: Diagnostics;

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    switch (in_vertex_index) {
        case 0u: { out.clip_position = vec4<f32>(-1.0, 3.0, 0.0, 1.0); }
        case 1u: { out.clip_position = vec4<f32>(-1.0, -1.0, 0.0, 1.0); }
        case 2u: { out.clip_position = vec4<f32>(3.0, -1.0, 0.0, 1.0); }
        default: {}
    }
    return out;
}

fn digit_bin(x: i32) -> f32 {
    switch (x) {
        case 0: { return 480599.0; }
        case 1: { return 139810.0; }
        case 2: { return 476951.0; }
        case 3: { return 476999.0; }
        case 4: { return 350020.0; }
        case 5: { return 464711.0; }
        case 6: { return 464727.0; }
        case 7: { return 476228.0; }
        case 8: { return 481111.0; }
        case 9: { return 481095.0; }
        default: { return 0.0; }
    }
}

fn print_value(
    frag_coord: vec2<f32>,
    starting_at: vec2<f32>,
    font_size: vec2<f32>,
    value: f32,
    digits: f32,
    decimals: f32
) -> f32 {
    let char_coord: vec2<f32> = (frag_coord * vec2<f32>(1.0, -1.0) - starting_at * vec2<f32>(1.0, -1.0) + font_size) / font_size;
    if (char_coord.y < 0.0 || char_coord.y >= 1.0) {
        return 0.0;
    }
    var bits: f32 = 0.0;
    let digit_index_1: f32 = digits - floor(char_coord.x) + 1.0;
    if (-digit_index_1 <= decimals) {
        let pow_1: f32 = pow(10.0, digit_index_1);
        let abs_value: f32 = abs(value);
        let pivot: f32 = max(abs_value, 1.5) * 10.0;
        if (pivot < pow_1) {
            if (value < 0.0 && pivot >= pow_1 * 0.1) {
                bits = 1792.0;
            }
        } else if (digit_index_1 == 0.0) {
            if (decimals > 0.0) {
                bits = 2.0;
            }
        } else {
            var value_2: f32;
            if (digit_index_1 < 0.0) {
                value_2 = fract(abs_value);
            } else {
                value_2 = abs_value * 10.0;
            }
            bits = digit_bin(i32((value_2 / pow_1) % 10.0));
        }
    }
    return floor((bits / pow(2.0, floor(fract(char_coord.x) * 4.0) + floor(char_coord.y * 5.0) * 4.0)) % 2.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let clear: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    let background: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.4);
    let fps_color: vec4<f32> = vec4<f32>(0.0, 1.0, 0.0, 1.0);

    let font_size: vec2<f32> = vec2<f32>(4.0, 5.0) * 10.0;
    let margin: vec2<f32> = vec2<f32>(20.0, 25.0);

    let is_fps_digit: f32 = print_value(
        in.clip_position.xy,
        margin,
        font_size,
        diagnostics.fps,
        3.0,
        1.0
    );

    var color: vec4<f32> = clear;
    if (in.clip_position.x < font_size.x * 5.0 + margin.x * 2.0
        && in.clip_position.y < font_size.y + margin.y * 2.0) {
        color = background;
    }
    color = mix(color, fps_color, is_fps_digit);

    return color;
}
