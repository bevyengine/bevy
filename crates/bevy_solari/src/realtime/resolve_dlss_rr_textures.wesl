import bevy_pbr::render::pbr_functions::{calculate_diffuse_color, calculate_F0};
import bevy_solari::realtime::gbuffer_utils::gpixel_resolve;
import bevy_solari::realtime::bindings::{gbuffer, depth_buffer, motion_vectors, view, diffuse_albedo, specular_albedo, normal_roughness, specular_motion_vectors, dlss_rr_depth, dlss_rr_motion_vectors, SPECULAR_GUIDE_ROUGHNESS_THRESHOLD};

enable wgpu_ray_query;

@compute @workgroup_size(8, 8, 1)
fn resolve_dlss_rr_textures(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_id = global_id.xy;
    if any(pixel_id >= vec2u(view.main_pass_viewport.zw)) { return; }

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    let surface_motion_vector = textureLoad(motion_vectors, pixel_id, 0).xy;

    // Defaults describing this pixel's own surface, which is the right answer for everything that
    // isn't a mirror. `replace_primary_surface` in `initial_path` overwrites them where it applies,
    // which is why these have to be written for every pixel first - including the sky, which the
    // early-out below returns from.
    //
    // The two mirror-aware copies are plain copies here. They exist so that DLSS Ray Reconstruction
    // can be handed a depth and a motion vector describing the reflection while the prepass originals
    // stay untouched for ReSTIR reprojection and super resolution.
    textureStore(dlss_rr_motion_vectors, pixel_id, vec4(surface_motion_vector, vec2(0.0)));
    textureStore(dlss_rr_depth, pixel_id, vec4(depth));

    if depth == 0.0 {
        textureStore(specular_motion_vectors, pixel_id, vec4(surface_motion_vector, vec2(0.0)));
        textureStore(diffuse_albedo, pixel_id, vec4(0.0));
        textureStore(specular_albedo, pixel_id, vec4(0.5));
        textureStore(normal_roughness, pixel_id, vec4(0.0, 0.0, 1.0, 0.0));
        return;
    }

    let surface = gpixel_resolve(textureLoad(gbuffer, pixel_id, 0), depth, pixel_id, view.main_pass_viewport.zw, view.world_from_clip);

    // The specular motion vector is deliberately *not* reset for pixels the chain is eligible to
    // write, which is what makes glossy reflections usable at all.
    //
    // A glossy surface only gets a reflection distance on frames where the path tracer happens to draw
    // its specular lobe - a coin flip weighted by `specular_lobe_sampling_probability`, so roughly
    // half the time. Resetting to the surface vector on the other frames would hand the denoiser a
    // motion field alternating between "this reflection moves with the reflected object" and "it is
    // painted on and slides with the floor", frame after frame. That incoherence is worse than either
    // answer alone.
    //
    // Leaving the texture untouched instead makes the guide at most a few frames stale, and always a
    // reflection vector rather than a surface one. It works because the guide textures are allocated
    // once and reused while the view size holds, so last frame's values are still there. The cost is a
    // stale vector for a frame or two after a disocclusion, which is a thing DLSS already handles.
    if surface.material.roughness > SPECULAR_GUIDE_ROUGHNESS_THRESHOLD {
        textureStore(specular_motion_vectors, pixel_id, vec4(surface_motion_vector, vec2(0.0)));
    }

    let F0 = calculate_F0(surface.material.base_color, surface.material.metallic, vec3(surface.material.reflectance));
    let wo = normalize(view.world_position - surface.world_position);

    textureStore(diffuse_albedo, pixel_id, vec4(calculate_diffuse_color(surface.material.base_color, surface.material.metallic, 0.0, 0.0), 0.0));
    textureStore(specular_albedo, pixel_id, vec4(env_brdf_approx2(F0, surface.material.roughness, surface.world_normal, wo), 0.0));
    textureStore(normal_roughness, pixel_id, vec4(surface.world_normal, surface.material.perceptual_roughness));
}

fn env_brdf_approx2(specular_color: vec3<f32>, alpha: f32, N: vec3<f32>, V: vec3<f32>) -> vec3<f32> {
    let NoV = abs(dot(N, V));

    var X: vec4<f32>;
    X.x = 1.0;
    X.y = NoV;
    X.z = NoV * NoV;
    X.w = NoV * X.z;

    var Y: vec4<f32>;
    Y.x = 1.0;
    Y.y = alpha;
    Y.z = alpha * alpha;
    Y.w = alpha * Y.z;

    let M1 = mat2x2<f32>(0.99044, 1.29678, -1.28514, -0.755907);
    let M2 = mat3x3<f32>(1.0, 20.3225, 121.563, 2.92338, -27.0302, 626.13, 59.4188, 222.592, 316.627);
    let M3 = mat2x2<f32>(0.0365463, 9.0632, 3.32707, -9.04756);
    let M4 = mat3x3<f32>(1.0, 9.04401, 5.56589, 3.59685, -16.3174, 19.7886, -1.36772, 9.22949, -20.2123);

    var bias = dot(M1 * X.xy, Y.xy) / dot(M2 * X.xyw, Y.xyw);
    let scale = dot(M3 * X.xy, Y.xy) / dot(M4 * X.xzw, Y.xyw);

    bias *= saturate(specular_color.g * 50.0);

    return fma(specular_color, vec3(max(0.0, scale)), vec3(max(0.0, bias)));
}
