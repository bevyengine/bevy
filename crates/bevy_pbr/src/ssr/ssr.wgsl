// A postprocessing pass that performs screen-space reflections.

#define_import_path bevy_pbr::ssr

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_pbr::{
    clustered_forward,
    clustered_forward::{fragment_cluster_index, unpack_clusterable_object_index_ranges},
    lighting,
    lighting::{LAYER_BASE, LAYER_CLEARCOAT, sample_visible_ggx, F_AB, perceptualRoughnessToRoughness, LightingInput},
    mesh_view_bindings,
    mesh_view_bindings::{view, depth_prepass_texture, deferred_prepass_texture, ssr_settings, globals, screen_space_ambient_occlusion_texture, light_probes},
    pbr_deferred_functions::pbr_input_from_deferred_gbuffer,
    pbr_deferred_types,
    pbr_functions,
    pbr_functions::{calculate_diffuse_color, calculate_F0},
    light_probe::query_light_probe,
    environment_map::{compute_radiances, environment_map_light_clearcoat, EnvironmentMapLight},
    prepass_utils,
    ssao_utils::ssao_multibounce,
    raymarch::{
        depth_ray_march_from_cs,
        depth_ray_march_march,
        depth_ray_march_new_from_depth,
        depth_ray_march_to_ws_dir,
    },
    utils,
    utils::interleaved_gradient_noise,
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
#import bevy_pbr::environment_map::environment_map_light
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
// * `jitter`: The jitter value for the raymarcher.
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
#ifdef DEPTH_PREPASS
    frag_coord.z = textureLoad(depth_prepass_texture, vec2<i32>(in.position.xy), 0);
#endif

    // Load the G-buffer data.
    let fragment = textureLoad(color_texture, vec2<i32>(frag_coord.xy), 0);
    let gbuffer = textureLoad(deferred_prepass_texture, vec2<i32>(frag_coord.xy), 0);
    var pbr_input = pbr_input_from_deferred_gbuffer(frag_coord, gbuffer);

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
    let ssao = textureLoad(screen_space_ambient_occlusion_texture, vec2<i32>(frag_coord.xy), 0).r;
    let ssao_multibounce = ssao_multibounce(ssao, pbr_input.material.base_color.rgb);
    pbr_input.diffuse_occlusion = min(pbr_input.diffuse_occlusion, ssao_multibounce);

    // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
    let NdotV_ao = max(dot(pbr_input.N, pbr_input.V), 0.0001);
    let roughness_ao = perceptualRoughnessToRoughness(pbr_input.material.perceptual_roughness);
    // Use SSAO to estimate the specular occlusion.
    // Lagarde and Rousiers 2014, "Moving Frostbite to Physically Based Rendering"
    pbr_input.specular_occlusion = saturate(pow(NdotV_ao + ssao, exp2(-16.0 * roughness_ao - 1.0)) - 1.0 + ssao);
#endif

    // Don't do anything if the surface is too rough, since we can't blur or do
    // temporal accumulation yet.
    let perceptual_roughness = pbr_input.material.perceptual_roughness;
    if (perceptual_roughness > ssr_settings.perceptual_roughness_threshold) {
        return fragment;
    }

    // Unpack the PBR input.
    let base_color = pbr_input.material.base_color.rgb;
    let metallic = pbr_input.material.metallic;
    let reflectance = pbr_input.material.reflectance;
    let specular_transmission = pbr_input.material.specular_transmission;
    let diffuse_transmission = pbr_input.material.diffuse_transmission;
    let diffuse_occlusion = pbr_input.diffuse_occlusion;
    let world_position = pbr_input.world_position.xyz;
    let N = pbr_input.N;
    let V = pbr_input.V;

    let roughness = lighting::perceptualRoughnessToRoughness(perceptual_roughness);

    // Do the raymarching.
    var ssr_specular = vec4(0.0);
    let noise = interleaved_gradient_noise(frag_coord.xy, globals.frame_count);

    for (var i: u32 = 0u; i < ssr_settings.samples; i = i + 1u) {
        var R = reflect(-V, N);
        if (roughness > 0.0) {
            let xi = vec2(
                fract(noise + f32(i) * 0.61803398875),
                fract(noise * 0.61803398875 + f32(i))
            );
            R = sample_visible_ggx(xi, roughness, N, V);
        }

        let sample_jitter = fract(noise + f32(i) * 0.61803398875);
        ssr_specular += evaluate_ssr(R, world_position, sample_jitter);
    }
    ssr_specular /= f32(ssr_settings.samples);

    // Calculate various values needed for both SSR weighting and environment mapping.
    let diffuse_color = calculate_diffuse_color(
        base_color,
        metallic,
        specular_transmission,
        diffuse_transmission
    );
    let NdotV = max(dot(N, V), 0.0001);
    let F_ab = F_AB(perceptual_roughness, NdotV);
    let F0 = calculate_F0(base_color, metallic, reflectance);

    // No real world material has specular values under 0.02, so we use this range as a
    // "pre-baked specular occlusion" that extinguishes the fresnel term, for artistic control.
    // See: https://google.github.io/filament/Filament.html#specularocclusion
    let specular_occlusion = saturate(dot(F0, vec3(50.0 * 0.33)));

    let Fr = max(vec3(1.0 - perceptual_roughness), F0) - F0;
    let kS = F0 + Fr * pow(1.0 - NdotV, 5.0);
    let Ess = F_ab.x + F_ab.y;
    let FssEss = kS * Ess * specular_occlusion;

    // Multiscattering approximation: https://www.jcgt.org/published/0008/01/03/paper.pdf
    // Useful reference: https://bruop.github.io/ibl
    let Ems = 1.0 - Ess;
    let Favg = F0 + (1.0 - F0) / 21.0;
    let Fms = FssEss * Favg / (1.0 - Ems * Favg);
    let FmsEms = Fms * Ems;
    let Edss = 1.0 - (FssEss + FmsEms);
    let kD = diffuse_color * Edss;

    // SSR specular part.
    //
    // Note that we don't multiply by `view.exposure` here because the sampled
    // `ssr_specular.rgb` is already exposed.
    var indirect_light = ssr_specular.rgb * FssEss * pbr_input.specular_occlusion * (1.0 - ssr_specular.a);

    // Sample the environment map if necessary.
    //
    // This will take the specular part of the environment map into account if
    // the ray missed. Otherwise, it only takes the diffuse part.
    //
    // TODO: Merge this with the duplicated code in `apply_pbr_lighting`.
    var env_specular_weight = pbr_input.specular_occlusion * ssr_specular.a;
#ifdef ENVIRONMENT_MAP
#ifdef STANDARD_MATERIAL_CLEARCOAT
    // Do the above calculations again for the clearcoat layer. Remember that
    // the clearcoat can have its own roughness and its own normal.
    let clearcoat = pbr_input.material.clearcoat;
    let clearcoat_perceptual_roughness = pbr_input.material.clearcoat_perceptual_roughness;
    let clearcoat_roughness = perceptualRoughnessToRoughness(clearcoat_perceptual_roughness);
    let clearcoat_N = pbr_input.clearcoat_N;
    let clearcoat_NdotV = max(dot(clearcoat_N, pbr_input.V), 0.0001);
    let clearcoat_R = reflect(-pbr_input.V, clearcoat_N);
#endif  // STANDARD_MATERIAL_CLEARCOAT

    // Pack all the values into a structure.
    var lighting_input: LightingInput;
    lighting_input.layers[LAYER_BASE].NdotV = NdotV;
    lighting_input.layers[LAYER_BASE].N = N;
    lighting_input.layers[LAYER_BASE].R = reflect(-V, N); // Use ideal reflection for probes
    lighting_input.layers[LAYER_BASE].perceptual_roughness = perceptual_roughness;
    lighting_input.layers[LAYER_BASE].roughness = roughness;
    lighting_input.P = world_position.xyz;
    lighting_input.V = V;
    lighting_input.diffuse_color = diffuse_color;
    lighting_input.F0_ = F0;
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
    let cluster_index = fragment_cluster_index(
        frag_coord.xy, frag_coord.z, false);
    var clusterable_object_index_ranges =
        unpack_clusterable_object_index_ranges(cluster_index);

    // Search for a reflection probe that contains the fragment.
    var query_result = query_light_probe(
        world_position,
        /*is_irradiance_volume=*/ false,
        &clusterable_object_index_ranges,
    );

    // If we didn't find a reflection probe, use the view environment map if applicable.
    if (query_result.texture_index < 0) {
        query_result.texture_index = light_probes.view_cubemap_index;
        query_result.intensity = light_probes.intensity_for_view;
        query_result.affects_lightmapped_mesh_diffuse =
            light_probes.view_environment_map_affects_lightmapped_mesh_diffuse != 0u;
    }

    if (query_result.texture_index >= 0) {
        let radiances = compute_radiances(
            lighting_input.layers[LAYER_BASE],
            &clusterable_object_index_ranges,
            world_position,
            false,
        );

        // We only add radiance (specular) if SSR missed.
        // Diffuse (irradiance) was already added in the main pass.
        indirect_light += view.exposure *
            radiances.radiance * FssEss * env_specular_weight;

#ifdef STANDARD_MATERIAL_CLEARCOAT
        // Composite clearcoat environment map lighting if present.
        var env_light: EnvironmentMapLight;
        env_light.diffuse = (FmsEms + kD) * radiances.irradiance;
        env_light.specular = radiances.radiance * FssEss;

        environment_map_light_clearcoat(
            &env_light,
            &lighting_input,
            &clusterable_object_index_ranges,
            false,
        );
        // Note: we need to handle clearcoat for SSR too, but for now we fallback to env map.
        // This is a complex area as SSR currently only traces one layer.
#endif
    }
#endif

    // Write the results.
    return vec4(fragment.rgb + indirect_light, 1.0);
}
