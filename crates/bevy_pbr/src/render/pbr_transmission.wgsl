#define_import_path bevy_pbr::transmission

#import bevy_pbr::{
    lighting,
    prepass_utils,
    utils::interleaved_gradient_noise,
    utils,
    mesh_view_bindings as view_bindings,
};

#import bevy_render::maths::PI

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping::approximate_inverse_tone_mapping
#endif

fn specular_transmissive_light(world_position: vec4<f32>, frag_coord: vec3<f32>, view_z: f32, N: vec3<f32>, V: vec3<f32>, F0: vec3<f32>, ior: f32, thickness: f32, perceptual_roughness: f32, specular_transmissive_color: vec3<f32>, transmitted_environment_light_specular: vec3<f32>) -> vec3<f32> {
    // Calculate the ratio between refaction indexes. Assume air/vacuum for the space outside the mesh
    let eta = 1.0 / ior;

    // Calculate incidence vector (opposite to view vector) and its dot product with the mesh normal
    let I = -V;
    let NdotI = dot(N, I);

    // Calculate refracted direction using Snell's law
    let k = 1.0 - eta * eta * (1.0 - NdotI * NdotI);
    let T = eta * I - (eta * NdotI + sqrt(k)) * N;

    // Calculate the exit position of the refracted ray, by propagating refacted direction through thickness
    let exit_position = world_position.xyz + T * thickness;

    // Transform exit_position into clip space
    let clip_exit_position = view_bindings::view.clip_from_world * vec4<f32>(exit_position, 1.0);

    // Scale / offset position so that coordinate is in right space for sampling transmissive background texture
    let offset_position = (clip_exit_position.xy / clip_exit_position.w) * vec2<f32>(0.5, -0.5) + 0.5;

    // Fetch background color
    var background_color: vec4<f32>;
    if perceptual_roughness == 0.0 {
        // If the material has zero roughness, we can use a faster approach without the blur
        background_color = fetch_transmissive_background_non_rough(offset_position, frag_coord);
    } else {
        background_color = fetch_transmissive_background(offset_position, frag_coord, view_z, perceptual_roughness);
    }

    // Compensate for exposure, since the background color is coming from an already exposure-adjusted texture
    background_color = vec4(background_color.rgb / view_bindings::view.exposure, background_color.a);

    // Dot product of the refracted direction with the exit normal (Note: We assume the exit normal is the entry normal but inverted)
    let MinusNdotT = dot(-N, T);

    // Calculate 1.0 - fresnel factor (how much light is _NOT_ reflected, i.e. how much is transmitted)
    let F = vec3(1.0) - lighting::fresnel(F0, MinusNdotT);

    // Calculate final color by applying fresnel multiplied specular transmissive color to a mix of background color and transmitted specular environment light
    return F * specular_transmissive_color * mix(transmitted_environment_light_specular, background_color.rgb, background_color.a);
}

fn fetch_transmissive_background_non_rough(offset_position: vec2<f32>, frag_coord: vec3<f32>) -> vec4<f32> {
    var background_color = textureSampleLevel(
        view_bindings::view_transmission_texture,
        view_bindings::view_transmission_sampler,
        offset_position,
        0.0
    );

#ifdef DEPTH_PREPASS
#ifndef WEBGL2
    // Use depth prepass data to reject values that are in front of the current fragment
    if prepass_utils::prepass_depth(vec4<f32>(offset_position * view_bindings::view.viewport.zw, 0.0, 0.0), 0u) > frag_coord.z {
        background_color.a = 0.0;
    }
#endif
#endif

#ifdef TONEMAP_IN_SHADER
    background_color = approximate_inverse_tone_mapping(background_color, view_bindings::view.color_grading);
#endif

    return background_color;
}

fn fetch_transmissive_background(offset_position: vec2<f32>, frag_coord: vec3<f32>, view_z: f32, perceptual_roughness: f32) -> vec4<f32> {
    // Calculate view aspect ratio, used to scale offset so that it's proportionate
    let aspect = view_bindings::view.viewport.z / view_bindings::view.viewport.w;

    // Calculate how “blurry” the transmission should be.
    // Blur is more or less eyeballed to look approximately “right”, since the “correct”
    // approach would involve projecting many scattered rays and figuring out their individual
    // exit positions. IRL, light rays can be scattered when entering/exiting a material (due to
    // roughness) or inside the material (due to subsurface scattering). Here, we only consider
    // the first scenario.
    //
    // Blur intensity is:
    // - proportional to the square of `perceptual_roughness`
    // - proportional to the inverse of view z
    let blur_intensity = (perceptual_roughness * perceptual_roughness) / view_z;

#ifdef SCREEN_SPACE_SPECULAR_TRANSMISSION_BLUR_TAPS
    let num_taps = #{SCREEN_SPACE_SPECULAR_TRANSMISSION_BLUR_TAPS}; // Controlled by the `Camera3d::screen_space_specular_transmission_quality` property
#else
    let num_taps = 8; // Fallback to 8 taps, if not specified
#endif
    let num_spirals = i32(ceil(f32(num_taps) / 8.0));
#ifdef TEMPORAL_JITTER
    let random_angle = interleaved_gradient_noise(frag_coord.xy, view_bindings::globals.frame_count);
#else
    let random_angle = interleaved_gradient_noise(frag_coord.xy, 0u);
#endif
    // Pixel checkerboard pattern (helps make the interleaved gradient noise pattern less visible)
    let pixel_checkboard = (
#ifdef TEMPORAL_JITTER
        // 0 or 1 on even/odd pixels, alternates every frame
        (i32(frag_coord.x) + i32(frag_coord.y) + i32(view_bindings::globals.frame_count)) % 2
#else
        // 0 or 1 on even/odd pixels
        (i32(frag_coord.x) + i32(frag_coord.y)) % 2
#endif
    );

    var result = vec4<f32>(0.0);
    for (var i: i32 = 0; i < num_taps; i = i + 1) {
        let current_spiral = (i >> 3u);
        let angle = (random_angle + f32(current_spiral) / f32(num_spirals)) * 2.0 * PI;
        let m = vec2(sin(angle), cos(angle));
        let rotation_matrix = mat2x2(
            m.y, -m.x,
            m.x, m.y
        );

        // Get spiral offset
        var spiral_offset: vec2<f32>;
        switch i & 7 {
            // https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare (slides 120-135)
            // TODO: Figure out a more reasonable way of doing this, as WGSL
            // seems to only allow constant indexes into constant arrays at the moment.
            // The downstream shader compiler should be able to optimize this into a single
            // constant when unrolling the for loop, but it's still not ideal.
            case 0: { spiral_offset = utils::SPIRAL_OFFSET_0_; } // Note: We go even first and then odd, so that the lowest
            case 1: { spiral_offset = utils::SPIRAL_OFFSET_2_; } // quality possible (which does 4 taps) still does a full spiral
            case 2: { spiral_offset = utils::SPIRAL_OFFSET_4_; } // instead of just the first half of it
            case 3: { spiral_offset = utils::SPIRAL_OFFSET_6_; }
            case 4: { spiral_offset = utils::SPIRAL_OFFSET_1_; }
            case 5: { spiral_offset = utils::SPIRAL_OFFSET_3_; }
            case 6: { spiral_offset = utils::SPIRAL_OFFSET_5_; }
            case 7: { spiral_offset = utils::SPIRAL_OFFSET_7_; }
            default: {}
        }

        // Make each consecutive spiral slightly smaller than the previous one
        spiral_offset *= 1.0 - (0.5 * f32(current_spiral + 1) / f32(num_spirals));

        // Rotate and correct for aspect ratio
        let rotated_spiral_offset = (rotation_matrix * spiral_offset) * vec2(1.0, aspect);

        // Calculate final offset position, with blur and spiral offset
        let modified_offset_position = offset_position + rotated_spiral_offset * blur_intensity * (1.0 - f32(pixel_checkboard) * 0.1);

        // Sample the view transmission texture at the offset position + noise offset, to get the background color
        var sample = textureSampleLevel(
            view_bindings::view_transmission_texture,
            view_bindings::view_transmission_sampler,
            modified_offset_position,
            0.0
        );

#ifdef DEPTH_PREPASS
#ifndef WEBGL2
        // Use depth prepass data to reject values that are in front of the current fragment
        if prepass_utils::prepass_depth(vec4<f32>(modified_offset_position * view_bindings::view.viewport.zw, 0.0, 0.0), 0u) > frag_coord.z {
            sample = vec4<f32>(0.0);
        }
#endif
#endif

        // As blur intensity grows higher, gradually limit *very bright* color RGB values towards a
        // maximum length of 1.0 to prevent stray “firefly” pixel artifacts. This can potentially make
        // very strong emissive meshes appear much dimmer, but the artifacts are noticeable enough to
        // warrant this treatment.
        let normalized_rgb = normalize(sample.rgb);
        result += vec4(min(sample.rgb, normalized_rgb / saturate(blur_intensity / 2.0)), sample.a);
    }

    result /= f32(num_taps);

#ifdef TONEMAP_IN_SHADER
    result = approximate_inverse_tone_mapping(result, view_bindings::view.color_grading);
#endif

    return result;
}
