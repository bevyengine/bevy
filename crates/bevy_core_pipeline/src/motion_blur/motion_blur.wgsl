#import bevy_pbr::prepass_utils
#import bevy_pbr::utils
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::globals::Globals

#ifdef MULTISAMPLED
@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var motion_vectors: texture_multisampled_2d<f32>;
#else
@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var motion_vectors: texture_2d<f32>;
#endif
@group(0) @binding(2) var texture_sampler: sampler;
struct MotionBlur {
    shutter_angle: f32,
    max_samples: u32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: vec3<f32>
#endif
}
@group(0) @binding(3) var<uniform> settings: MotionBlur;
@group(0) @binding(4) var<uniform> globals: Globals;

@fragment
fn fragment(
    #ifdef MULTISAMPLED
        @builtin(sample_index) sample_index: u32,
    #endif
    in: FullscreenVertexOutput
) -> @location(0) vec4<f32> { 
    let base_color = textureSample(screen_texture, texture_sampler, in.uv);;
    if i32(settings.max_samples) < 2 || settings.shutter_angle <= 0.0 {
        return base_color;
    }

    let shutter_angle = settings.shutter_angle;
    let texture_size = vec2<f32>(textureDimensions(screen_texture));
    let frag_coords = vec2<i32>(in.uv * texture_size);

#ifdef MULTISAMPLED
    let this_motion_vector = textureLoad(motion_vectors, frag_coords, i32(sample_index)).rg;
#else
    let this_motion_vector = textureSample(motion_vectors, texture_sampler, in.uv).rg;
#endif
    
    // The exposure vector is the distance that this fragment moved while the camera shutter was
    // open. This is the motion vector (total distance traveled) multiplied by the shutter angle (a
    // fraction). In film, the shutter angle is commonly 0.5 or "180 degrees" (out of 360 total).
    // This means that for a frame time of 20ms, the shutter is only open for 10ms.
    //
    // Using a shutter angle larger than 1.0 is non-physical, objects would need to move further
    // than they physically travelled during a frame, which is not possible.
    let exposure_vector = shutter_angle * this_motion_vector;

    var accumulator: vec4<f32>;
    var weight_total = 0.0;
    let n_samples_half = i32(settings.max_samples) / 2;
    let noise = hash_noise(frag_coords, globals.frame_count); // 0 to 1
       
    for (var i = -n_samples_half; i < n_samples_half; i++) {
        // The current sample step vector, from in.uv
        let step_vector = 0.5 * exposure_vector * (f32(i) + noise) / f32(n_samples_half);
        var sample_uv = in.uv + step_vector;
        let sample_coords = vec2<i32>(sample_uv * texture_size);

    #ifdef MULTISAMPLED
        let sample_motion = textureLoad(motion_vectors, sample_coords, i32(sample_index)).rg;
    #else
        let sample_motion = textureSample(motion_vectors, texture_sampler, sample_uv).rg;
    #endif

        // The following weight calculation is used to eliminate ghosting artifacts that are common
        // in motion-vector-based motion blur implementations. While some resources recommend using
        // depth, I've found that sampling the velocity results in significantly better results.
        // Unlike a depth heuristic, this is not scale dependent.
        //
        // The most distracting artifacts occur when a stationary object is incorrectly sampled
        // while blurring a moving object, causing the stationary object to blur when it should be
        // sharp ("background bleeding"). This is most obvious when the camera is tracking a fast
        // moving object. The tracked object should be sharp, and should not bleed into the motion
        // blurred background.
        //
        // To attenuate these incorrect samples, we compare the motion of the fragment being blurred
        // to the UV being sampled, to answer the question "is it possible that this sample was
        // occluding the fragment?"
        //
        // Note to future maintainers: 
        //
        // I have repeatedly attempted to use depth tests, however:
        //   - All occlusion experiments (foreground vs background) resulted in distracting
        //     artifacts caused by discontinuities introduced by the depth check.
        //   - Using depth to weight samples requires some hueristic that ends up being scale and
        //     distance dependant.
        //
        // Proceed with caution when making any changes here, and ensure you check all
        // oclusion/disocclusion scenarios and fullscreen camera rotation blur for regressions.
        let frag_speed = length(step_vector);
        let sample_speed = length(sample_motion / 2.0); // Halved because the sample is centered
        let cos_angle = dot(step_vector, sample_motion) / (frag_speed * sample_speed);
        // Sign is ignored because the sample is centered.
        let motion_similarity = clamp(abs(cos_angle), 0.0, 1.0);
        // Project the sample's motion onto the frag's motion vector. If the sample did not cover
        // enough distance to each the original frag, there is no way it could have influenced this
        // frag at all, and should be discarded.
        var weight = step(frag_speed, sample_speed * motion_similarity);
                
        weight_total += weight;
        accumulator += weight * textureSample(screen_texture, texture_sampler, sample_uv);
    }

    if weight_total <= 0.0 {
        accumulator = base_color;
        weight_total = 1.0;
    }

    return accumulator / weight_total;
}

// The following functions are used to generate noise. This could potentially be improved with blue
// noise in the future.

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