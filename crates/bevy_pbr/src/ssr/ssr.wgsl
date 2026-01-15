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

@group(2) @binding(4) var stbn_texture: texture_2d_array<f32>;

struct BrdfSample {
    wi: vec3<f32>,
    value_over_pdf: vec3<f32>,
}

fn sample_specular_brdf(wo: vec3<f32>, roughness: f32, F0: vec3<f32>, urand: vec2<f32>, N: vec3<f32>) -> BrdfSample {
    var brdf_sample: BrdfSample;
    
    // Use VNDF sampling for the half-vector.
    let wi = lighting::sample_visible_ggx(urand, roughness, N, wo);
    let H = normalize(wo + wi);
    let NdotL = max(dot(N, wi), 0.0001);
    let NdotV = max(dot(N, wo), 0.0001);
    let VdotH = max(dot(wo, H), 0.0001);

    let F = lighting::F_Schlick_vec(F0, 1.0, VdotH);

    // Height-correlated Smith G2 / G1(V)
    let a2 = roughness * roughness;
    let lambdaV = NdotL * sqrt((NdotV - a2 * NdotV) * NdotV + a2);
    let lambdaL = NdotV * sqrt((NdotL - a2 * NdotL) * NdotL + a2);

    brdf_sample.wi = wi;
    brdf_sample.value_over_pdf = F * (NdotV * NdotL + lambdaV) / (lambdaV + lambdaL);

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

    // Don't do anything if the surface is too rough or too smooth
    let perceptual_roughness = pbr_input.material.perceptual_roughness;

    var min_fade: f32;
    if (ssr_settings.min_perceptual_roughness >= ssr_settings.min_perceptual_roughness_fully_active) {
        min_fade = step(ssr_settings.min_perceptual_roughness, perceptual_roughness);
    } else {
        min_fade = smoothstep(
            ssr_settings.min_perceptual_roughness,
            ssr_settings.min_perceptual_roughness_fully_active,
            perceptual_roughness
        );
    }

    var max_fade: f32;
    if (ssr_settings.max_perceptual_roughness_starts_to_fade >= ssr_settings.max_perceptual_roughness) {
        max_fade = step(perceptual_roughness, ssr_settings.max_perceptual_roughness);
    } else {
        max_fade = 1.0 - smoothstep(
            ssr_settings.max_perceptual_roughness_starts_to_fade,
            ssr_settings.max_perceptual_roughness,
            perceptual_roughness
        );
    }

    var fade = saturate(min_fade) * saturate(max_fade);

    let ndc_position = frag_coord_to_ndc(vec4(in.position.xy, frag_coord.z, 1.0));
    let uv = ndc_to_uv(ndc_position.xy);
    let dist = min(uv, vec2(1.0) - uv);
    var fade_xy: vec2<f32>;
    if (ssr_settings.edge_fadeout_no_longer_active >= ssr_settings.edge_fadeout_fully_active) {
        fade_xy = step(vec2(ssr_settings.edge_fadeout_no_longer_active), dist);
    } else {
        fade_xy = smoothstep(
            vec2(ssr_settings.edge_fadeout_no_longer_active),
            vec2(ssr_settings.edge_fadeout_fully_active),
            dist
        );
    }
    fade *= fade_xy.x * fade_xy.y;

    if (fade <= 0.0 || perceptual_roughness > ssr_settings.max_perceptual_roughness) {
        return fragment;
    }

    // Unpack the PBR input.
    var specular_occlusion = pbr_input.specular_occlusion;
    let world_position = pbr_input.world_position.xyz;
    let N = pbr_input.N;
    let V = pbr_input.V;

    // Build a basis for sampling the BRDF.
    let tangent_to_world = orthonormalize(N);

    let roughness = lighting::perceptualRoughnessToRoughness(perceptual_roughness);
    let F0 = pbr_functions::calculate_F0(pbr_input.material.base_color.rgb, pbr_input.material.metallic, pbr_input.material.reflectance);

    // Get some random numbers. If the spatio-temporal blue noise (STBN) texture
    // is available (i.e. not the 1x1 placeholder), we use it. Otherwise, we
    // fall back to procedural noise.
    let stbn_dims = textureDimensions(stbn_texture);
    var urand: vec2<f32>;
    var raymarch_jitter: f32;
    if (all(stbn_dims > vec2(1u))) {
        let stbn_layers = textureNumLayers(stbn_texture);
        let stbn_noise = textureLoad(
            stbn_texture,
            vec2<u32>(in.position.xy) % stbn_dims,
            i32(globals.frame_count % u32(stbn_layers)),
            0
        );
        urand = stbn_noise.xy;
        // Use the third channel for jitter to avoid correlation with BRDF sampling.
        raymarch_jitter = stbn_noise.z;
    } else {
        // Fallback to PCG-based procedural noise.
        // We use a XOR-sum of products with large primes to decorrelate the
        // seed from the screen-space coordinates and frame count, avoiding
        // visible "crawling" artifacts.
        var state = (u32(in.position.x) * 2131358057u) ^
                    (u32(in.position.y) * 3416869721u) ^
                    (globals.frame_count * 1199786941u);
        urand = utils::rand_vec2f(&state);
        raymarch_jitter = utils::rand_f(&state);
    }

    // Sample the BRDF.
    let N_tangent = vec3(0.0, 0.0, 1.0);
    let V_tangent = V * tangent_to_world;
    
    let brdf_sample = sample_specular_brdf(V_tangent, roughness, F0, urand, N_tangent);
    let R_stochastic = tangent_to_world * brdf_sample.wi;
    let brdf_sample_value_over_pdf = brdf_sample.value_over_pdf;

    // Do the raymarching.
    let ssr_specular = evaluate_ssr(R_stochastic, world_position, raymarch_jitter);
    var indirect_light = (ssr_specular.rgb * brdf_sample_value_over_pdf) * fade;
    specular_occlusion = mix(specular_occlusion, specular_occlusion * ssr_specular.a, fade);

    // Sample the environment map if necessary.
    //
    // This will take the specular part of the environment map into account.
    //
    // TODO: Merge this with the duplicated code in `apply_pbr_lighting`.
#ifdef ENVIRONMENT_MAP
    // Unpack values required for environment mapping.
    let base_color = pbr_input.material.base_color.rgb;
    let metallic = pbr_input.material.metallic;
    let reflectance = pbr_input.material.reflectance;
    let specular_transmission = pbr_input.material.specular_transmission;
    let diffuse_transmission = pbr_input.material.diffuse_transmission;

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

    // Don't add stochastic noise to hits that sample the prefiltered env map.
    // The prefiltered env map already accounts for roughness.
    let R = reflect(-V, N);

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
    //
    // We pass `true` for `found_diffuse_indirect` here because we only want
    // the specular part; the diffuse part was already accumulated in the
    // main PBR pass.
    let environment_light = environment_map::environment_map_light(
        &lighting_input, &clusterable_object_index_ranges, true);

    // Accumulate the environment map light.
    indirect_light += (view.exposure * environment_light.specular * specular_occlusion) * fade;
#endif

    // Write the results.
    return vec4(fragment.rgb + indirect_light, fragment.a);
}
