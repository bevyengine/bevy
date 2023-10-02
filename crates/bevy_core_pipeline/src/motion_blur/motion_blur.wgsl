#import bevy_pbr::prepass_utils
#import bevy_pbr::utils
#import bevy_core_pipeline::fullscreen_vertex_shader FullscreenVertexOutput
#import bevy_render::globals Globals

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
struct MotionBlur {
    shutter_angle: f32,
    max_samples: u32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: vec3<f32>
#endif
}
@group(0) @binding(4) var<uniform> settings: MotionBlur;
@group(0) @binding(5) var<uniform> globals: Globals;

@fragment
fn fragment(
    #ifdef MULTISAMPLED
        @builtin(sample_index) sample_index: u32,
    #endif
    in: FullscreenVertexOutput
) -> @location(0) vec4<f32> { 
    let shutter_angle = settings.shutter_angle;
    let texture_size = vec2<f32>(textureDimensions(screen_texture));
    let frag_coords = vec2<i32>(in.uv * texture_size);

#ifdef MULTISAMPLED
    let this_motion_vector = textureLoad(motion_vectors, frag_coords, i32(sample_index)).rg;
#else
    let this_motion_vector = textureSample(motion_vectors, texture_sampler, in.uv).rg;
#endif
    
    let exposure_vector = shutter_angle * this_motion_vector;
    let speed = length(exposure_vector * texture_size);

    // Number of samples should always be odd so we can gaurantee symmetric sampling. This also
    // ensures we always have at least one sample, the central sample, which will be unjittered.
    let n_samples = i32(clamp(speed, 2.0, f32(settings.max_samples)) / 2.0) * 2 + 1;

#ifdef NO_DEPTH_TEXTURE_SUPPORT
    let this_depth = 0.0;
    let depth_supported = false;
#else
    let depth_supported = true;
#ifdef MULTISAMPLED
    let this_depth = textureLoad(depth, frag_coords, i32(sample_index));
#else
    let this_depth = textureSample(depth, texture_sampler, in.uv);
#endif
#endif

    var weight_total = 0.0;
    var accumulator = vec4<f32>(0.0);
    let noise = hash_noise(frag_coords, globals.frame_count) * 2.0 - 1.0;
    let jitter = noise / f32(n_samples - 1);

    for (var i = 0; i < n_samples; i++) {
        let sample_percent = f32(i) / f32(n_samples - 1) - 0.5;

        var offset = vec2<f32>(0.0);
        if n_samples > 1 && abs(sample_percent) > 0.001 {
            // We want the central sample to have an offset of zero.
            offset = exposure_vector * (sample_percent + jitter);
        }
        let sample_uv = in.uv + offset;
        let sample_coords = vec2<i32>(sample_uv * texture_size);

        // If depth is not considered during sampling, you can end up sampling objects in front of a
        // fast moving object, which will cause the (possibly stationary) objects in front of that
        // fast moving object to smear. To prevent this, we check the depth and velocity of the
        // fragment we are sampling.
    #ifdef NO_DEPTH_TEXTURE_SUPPORT
        let sample_depth = 0.0;
    #else
    #ifdef MULTISAMPLED
        let sample_depth = textureLoad(depth, sample_coords, i32(sample_index));
    #else
        let sample_depth = textureSample(depth, texture_sampler, sample_uv);
    #endif
    #endif
        
    #ifdef MULTISAMPLED
        let sample_motion_vector = textureLoad(motion_vectors, sample_coords, i32(sample_index)).rg;
    #else
        let sample_motion_vector = textureSample(motion_vectors, texture_sampler, sample_uv).rg;
    #endif

        var weight = 1.0;
        // if the sampled frag is in front of this frag, we want to scale its weight by how parallel
        // their motion vectors are. This is because that means the sampled fragment is more likely
        // to have occupied this fragment during the course of its motion.
        if sample_depth > this_depth || !depth_supported {
            let this_len = length(this_motion_vector);
            let sample_len = length(sample_motion_vector);
            let cos_angle = dot(this_motion_vector, sample_motion_vector) / (this_len * sample_len);
            let motion_similarity = clamp(abs(cos_angle), 0.0, 1.0);
            // If the foreground sampled frag is not moving much, we definitely shouldn't sample it,
            // because there is no way that it could've contributed to this fragment's color.
            let length_ratio = clamp(sample_len / this_len, 0.0, 1.0);
            // Varies from 0 to 1, meaning the strength is only attenueted, never boosted.
            weight = pow(motion_similarity * length_ratio, 2.0);
        }
        weight_total += weight;
        accumulator += weight * textureSample(screen_texture, texture_sampler, sample_uv);
    }

    return accumulator / weight_total;
}

fn uhash(a: u32, b: u32) -> u32 { 
    var x = ((a * 1597334673u) ^ (b * 3812015801u));
    // from https://nullprogram.com/blog/2018/07/31/
    x = x ^ (x >> 16u);
    x = x * 0x7feb352du;
    x = x ^ (x >> 15u);
    x = x * 0x846ca68bu;
    x = x ^ (x >> 16u);
    return x;
}

fn unormf(n: u32) -> f32 { 
    return f32(n) * (1.0 / f32(0xffffffffu)); 
}

fn hash_noise(ifrag_coord: vec2<i32>, frame: u32) -> f32 {
    let urnd = uhash(u32(ifrag_coord.x), (u32(ifrag_coord.y) << 11u) + frame);
    return unormf(urnd);
}