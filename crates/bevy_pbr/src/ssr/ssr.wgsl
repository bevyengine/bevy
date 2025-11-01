// A postprocessing pass that performs screen-space reflections.

#define_import_path bevy_pbr::ssr

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_pbr::{
    clustered_forward,
    lighting,
    lighting::{LAYER_BASE, LAYER_CLEARCOAT},
    mesh_view_bindings::{view, depth_prepass_texture, deferred_prepass_texture, ssr_settings},
    pbr_deferred_functions::pbr_input_from_deferred_gbuffer,
    pbr_deferred_types,
    prepass_utils,
    raymarch::{
        depth_ray_march_from_cs,
        depth_ray_march_march,
        depth_ray_march_new_from_depth,
        depth_ray_march_to_ws_dir,
    },
    utils,
    view_transformations::{
        depth_ndc_to_view_z,
        frag_coord_to_ndc,
        ndc_to_frag_coord,
        ndc_to_uv,
        position_view_to_ndc,
        position_world_to_ndc,
        position_world_to_view,
    },
}
#import bevy_render::view::View

#ifdef ENVIRONMENT_MAP
#import bevy_pbr::environment_map
#endif

// The texture representing the color framebuffer.
@group(2) @binding(0) var color_texture: texture_2d<f32>;

// The sampler that lets us sample from the color framebuffer.
@group(2) @binding(1) var color_sampler: sampler;

// Group 1, bindings 2 and 3 are in `raymarch.wgsl`.

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
// [1]: https://lettier.github.io/3d-game-shaders-for-beginners/screen-space-reflection.html
fn evaluate_ssr(R_world: vec3<f32>, P_world: vec3<f32>) -> vec4<f32> {
    let depth_size = vec2<f32>(textureDimensions(depth_prepass_texture));

    var raymarch = depth_ray_march_new_from_depth(depth_size);
    depth_ray_march_from_cs(&raymarch, position_world_to_ndc(P_world));
    depth_ray_march_to_ws_dir(&raymarch, normalize(R_world));
    raymarch.linear_steps = ssr_settings.linear_steps;
    raymarch.bisection_steps = ssr_settings.bisection_steps;
    raymarch.use_secant = ssr_settings.use_secant != 0u;
    raymarch.depth_thickness_linear_z = ssr_settings.thickness;
    raymarch.jitter = 1.0;  // Disable jitter for now.
    raymarch.march_behind_surfaces = false;

    let raymarch_result = depth_ray_march_march(&raymarch);
    if (raymarch_result.hit) {
        return vec4(
            textureSampleLevel(color_texture, color_sampler, raymarch_result.hit_uv, 0.0).rgb,
            0.0
        );
    }

    return vec4(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Sample the depth.
    var frag_coord = in.position;
    frag_coord.z = prepass_utils::prepass_depth(in.position, 0u);

    // Load the G-buffer data.
    let fragment = textureLoad(color_texture, vec2<i32>(frag_coord.xy), 0);
    let gbuffer = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    let pbr_input = pbr_input_from_deferred_gbuffer(frag_coord, gbuffer);

    // Don't do anything if the surface is too rough, since we can't blur or do
    // temporal accumulation yet.
    let perceptual_roughness = pbr_input.material.perceptual_roughness;
    if (perceptual_roughness > ssr_settings.perceptual_roughness_threshold) {
        return fragment;
    }

#ifdef ENVIRONMENT_MAP
    var lighting_input = lighting::pbr_input_to_lighting_input(pbr_input);
    let R = lighting_input.layers[LAYER_BASE].R;
#else // ENVIRONMENT_MAP
    let R = reflect(-pbr_input.V, pbr_input.N);
#endif // ENVIRONMENT_MAP

    // Do the raymarching.
    let world_position = pbr_input.world_position.xyz;
    let ssr_specular = evaluate_ssr(R, world_position);
    var indirect_light = ssr_specular.rgb;

    let specular_occlusion = pbr_input.specular_occlusion * ssr_specular.a;

    // Sample the environment map if necessary.
    //
    // This will take the specular part of the environment map into account if
    // the ray missed. Otherwise, it only takes the diffuse part.
    //
    // TODO: Merge this with the duplicated code in `apply_pbr_lighting`.
#ifdef ENVIRONMENT_MAP
    // Determine which cluster we're in. We'll need this to find the right
    // reflection probe.
    let cluster_index = clustered_forward::fragment_cluster_index(
        frag_coord.xy, frag_coord.z, false);
    var clusterable_object_index_ranges =
        clustered_forward::unpack_clusterable_object_index_ranges(cluster_index);

    // Sample the environment map.
    let environment_light = environment_map::environment_map_light(
        &lighting_input, &clusterable_object_index_ranges, false);

    // Accumulate the environment map light.
    indirect_light += view.exposure *
        (environment_light.diffuse * pbr_input.diffuse_occlusion +
        environment_light.specular * specular_occlusion);
#endif // ENVIRONMENT_MAP

    // Write the results.
    return vec4(fragment.rgb + indirect_light, 1.0);
}
