// A postprocessing pass that performs screen-space reflections.

#define_import_path bevy_pbr::ssr

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_pbr::{
    clustered_forward,
    lighting,
    lighting::{LAYER_BASE, LAYER_CLEARCOAT},
    mesh_view_bindings::{
        view,
        globals,
        depth_prepass_texture,
        deferred_prepass_texture,
        ssr_settings
    },
    pbr_deferred_functions::pbr_input_from_deferred_gbuffer,
    pbr_deferred_types,
    pbr_functions,
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
#import bevy_render::{
    view::View,
    maths::orthonormalize,
}

#ifdef ENVIRONMENT_MAP
#import bevy_pbr::environment_map
#endif

// The texture representing the color framebuffer.
@group(2) @binding(0) var color_texture: texture_2d<f32>;

// The sampler that lets us sample from the color framebuffer.
@group(2) @binding(1) var color_sampler: sampler;

// Group 1, bindings 2 and 3 are in `raymarch.wgsl`.

struct BrdfSample {
    wi: vec3<f32>,
    value_over_pdf: vec3<f32>,
}

fn sample_specular_brdf(wo: vec3<f32>, roughness: f32, F0: vec3<f32>, urand: vec2<f32>, N: vec3<f32>) -> BrdfSample {
    var brdf_sample: BrdfSample;
    
    // Use VNDF sampling for the half-vector.
    // wo is view direction. In sample_visible_ggx, view is from surface to eye.
    // In our context, V is from surface to eye, so we use V.
    // sample_visible_ggx handles world space if passed world space N and V.
    let wi = lighting::sample_visible_ggx(urand, roughness, N, wo);
    let H = normalize(wo + wi);
    let NdotL = max(dot(N, wi), 0.0001);
    let NdotV = max(dot(N, wo), 0.0001);
    let VdotH = max(dot(wo, H), 0.0001);

    let F = lighting::F_Schlick_vec(F0, 1.0, VdotH);
    let G1V = lighting::G_Smith(NdotV, NdotV, roughness);
    let G2 = lighting::G_Smith(NdotV, NdotL, roughness);

    brdf_sample.wi = wi;
    // (BRDF * NdotL) / PDF = F * G2 / G1V
    brdf_sample.value_over_pdf = F * (G2 / G1V);

    return brdf_sample;
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
// * `jitter`: Jitter to apply to the first step of the linear search; 0..=1
//   range.
//
// [1]: https://lettier.github.io/3d-game-shaders-for-beginners/screen-space-reflection.html
fn evaluate_ssr(R_world: vec3<f32>, P_world: vec3<f32>, jitter: f32) -> vec4<f32> {
    let depth_size = vec2<f32>(textureDimensions(depth_prepass_texture));

    var raymarch = depth_ray_march_new_from_depth(depth_size);
    depth_ray_march_from_cs(&raymarch, position_world_to_ndc(P_world));
    depth_ray_march_to_ws_dir(&raymarch, normalize(R_world));
    raymarch.linear_steps = ssr_settings.linear_steps;
    raymarch.bisection_steps = ssr_settings.bisection_steps;
    raymarch.use_secant = ssr_settings.use_secant != 0u;
    raymarch.depth_thickness_linear_z = ssr_settings.thickness;
    raymarch.jitter = jitter;
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

    // Unpack the PBR input.
    var specular_occlusion = pbr_input.specular_occlusion;
    let world_position = pbr_input.world_position.xyz;
    let N = pbr_input.N;
    let V = pbr_input.V;

    // Build a basis for sampling the BRDF, as BRDF sampling functions assume that the normal faces +Z.
    let tangent_to_world = orthonormalize(N);

    // Get a good quality sample from the BRDF, using VNDF.
    let roughness = lighting::perceptualRoughnessToRoughness(perceptual_roughness);
    let F0 = pbr_functions::calculate_F0(pbr_input.material.base_color.rgb, pbr_input.material.metallic, pbr_input.material.reflectance);

    // Get some random numbers.
    // We use a custom seed to avoid the 64-frame cycle of interleaved_gradient_noise,
    // which can cause visible flickering at high frame rates.
    var rng_seed = u32(in.position.x) + u32(in.position.y) * 16384u + globals.frame_count * 31337u;
    let urand = utils::rand_vec2f(&rng_seed);
    let raymarch_jitter = utils::rand_f(&rng_seed);

    // Sample the BRDF.
    // wo = mul(V, tangent_to_world) if we were in tangent space.
    // But sample_visible_ggx takes world space N and V.
    // The h3r2tic example uses tangent space sampling.
    // Let's stick to the example's structure as much as possible.
    
    // In tangent space, N is (0, 0, 1).
    let N_tangent = vec3(0.0, 0.0, 1.0);
    let V_tangent = V * tangent_to_world; // Assuming mat3x3 * vec3 is what we want for world->tangent.
    // Wait, if tangent_to_world columns are x_basis, y_basis, z_basis(N), then
    // V_tangent = vec3(dot(V, x_basis), dot(V, y_basis), dot(V, N)).
    // Which is V * tangent_to_world in WGSL (row-vector * matrix).
    
    let brdf_sample = sample_specular_brdf(V_tangent, roughness, F0, urand, N_tangent);
    let R = tangent_to_world * brdf_sample.wi;

    // Do the raymarching.
    let ssr_specular = evaluate_ssr(R, world_position, raymarch_jitter);
    var indirect_light = ssr_specular.rgb * brdf_sample.value_over_pdf;
    specular_occlusion *= ssr_specular.a;

    // Sample the environment map if necessary.
    //
    // This will take the specular part of the environment map into account if
    // the ray missed. Otherwise, it only takes the diffuse part.
    //
    // TODO: Merge this with the duplicated code in `apply_pbr_lighting`.
#ifdef ENVIRONMENT_MAP
    // Unpack values required for environment mapping.
    let base_color = pbr_input.material.base_color.rgb;
    let metallic = pbr_input.material.metallic;
    let reflectance = pbr_input.material.reflectance;
    let specular_transmission = pbr_input.material.specular_transmission;
    let diffuse_transmission = pbr_input.material.diffuse_transmission;
    let diffuse_occlusion = pbr_input.diffuse_occlusion;

#ifdef STANDARD_MATERIAL_CLEARCOAT
    // Do the above calculations again for the clearcoat layer. Remember that
    // the clearcoat can have its own roughness and its own normal.
    let clearcoat = pbr_input.material.clearcoat;
    let clearcoat_perceptual_roughness = pbr_input.material.clearcoat_perceptual_roughness;
    let clearcoat_roughness = lighting::perceptualRoughnessToRoughness(clearcoat_perceptual_roughness);
    let clearcoat_N = pbr_input.clearcoat_N;
    let clearcoat_NdotV = max(dot(clearcoat_N, pbr_input.V), 0.0001);
    let clearcoat_R = reflect(-pbr_input.V, clearcoat_N);
#endif  // STANDARD_MATERIAL_CLEARCOAT

    // Calculate various other values needed for environment mapping.
    let env_roughness = lighting::perceptualRoughnessToRoughness(perceptual_roughness);
    let diffuse_color = pbr_functions::calculate_diffuse_color(
        base_color,
        metallic,
        specular_transmission,
        diffuse_transmission
    );
    let NdotV = max(dot(N, V), 0.0001);
    let F_ab = lighting::F_AB(perceptual_roughness, NdotV);
    let F0_env = pbr_functions::calculate_F0(base_color, metallic, reflectance);

    // Pack all the values into a structure.
    var lighting_input: lighting::LightingInput;
    lighting_input.layers[LAYER_BASE].NdotV = NdotV;
    lighting_input.layers[LAYER_BASE].N = N;
    lighting_input.layers[LAYER_BASE].R = R;
    lighting_input.layers[LAYER_BASE].perceptual_roughness = perceptual_roughness;
    lighting_input.layers[LAYER_BASE].roughness = env_roughness;
    lighting_input.P = world_position.xyz;
    lighting_input.V = V;
    lighting_input.diffuse_color = diffuse_color;
    lighting_input.F0_ = F0_env;
    lighting_input.F_ab = F_ab;
#ifdef STANDARD_MATERIAL_CLEARCOAT
    lighting_input.layers[LAYER_CLEARCOAT].NdotV = clearcoat_NdotV;
    lighting_input.layers[LAYER_CLEARCOAT].N = clearcoat_N;
    lighting_input.layers[LAYER_CLEARCOAT].R = clearcoat_R;
    lighting_input.layers[LAYER_CLEARCOAT].perceptual_roughness = clearcoat_perceptual_roughness;
    lighting_input.layers[LAYER_CLEARCOAT].roughness = clearcoat_roughness;
    lighting_input.clearcoat_strength = clearcoat;
#endif  // STANDARD_MATERIAL_CLEARCOAT

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
        (environment_light.diffuse * diffuse_occlusion +
        environment_light.specular * specular_occlusion);
#endif

    // Write the results.
    return vec4(fragment.rgb + indirect_light, 1.0);
}
