#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<storage> values: array<f32>;
struct Config {
    dt_min: f32,
    dt_max: f32,
    dt_min_log2: f32,
    dt_max_log2: f32,
    proportional_width: u32,
    min_color: vec4<f32>,
    max_color: vec4<f32>,
}
@group(1) @binding(1) var<uniform> config: Config;

// Gets a color based on the delta time
// TODO use customizable gradient
fn color_from_dt(dt: f32) -> vec4<f32> {
    return mix(config.min_color, config.max_color, dt / config.dt_max);
}

// Draw an SDF square
fn sdf_square(pos: vec2<f32>, half_size: vec2<f32>, offset: vec2<f32>) -> f32 {
    let p = pos - offset;
    let dist = abs(p) - half_size;
    let outside_dist = length(max(dist, vec2<f32>(0.0, 0.0)));
    let inside_dist = min(max(dist.x, dist.y), 0.0);
    return outside_dist + inside_dist;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let dt_min = config.dt_min;
    let dt_max = config.dt_max;
    let dt_min_log2 = config.dt_min_log2;
    let dt_max_log2 = config.dt_max_log2;

    // The general algorithm is highly inspired by
    // <https://asawicki.info/news_1758_an_idea_for_visualization_of_frame_times>

    let len = arrayLength(&values);
    var graph_width = 0.0;
    for (var i = 0u; i <= len; i += 1u) {
        let dt = values[len - i];

        var frame_width: f32;
        if config.proportional_width == 1u {
            frame_width = (dt / dt_min) / f32(len);
        } else {
            frame_width = 0.015;
        }

        let frame_height_factor = (log2(dt) - dt_min_log2) / (dt_max_log2 - dt_min_log2);
        let frame_height_factor_norm = min(max(0.0, frame_height_factor), 1.0);
        let frame_height = mix(0.0, 1.0, frame_height_factor_norm);

        let size = vec2(frame_width, frame_height) / 2.0;
        let offset = vec2(1.0 - graph_width - size.x, 1. - size.y);
        if (sdf_square(in.uv, size, offset) < 0.0) {
            return color_from_dt(dt);
        }

        graph_width += frame_width;
    }

    return vec4(0.0, 0.0, 0.0, 0.5);
}

