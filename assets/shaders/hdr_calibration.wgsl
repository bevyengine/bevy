#import bevy_pbr::mesh_view_bindings::view
#import bevy_render::color_operations::hsv_to_rgb

const PI: f32 = 3.14159265358979323846;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(1) var screen_texture: texture_2d<f32>;
@group(0) @binding(2) var texture_sampler: sampler;
@group(0) @binding(3) var<uniform> settings: HdrCalibrationEffect;

struct HdrCalibrationEffect {
    enabled: f32,
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(vertex_index % 2u) * 2.0 - 1.0;
    let y = f32(vertex_index / 2u) * 2.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    if (settings.enabled < 0.5) {
        return textureSample(screen_texture, texture_sampler, in.uv);
    }

    let uv = in.uv;
    
    // Top half: Luminance gradient (0 to max_luminance / paper_white)
    // Bottom half: Color gradient
    
    var color: vec3<f32>;
    
    let paper_white = view.color_grading.paper_white;
    let max_luminance = view.color_grading.max_luminance;
    let max_ratio = max_luminance / paper_white;

    if (uv.y < 0.5) {
        // Top half: Grayscale luminance gradient
        // Map [0, 1] to [0, max_ratio]
        let intensity = uv.x * max_ratio;
        color = vec3<f32>(intensity);
        
        // Add some markers
        if (abs(intensity - 1.0) < 0.005) {
            color = vec3<f32>(max_ratio, 0.0, 0.0); // Red line at paper white
        }
    } else {
        // Bottom half: Hue gradient
        let hue = uv.x * 2.0 * PI;
        
        // Use standard HSV to RGB conversion for smoother gradient
        var color_hue = hsv_to_rgb(vec3<f32>(hue, 1.0, 1.0));
        
        // Add brightness steps
        let y_step = (uv.y - 0.5) * 2.0; // [0, 1]
        if (y_step > 0.75) {
            // Very bottom strip at max luminance
            color = color_hue * max_ratio;
        } else {
            // Upper part at paper white
            color = color_hue;
        }
    }
    
    return vec4<f32>(color, 1.0);
}
