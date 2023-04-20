#define_import_path bevy_core_pipeline::debug_gradient

// Define a function to map a range of values to a high contrast color gradient
// optimized for representational clarity.
// this uses the matplotlib plasma color map.

// Color from the plasma gradient quantized to six colors, for value between
// 0 and 5, clamped.
fn plasma_color_at_index(value: f32) -> vec3<f32> {
    let plasma_colors = array(
        vec3(0.868,0.941,0.011), // #f0f921
        vec3(0.966,0.386,0.0326), // #fca636
        vec3(0.753,0.126,0.121), // #e16462
        vec3(0.444,0.0188,0.282), // #b12a90
        vec3(0.144,0.0,0.396), // #6a00a8
        vec3(0.00142,0.000488,0.245), // #0d0887
    );
    // There is no way to index an array in wgsl from a dynamic value,
    // the index (between square brackets) must be a const value.
    // So we implement array indexing as a series of if/else
    if value < 1.0 {
        return plasma_colors[0];
    } else if value < 2.0 {
        return plasma_colors[1];
    } else if value < 3.0 {
        return plasma_colors[2];
    } else if value < 4.0 {
        return plasma_colors[3];
    } else if value < 5.0 {
        return plasma_colors[4];
    } else {
        return plasma_colors[5];
    }
}

// Return the color sampled from the plasma color map,
// `value` is clamped between 0 and 1.
fn debug_gradient(value: f32) -> vec4<f32> {
    let colors_offset = saturate(value) * 5.0;
    let low_ratio = fract(colors_offset);

    let low_color_index = u32(floor(colors_offset));
    let low_color = plasma_color_at_index(colors_offset);
    let high_color = plasma_color_at_index(colors_offset + 1.0);

    return vec4(mix(low_color, high_color, low_ratio), 1.0);
}

