#import bevy_pbr::prepass_utils
#import bevy_pbr::utils
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::globals::Globals

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
    samples: u32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: vec2<f32>
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
    let texture_size = vec2<f32>(textureDimensions(screen_texture));
    let frag_coords = vec2<i32>(in.uv * texture_size);

#ifdef MULTISAMPLED
    let base_color = textureLoad(screen_texture, frag_coords, i32(sample_index));
#else
    let base_color = textureSample(screen_texture, texture_sampler, in.uv);
#endif

    let shutter_angle = settings.shutter_angle;

#ifdef MULTISAMPLED
    let this_motion_vector = textureLoad(motion_vectors, frag_coords, i32(sample_index)).rg;
#else
    let this_motion_vector = textureSample(motion_vectors, texture_sampler, in.uv).rg;
#endif

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
    
    // The exposure vector is the distance that this fragment moved while the camera shutter was
    // open. This is the motion vector (total distance traveled) multiplied by the shutter angle (a
    // fraction). In film, the shutter angle is commonly 0.5 or "180 degrees" (out of 360 total).
    // This means that for a frame time of 20ms, the shutter is only open for 10ms.
    //
    // Using a shutter angle larger than 1.0 is non-physical, objects would need to move further
    // than they physically travelled during a frame, which is not possible. Note: we allow values
    // larger than 1.0 because it may be desired for artistic reasons.
    let exposure_vector = shutter_angle * this_motion_vector;

    var accumulator: vec4<f32>;
    var weight_total = 0.0;
    let n_samples = i32(settings.samples);
    let noise = utils::interleaved_gradient_noise(vec2<f32>(frag_coords), globals.frame_count); // 0 to 1
       
    for (var i = -n_samples; i < n_samples; i++) {
        // The current sample step vector, from in.uv
        let step_vector = 0.5 * exposure_vector * (f32(i) + noise) / f32(n_samples);
        var sample_uv = in.uv + step_vector;

        // If the sample is off screen, skip it.
        if sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0 {
            continue;
        }

        let sample_coords = vec2<i32>(sample_uv * texture_size);

    #ifdef MULTISAMPLED
        let sample_color = textureLoad(screen_texture, sample_coords, i32(sample_index));
    #else
        let sample_color = textureSample(screen_texture, texture_sampler, sample_uv);
    #endif
    #ifdef MULTISAMPLED
        let sample_motion = textureLoad(motion_vectors, sample_coords, i32(sample_index)).rg;
    #else
        let sample_motion = textureSample(motion_vectors, texture_sampler, sample_uv).rg;
    #endif
    #ifdef NO_DEPTH_TEXTURE_SUPPORT
        let sample_depth = 0.0;
    #else
    #ifdef MULTISAMPLED
        let sample_depth = textureLoad(depth, sample_coords, i32(sample_index));
    #else
        let sample_depth = textureSample(depth, texture_sampler, sample_uv);
    #endif
    #endif

        var weight = 1.0;
        let is_sample_in_fg = !(depth_supported && sample_depth < this_depth && sample_depth > 0.0);
        // If the depth is 0.0, this fragment has no depth written to it and we assume it is in the
        // background. This ensures that things like skyboxes, which do not write to depth, are
        // correctly sampled in motion blur.
        if sample_depth != 0.0 && is_sample_in_fg {
            // The following weight calculation is used to eliminate ghosting artifacts that are
            // common in motion-vector-based motion blur implementations. While some resources
            // recommend using depth, I've found that sampling the velocity results in significantly
            // better results. Unlike a depth heuristic, this is not scale dependent.
            //
            // The most distracting artifacts occur when a stationary foreground object is
            // incorrectly sampled while blurring a moving background object, causing the stationary
            // object to blur when it should be sharp ("background bleeding"). This is most obvious
            // when the camera is tracking a fast moving object. The tracked object should be sharp,
            // and should not bleed into the motion blurred background.
            //
            // To attenuate these incorrect samples, we compare the motion of the fragment being
            // blurred to the UV being sampled, to answer the question "is it possible that this
            // sample was occluding the fragment?"
            //
            // Note to future maintainers: proceed with caution when making any changes here, and
            // ensure you check all occlusion/disocclusion scenarios and fullscreen camera rotation
            // blur for regressions.
            let frag_speed = length(step_vector);
            let sample_speed = length(sample_motion) / 2.0; // Halved because the sample is centered
            let cos_angle = dot(step_vector, sample_motion) / (frag_speed * sample_speed * 2.0);
            let motion_similarity = clamp(abs(cos_angle), 0.0, 1.0);
            if sample_speed * motion_similarity < frag_speed {
                // Project the sample's motion onto the frag's motion vector. If the sample did not
                // cover enough distance to reach the original frag, there is no way it could have
                // influenced this frag at all, and should be discarded.
                weight = 0.0;
            }
        }
        weight_total += weight;
        accumulator += weight * sample_color;
    }

    let has_moved_less_than_a_pixel = 
        dot(this_motion_vector * texture_size, this_motion_vector * texture_size) < 1.0;
    // In case no samples were accepted, fall back to base color.
    // We also fall back if motion is small, to not break antialiasing.
    if weight_total <= 0.0 || has_moved_less_than_a_pixel {
        accumulator = base_color;
        weight_total = 1.0;
    }
    return accumulator / weight_total;
}