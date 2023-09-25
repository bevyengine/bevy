#import bevy_pbr::prepass_utils
#import bevy_pbr::utils
#import bevy_core_pipeline::fullscreen_vertex_shader FullscreenVertexOutput

#ifdef MULTISAMPLED
@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var motion_vectors: texture_multisampled_2d<f32>;
@group(0) @binding(2) var depth: texture_depth_multisampled_2d;
#else
@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var motion_vectors: texture_2d<f32>;
@group(0) @binding(2) var depth: texture_depth_2d;
#endif
@group(0) @binding(3) var texture_sampler: sampler;
struct PostProcessSettings {
    shutter_angle: f32,
    max_samples: u32,
    depth_bias: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: vec3<f32>
#endif
}
@group(0) @binding(4) var<uniform> settings: PostProcessSettings;

@fragment
fn fragment(
    #ifdef MULTISAMPLED
        @builtin(sample_index) sample_index: u32,
    #endif
    in: FullscreenVertexOutput
) -> @location(0) vec4<f32> {
#ifndef MULTISAMPLED
    let sample_index = 0u;
#endif    
    let shutter_angle = settings.shutter_angle;
    let texture_size = vec2<f32>(textureDimensions(screen_texture));
    let frag_coords = vec2<i32>(in.uv * texture_size);

    let motion_vector = textureLoad(motion_vectors, frag_coords, i32(sample_index)).rg;
    let exposure_vector = shutter_angle * motion_vector;
    let speed = length(exposure_vector * texture_size);
    let n_samples = i32(clamp(speed * 2.0, 1.0, f32(settings.max_samples)));

    let this_depth = textureLoad(depth, frag_coords, i32(sample_index));
    let base_color = textureSample(screen_texture, texture_sampler, in.uv);
    var weight_total = 0.0;
    var accumulator = vec4<f32>(0.0);

    for (var i = 0; i < n_samples; i++) {
        var offset = vec2<f32>(0.0);
        if speed > 1.0 && n_samples > 1 {
            offset = exposure_vector * ((f32(i) + noise(in.uv)) / f32(n_samples) - 0.5);
        }
        let sample_uv = in.uv + offset;
        let sample_coords = vec2<i32>(sample_uv * texture_size);

        // If depth is not considered during sampling, you can end up sampling objects in front of a
        // fast moving object, which will cause the (possibly stationary) objects in front of that
        // fast moving object to smear. To prevent this, we check the depth of the fragment we are
        // sampling. If it is closer to the camera than this fragment (plus the user-defined bias),
        // we discard it. If the bias is too small, fragments from the same object will be filtered
        // out.
        let sample_depth = textureLoad(depth, sample_coords, i32(sample_index));
        let weight = step(settings.depth_bias, this_depth - sample_depth);

        weight_total += weight;
        accumulator += weight * textureSample(screen_texture, texture_sampler, sample_uv);
    }

    // Avoid black pixels by falling back to the unblurred fragment color
    if weight_total == 0.0 {
        accumulator =  base_color;
        weight_total = 1.0;
    }

    return accumulator / weight_total;
}

fn noise(frag_coord: vec2<f32>) -> f32 {
    let k1 = vec2<f32>(23.14069263277926, 2.665144142690225);
    return fract(cos(dot(frag_coord, k1)) * 12345.6789);
}