#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_vertex_output  MeshVertexOutput
#import bevy_pbr::utils               PI

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

// Sweep across hues on y axis with value from 0.0 to +15EV across x axis 
// quantized into 24 steps for both axis.
fn color_sweep(uv: vec2<f32>) -> vec3<f32> {
    var uv = uv;
    let steps = 24.0;
    uv.y = uv.y * (1.0 + 1.0 / steps);
    let ratio = 2.0;
    
    let h = PI * 2.0 * floor(1.0 + steps * uv.y) / steps;
    let L = floor(uv.x * steps * ratio) / (steps * ratio) - 0.5;
    
    var color = vec3(0.0);
    if uv.y < 1.0 { 
        color = cos(h + vec3(0.0, 1.0, 2.0) * PI * 2.0 / 3.0);
        let maxRGB = max(color.r, max(color.g, color.b));
        let minRGB = min(color.r, min(color.g, color.b));
        color = exp(15.0 * L) * (color - minRGB) / (maxRGB - minRGB);
    } else {
        color = vec3(exp(15.0 * L));
    }
    return color;
}

fn hsv_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3(0.0), vec3(1.0)), c.y);
}

// Generates a continuous sRGB sweep.
fn continuous_hue(uv: vec2<f32>) -> vec3<f32> {
    return hsv_to_srgb(vec3(uv.x, 1.0, 1.0)) * max(0.0, exp2(uv.y * 9.0) - 1.0);
}

@fragment
fn fragment(
    in: MeshVertexOutput,
) -> @location(0) vec4<f32> {
    var uv = in.uv;
    var out = vec3(0.0);
    if uv.y > 0.5 {
        uv.y = 1.0 - uv.y;
        out = color_sweep(vec2(uv.x, uv.y * 2.0));
    } else {
        out = continuous_hue(vec2(uv.y * 2.0, uv.x));
    }
    var color = vec4(out, 1.0);
#ifdef TONEMAP_IN_SHADER
    color = tone_mapping(color, bevy_pbr::mesh_view_bindings::view.color_grading);
#endif
    return color;
}
