// A postprocessing pass that performs screen-space reflections.

#define_import_path bevy_pbr::ssr

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_pbr::{
    lighting,
    mesh_view_bindings::{view, depth_prepass_texture, deferred_prepass_texture, ssr_settings},
    pbr_deferred_functions::pbr_input_from_deferred_gbuffer,
    pbr_deferred_types,
    pbr_functions,
    prepass_utils,
    utils,
    view_transformations::{
        depth_ndc_to_view_z,
        frag_coord_to_ndc,
        ndc_to_frag_coord,
        ndc_to_uv,
        position_view_to_ndc,
        position_world_to_view,
    },
}
#import bevy_render::view::View

#ifdef ENVIRONMENT_MAP
#import bevy_pbr::environment_map
#endif

@group(1) @binding(0) var framebuffer_texture: texture_2d<f32>;

const RAY_HIT: f32 = 1.0;
const RAY_MISS: f32 = -1.0;
const RAY_CONTINUE: f32 = 0.0;

// Performs a single step of the screen space reflection raymarching.
//
// The arguments are:
//
// * `P_view` is the origin of the ray in view space (i.e. the position of the
//   fragment being rendered).
//
// * `P_ndc` is the origin of the ray in screen space.
//
// * `R_view` is the reflection vector in view space.
//
// * `R_ndc` is the reflection vector in screen space.
//
// * `t_ndc` is the current value of the parameter that determines how far along
//   the ray we are in screen space.
fn evaluate_ssr_step(
    P_view: vec3<f32>,
    P_ndc: vec2<f32>,
    R_view: vec3<f32>,
    R_ndc: vec2<f32>,
    t_ndc: f32,
) -> vec3<f32> {
    // Advance by one step. `Q_ndc` stores our current position along the ray.
    let Q_ndc = P_ndc + t_ndc * R_ndc;

    // If we left the screen bounds, we missed.
    if (any(abs(Q_ndc) > vec2(1.0))) {
        return vec3(0.0, 0.0, RAY_MISS);
    }

    // Convert the screen space parameter `t_ndc` to the view space parameter
    // `t_view` by applying perspective correction.
    let t_view = P_view.z * t_ndc / (R_view.z + P_view.z - R_view.z * t_ndc);
    let Q_z_view = P_view.z + R_view.z * t_view;

    let Q_frag_coord = ndc_to_frag_coord(Q_ndc);
    let depth_ndc = prepass_utils::prepass_depth(vec4(Q_frag_coord, 0.0, 0.0), 0u);
    let depth_view = depth_ndc_to_view_z(depth_ndc);

    let hit = Q_z_view < depth_view && Q_z_view >= depth_view - ssr_settings.thickness;
    return vec3(Q_frag_coord, select(RAY_CONTINUE, RAY_HIT, hit));
}

// Returns the reflected color in the RGB channel and the specular occlusion in
// the alpha channel.
//
// The general approach here is similar to [1]. We first project the reflection
// ray into screen space. Then we perform uniform steps along that screen-space
// reflected ray, converting each step to view space.
//
// The arguments are:
//
// * `R_world`: The reflection vector in world space.
//
// * `P_world`: The current position in world space.
//
// * `P_frag_coord`: The current position in framebuffer space.
//
// [1]: https://lettier.github.io/3d-game-shaders-for-beginners/screen-space-reflection.html
fn evaluate_ssr(R_world: vec3<f32>, P_world: vec3<f32>, P_frag_coord: vec4<f32>) -> vec4<f32> {
    // In this code, "NDC" refers to the 2D normalized device coordinates, with
    // depth omitted.

    // Calculate our current position.
    let P_view = position_world_to_view(P_world);
    let P_ndc = frag_coord_to_ndc(P_frag_coord).xy;

    // Determine the reflection vector. Note that this is not normalized; this
    // matters for the perspective correction that will happen during the
    // raymarch.
    let PR_view = position_world_to_view(P_world + R_world);
    let R_ndc = position_view_to_ndc(PR_view).xy - P_ndc;
    let R_view = PR_view - P_view;

    // Calculate how much to increment the parametric parameter t by for each
    // major step.
    let t_step_ndc = 2.0 * inverseSqrt(dot(R_ndc, R_ndc)) / f32(ssr_settings.major_step_count);

    // Do the major search. This is just a linear series of raymarching steps,
    // spaced a fixed distance apart *in screen space*.
    var step_index: i32;
    var step_result: vec3<f32>;
    for (step_index = 1; step_index <= ssr_settings.major_step_count; step_index += 1) {
        // Evaluate a single step.
        let t_ndc = f32(step_index) * t_step_ndc;
        step_result = evaluate_ssr_step(P_view, P_ndc, R_view, R_ndc, t_ndc);

        // If we fell off the screen, bail out and return a miss.
        if (step_result.z == RAY_MISS) {
            return vec4(0.0, 0.0, 0.0, 1.0);
        }

        // If we hit something, we're done.
        if (step_result.z == RAY_HIT) {
            break;
        }
    }

    // If we finished the above loop without finding anything, then bail out.
    if (step_result.z == RAY_CONTINUE) {
        return vec4(0.0, 0.0, 0.0, 1.0);
    }

    // Perform the minor search. This is a binary search that narrows down the
    // results of the major search in order to find a more accurate point of
    // ray-geometry intersetion.
    var t_min_ndc = f32(step_index - 1) * t_step_ndc;
    var t_max_ndc = f32(step_index) * t_step_ndc;
    for (step_index = 0; step_index < ssr_settings.minor_step_count; step_index += 1) {
        // Evaluate the midpoint of our bracketing range.
        let t_ndc = mix(t_min_ndc, t_max_ndc, 0.5);
        step_result = evaluate_ssr_step(P_view, P_ndc, R_view, R_ndc, t_ndc);

        // Update our bracketing range as appropriate.
        if (step_result.z == RAY_CONTINUE) {
            t_min_ndc = t_ndc;
        } else {
            t_max_ndc = t_ndc;
        }
    }

    // We successfully found an intersection. Sample the framebuffer texture at
    // the appropriate location.
    return vec4(textureLoad(framebuffer_texture, vec2<i32>(round(step_result.xy)), 0).rgb, 0.0);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Sample the depth.
    var frag_coord = in.position;
    frag_coord.z = prepass_utils::prepass_depth(in.position, 0u);

    // Load the G-buffer data.
    let fragment = textureLoad(framebuffer_texture, vec2<i32>(frag_coord.xy), 0);
    let gbuffer = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    let pbr_input = pbr_input_from_deferred_gbuffer(frag_coord, gbuffer);

    // Don't do anything if the surface is too rough, since we can't blur or do
    // temporal accumulation yet.
    let perceptual_roughness = pbr_input.material.perceptual_roughness;
    if (perceptual_roughness > ssr_settings.perceptual_roughness_threshold) {
        return fragment;
    }

    // Unpack the PBR input.
    var specular_occlusion = pbr_input.specular_occlusion;
    let world_position = pbr_input.world_position.xyz;
    let N = pbr_input.N;
    let V = pbr_input.V;

    // Calculate the reflection vector.
    let R = reflect(-V, N);

    // Do the raymarching.
    let ssr_specular = evaluate_ssr(R, world_position, frag_coord);
    var indirect_light = ssr_specular.rgb;
    specular_occlusion *= ssr_specular.a;

    // Sample the environment map if necessary.
    //
    // This will take the specular part of the environment map into account if
    // the ray missed. Otherwise, it only takes the diffuse part.
#ifdef ENVIRONMENT_MAP
    // Unpack values required for environment mapping.
    let base_color = pbr_input.material.base_color.rgb;
    let metallic = pbr_input.material.metallic;
    let reflectance = pbr_input.material.reflectance;
    let specular_transmission = pbr_input.material.specular_transmission;
    let diffuse_transmission = pbr_input.material.diffuse_transmission;
    let diffuse_occlusion = pbr_input.diffuse_occlusion;

    // Calculate various other values needed for environment mapping.
    let roughness = lighting::perceptualRoughnessToRoughness(perceptual_roughness);
    let diffuse_color = pbr_functions::calculate_diffuse_color(
        base_color,
        metallic,
        specular_transmission,
        diffuse_transmission
    );
    let NdotV = max(dot(N, V), 0.0001);
    let f_ab = lighting::F_AB(perceptual_roughness, NdotV);
    let F0 = pbr_functions::calculate_F0(base_color, metallic, reflectance);

    // Sample the environment map.
    let environment_light = environment_map::environment_map_light(
        perceptual_roughness,
        roughness,
        diffuse_color,
        NdotV,
        f_ab,
        N,
        R,
        F0,
        world_position,
        false);

    // Accumulate the environment map light.
    indirect_light += view.exposure *
        (environment_light.diffuse * diffuse_occlusion +
        environment_light.specular * specular_occlusion);
#endif

    // Write the results.
    return vec4(fragment.rgb + indirect_light, 1.0);
}
