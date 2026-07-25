// https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf
enable wgpu_ray_query;

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::{rand_f, rand_u, sample_disk}
#import bevy_render::utils::octahedral_decode
#import bevy_solari::brdf::{brdf_pdf, evaluate_brdf, F_AB}
#import bevy_solari::gbuffer_utils::{gpixel_resolve, permute_pixel, pixel_dissimilar}
#import bevy_solari::initial_path::{generate_initial_reservoir, InitialSamplingResult}
#import bevy_solari::realtime_bindings::{depth_buffer, empty_reservoir, gbuffer, motion_vectors, previous_depth_buffer, previous_gbuffer, previous_view, reservoirs_a, reservoirs_b, Reservoir, constants, view, view_output}
#import bevy_solari::sampling::{balance_heuristic, calculate_resolved_light_contribution, isinf, isnan, LightSample, NULL_LIGHT_ID, power_heuristic, resolve_light_sample, ResolvedLightSample, trace_visibility, trace_visibility_previous_frame}
#import bevy_solari::scene_bindings::{light_sources, LIGHT_NOT_PRESENT_THIS_FRAME, previous_frame_light_id_translations, RAY_T_MAX, RAY_T_MIN, ResolvedMaterial}
#import bevy_solari::world_cache::{query_world_cache, WORLD_CACHE_CELL_LIFETIME}

const SPATIAL_REUSE_RADIUS_PIXELS = 30.0;

@compute @workgroup_size(8, 8, 1)
fn initial_and_temporal(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.main_pass_viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.main_pass_viewport.z);
    var rng = pixel_index + constants.frame_rng;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        reservoirs_b[pixel_index] = empty_reservoir();
        return;
    }
    let surface = gpixel_resolve(textureLoad(gbuffer, global_id.xy, 0), depth, global_id.xy, view.main_pass_viewport.zw, view.world_from_clip);

    let initial = generate_initial_reservoir(surface.world_position, surface.world_normal, surface.material, workgroup_id.xy, global_id.xy, &rng);
    textureStore(view_output, global_id.xy, vec4(initial.non_resampled_radiance, 0.0));

    let temporal = load_temporal_reservoir(global_id.xy, depth, surface.world_position, surface.world_normal);
    let previous_camera_homogeneous = previous_view.world_from_clip * (previous_view.clip_from_view * vec4(0.0, 0.0, 0.0, 1.0));
    let previous_camera_world_position = previous_camera_homogeneous.xyz / previous_camera_homogeneous.w;
    let merge_result = merge_reservoirs(initial.reservoir, surface.world_position, surface.world_normal, surface.material,
        temporal.reservoir, temporal.world_position, temporal.world_normal, temporal.material, previous_camera_world_position, &rng);

    reservoirs_b[pixel_index] = merge_result.merged_reservoir;
}

@compute @workgroup_size(8, 8, 1)
fn spatial_and_shade(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.main_pass_viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.main_pass_viewport.z);
    var rng = pixel_index + constants.frame_rng + 0x6A09E667u;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        reservoirs_a[pixel_index] = empty_reservoir();
        return;
    }
    let surface = gpixel_resolve(textureLoad(gbuffer, global_id.xy, 0), depth, global_id.xy, view.main_pass_viewport.zw, view.world_from_clip);

    let input_reservoir = reservoirs_b[pixel_index];
    let wo = normalize(view.world_position - surface.world_position);
    let NdotV = max(dot(surface.world_normal, wo), 0.0001);
    let F_ab = F_AB(surface.material.perceptual_roughness, NdotV);

    let spatial = load_spatial_reservoir(global_id.xy, depth, surface.world_position, surface.world_normal, &rng);
    let merge_result = merge_reservoirs(input_reservoir, surface.world_position, surface.world_normal, surface.material,
        spatial.reservoir, spatial.world_position, spatial.world_normal, spatial.material, view.world_position, &rng);

    reservoirs_a[pixel_index] = merge_result.merged_reservoir;

    var pixel_color = merge_result.selected_sample_brdf_radiance * merge_result.merged_reservoir.unbiased_contribution_weight;
    pixel_color += surface.material.emissive;
    pixel_color += textureLoad(view_output, global_id.xy).rgb;
    pixel_color *= view.exposure;
    textureStore(view_output, global_id.xy, vec4(pixel_color, 1.0));

#ifdef VISUALIZE_WORLD_CACHE
    textureStore(view_output, global_id.xy, vec4(query_world_cache(surface.world_position, surface.world_normal, view.world_position, RAY_T_MAX, WORLD_CACHE_CELL_LIFETIME, &rng) * view.exposure, 1.0));
#endif
}

fn load_temporal_reservoir(pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>) -> NeighborInfo {
    if bool(constants.reset) {
        return NeighborInfo(empty_reservoir(), vec3(0.0), vec3(0.0), empty_material());
    }

    let motion_vector = textureLoad(motion_vectors, pixel_id, 0).xy;
    let temporal_pixel_id_float = round(vec2<f32>(pixel_id) - (motion_vector * view.main_pass_viewport.zw));

    // If reprojection lands off-screen, fall back to this pixel's own previous reservoir rather than
    // dropping history. The dissimilarity check below still validates the surface, and a same-pixel
    // guess that passes it beats restarting from a confidence-1 reservoir at the screen edge.
    var point_temporal_pixel_id = pixel_id;
    if all(temporal_pixel_id_float >= vec2(0.0)) && all(temporal_pixel_id_float < view.main_pass_viewport.zw) {
        point_temporal_pixel_id = vec2<u32>(temporal_pixel_id_float);
    }

    var permute_rng = constants.frame_rng;
    let permuted_temporal_pixel_id = permute_pixel(point_temporal_pixel_id, rand_u(&permute_rng), view.main_pass_viewport.zw);

    // Check if the pixel features have changed heavily between the current and previous frame
    let temporal_depth = textureLoad(previous_depth_buffer, permuted_temporal_pixel_id, 0);
    let temporal_surface = gpixel_resolve(textureLoad(previous_gbuffer, permuted_temporal_pixel_id, 0), temporal_depth, permuted_temporal_pixel_id, view.main_pass_viewport.zw, previous_view.world_from_clip);
    if pixel_dissimilar(depth, world_position, temporal_surface.world_position, world_normal, temporal_surface.world_normal, view) {
        return NeighborInfo(empty_reservoir(), vec3(0.0), vec3(0.0), empty_material());
    }

    let temporal_pixel_index = permuted_temporal_pixel_id.x + permuted_temporal_pixel_id.y * u32(view.main_pass_viewport.z);
    var temporal = NeighborInfo(reservoirs_a[temporal_pixel_index], temporal_surface.world_position, temporal_surface.world_normal, temporal_surface.material);

    // Check if the light selected in the previous frame no longer exists in the current frame (e.g. entity despawned)
    if temporal.reservoir.light_sample.light_id != NULL_LIGHT_ID {
        let previous_light_id = temporal.reservoir.light_sample.light_id >> 16u;
        let triangle_id = temporal.reservoir.light_sample.light_id & 0xFFFFu;
        let light_id = previous_frame_light_id_translations[previous_light_id];
        if light_id == LIGHT_NOT_PRESENT_THIS_FRAME {
            return NeighborInfo(empty_reservoir(), vec3(0.0), vec3(0.0), empty_material());
        }
        temporal.reservoir.light_sample.light_id = (light_id << 16u) | triangle_id;
    }

    temporal.reservoir.confidence_weight = min(temporal.reservoir.confidence_weight, constants.confidence_weight_cap);

    return temporal;
}

fn load_spatial_reservoir(pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>, rng: ptr<function, u32>) -> NeighborInfo {
    for (var i = 0u; i < 5u; i++) {
        let spatial_pixel_id = get_neighbor_pixel_id(pixel_id, SPATIAL_REUSE_RADIUS_PIXELS, rng);

        if all(spatial_pixel_id == pixel_id) {
            continue;
        }

        let spatial_depth = textureLoad(depth_buffer, spatial_pixel_id, 0);
        let spatial_surface = gpixel_resolve(textureLoad(gbuffer, spatial_pixel_id, 0), spatial_depth, spatial_pixel_id, view.main_pass_viewport.zw, view.world_from_clip);
        if pixel_dissimilar(depth, world_position, spatial_surface.world_position, world_normal, spatial_surface.world_normal, view) {
            continue;
        }

        let spatial_pixel_index = spatial_pixel_id.x + spatial_pixel_id.y * u32(view.main_pass_viewport.z);
        return NeighborInfo(reservoirs_b[spatial_pixel_index], spatial_surface.world_position, spatial_surface.world_normal, spatial_surface.material);
    }

    return NeighborInfo(empty_reservoir(), world_position, world_normal, empty_material());
}

fn get_neighbor_pixel_id(center_pixel_id: vec2<u32>, search_radius: f32, rng: ptr<function, u32>) -> vec2<u32> {
    var spatial_id = vec2<f32>(center_pixel_id) + sample_disk(search_radius, rng);
    spatial_id = clamp(spatial_id, vec2(0.0), view.main_pass_viewport.zw - 1.0);
    return vec2<u32>(spatial_id);
}

struct NeighborInfo {
    reservoir: Reservoir,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    material: ResolvedMaterial,
}

fn empty_material() -> ResolvedMaterial {
    return ResolvedMaterial(vec3(0.0), vec3(0.0), 0.0, 0.0, 0.0, 0.0);
}

fn jacobian(
    new_world_position: vec3<f32>,
    original_world_position: vec3<f32>,
    sample_point_world_position: vec3<f32>,
    sample_point_world_normal: vec3<f32>,
) -> f32 {
    let r = new_world_position - sample_point_world_position;
    let q = original_world_position - sample_point_world_position;
    let rl = length(r);
    let ql = length(q);
    let phi_r = saturate(dot(r / rl, sample_point_world_normal));
    let phi_q = saturate(dot(q / ql, sample_point_world_normal));
    let jacobian = (phi_r * ql * ql) / (phi_q * rl * rl);
    return select(jacobian, 0.0, isinf(jacobian) || isnan(jacobian));
}

struct ReservoirMergeResult {
    merged_reservoir: Reservoir,
    selected_sample_brdf_radiance: vec3<f32>,
}

fn merge_reservoirs(
    canonical_reservoir: Reservoir,
    canonical_world_position: vec3<f32>,
    canonical_world_normal: vec3<f32>,
    canonical_material: ResolvedMaterial,
    other_reservoir: Reservoir,
    other_world_position: vec3<f32>,
    other_world_normal: vec3<f32>,
    other_material: ResolvedMaterial,
    other_view_position: vec3<f32>,
    rng: ptr<function, u32>,
) -> ReservoirMergeResult {
    var canonical_resolved: ResolvedLightSample;
    if canonical_reservoir.light_sample.light_id != NULL_LIGHT_ID {
        canonical_resolved = resolve_light_sample(canonical_reservoir.light_sample, light_sources[canonical_reservoir.light_sample.light_id >> 16u]);
    }

    let canonical_wo = normalize(view.world_position - canonical_world_position);
    let canonical_NdotV = max(dot(canonical_world_normal, canonical_wo), 0.0001);
    let canonical_F_ab = F_AB(canonical_material.perceptual_roughness, canonical_NdotV);
    let canonical_sample_at_canonical = reservoir_contribution(canonical_reservoir, canonical_resolved, canonical_world_position, canonical_world_normal, canonical_wo, canonical_material, canonical_F_ab);

    // Skip resampling empty reservoirs
    if other_reservoir.confidence_weight == 0.0 {
        return ReservoirMergeResult(canonical_reservoir, canonical_sample_at_canonical.brdf_radiance);
    }

    var other_resolved: ResolvedLightSample;
    if other_reservoir.light_sample.light_id != NULL_LIGHT_ID {
        other_resolved = resolve_light_sample(other_reservoir.light_sample, light_sources[other_reservoir.light_sample.light_id >> 16u]);
    }
    let other_wo = normalize(other_view_position - other_world_position);
    let other_NdotV = max(dot(other_world_normal, other_wo), 0.0001);
    let other_F_ab = F_AB(other_material.perceptual_roughness, other_NdotV);

    // Contributions for resampling and MIS
    var other_sample_at_canonical = reservoir_contribution(other_reservoir, other_resolved, canonical_world_position, canonical_world_normal, canonical_wo, canonical_material, canonical_F_ab);
    var canonical_sample_at_other = reservoir_contribution(canonical_reservoir, canonical_resolved, other_world_position, other_world_normal, other_wo, other_material, other_F_ab);
    let other_sample_at_other = reservoir_contribution(other_reservoir, other_resolved, other_world_position, other_world_normal, other_wo, other_material, other_F_ab);

    // Jacobians for resampling and MIS. Light samples don't need a reprojection jacobian,
    // since calculate_resolved_light_contribution already accounts for the shading point's geometry.
    var other_sample_at_canonical_jacobian = 1.0;
    if other_reservoir.light_sample.light_id == NULL_LIGHT_ID {
        other_sample_at_canonical_jacobian = jacobian(
            canonical_world_position,
            other_world_position,
            other_reservoir.sample_point_world_position,
            octahedral_decode(other_reservoir.sample_point_world_normal)
        );
    }
    var canonical_sample_at_other_jacobian = 1.0;
    if canonical_reservoir.light_sample.light_id == NULL_LIGHT_ID {
        canonical_sample_at_other_jacobian = jacobian(
            other_world_position,
            canonical_world_position,
            canonical_reservoir.sample_point_world_position,
            octahedral_decode(canonical_reservoir.sample_point_world_normal)
        );
    }

    // Don't merge samples with huge jacobians, as it explodes the variance
    if other_sample_at_canonical_jacobian < 0.125 || other_sample_at_canonical_jacobian > 8.0 {
        other_sample_at_canonical_jacobian = 0.0;
    }
    if canonical_sample_at_other_jacobian < 0.125 || canonical_sample_at_other_jacobian > 8.0 {
        canonical_sample_at_other_jacobian = 0.0;
    }

    // Visibility for the cross-domain targets
    if other_sample_at_canonical.target_function > 0.0 && other_sample_at_canonical_jacobian > 0.0 {
        let visibility = trace_visibility(canonical_world_position + canonical_world_normal * RAY_T_MIN, other_sample_at_canonical.sample_world_position);
        other_sample_at_canonical.target_function *= visibility;
    }
    if canonical_sample_at_other.target_function > 0.0 && canonical_sample_at_other_jacobian > 0.0 {
#ifdef SPATIAL_MERGE
        let visibility = trace_visibility(other_world_position + other_world_normal * RAY_T_MIN, canonical_sample_at_other.sample_world_position);
#else
        let visibility = trace_visibility_previous_frame(other_world_position + other_world_normal * RAY_T_MIN, canonical_sample_at_other.sample_world_position);
#endif
        canonical_sample_at_other.target_function *= visibility;
    }

    // Defensive balance heuristic MIS (for spatial reuse only)
    let total_confidence_weight = canonical_reservoir.confidence_weight + other_reservoir.confidence_weight;
    var defensive_t_c = 0.0;
#ifdef SPATIAL_MERGE
    defensive_t_c = select(1.0, canonical_reservoir.confidence_weight / total_confidence_weight, total_confidence_weight > 0.0);
#endif

    // Resampling weight for canonical sample
    let canonical_balance_mis_weight = balance_heuristic(
        canonical_reservoir.confidence_weight * canonical_sample_at_canonical.target_function,
        other_reservoir.confidence_weight * canonical_sample_at_other.target_function * canonical_sample_at_other_jacobian,
    );
    let canonical_sample_mis_weight = mix(canonical_balance_mis_weight, 1.0, defensive_t_c);
    let canonical_sample_resampling_weight = canonical_sample_mis_weight * canonical_sample_at_canonical.target_function * canonical_reservoir.unbiased_contribution_weight;

    // Resampling weight for other sample
    let other_balance_mis_weight = balance_heuristic(
        other_reservoir.confidence_weight * other_sample_at_other.target_function,
        canonical_reservoir.confidence_weight * other_sample_at_canonical.target_function * other_sample_at_canonical_jacobian,
    );
    let other_sample_mis_weight = mix(other_balance_mis_weight, 0.0, defensive_t_c);
    let other_sample_resampling_weight = other_sample_mis_weight * other_sample_at_canonical.target_function * other_reservoir.unbiased_contribution_weight * other_sample_at_canonical_jacobian;

    // Perform resampling
    var combined_reservoir = empty_reservoir();
    combined_reservoir.confidence_weight = canonical_reservoir.confidence_weight + other_reservoir.confidence_weight;
    let weight_sum = canonical_sample_resampling_weight + other_sample_resampling_weight;

    if rand_f(rng) * weight_sum < other_sample_resampling_weight {
        combined_reservoir.sample_point_world_position = other_reservoir.sample_point_world_position;
        combined_reservoir.sample_point_world_normal = other_reservoir.sample_point_world_normal;
        combined_reservoir.radiance = other_reservoir.radiance;
        combined_reservoir.light_sample = other_reservoir.light_sample;

        let inverse_target_function = select(0.0, 1.0 / other_sample_at_canonical.target_function, other_sample_at_canonical.target_function > 0.0);
        combined_reservoir.unbiased_contribution_weight = weight_sum * inverse_target_function;

        return ReservoirMergeResult(combined_reservoir, other_sample_at_canonical.brdf_radiance);
    } else {
        combined_reservoir.sample_point_world_position = canonical_reservoir.sample_point_world_position;
        combined_reservoir.sample_point_world_normal = canonical_reservoir.sample_point_world_normal;
        combined_reservoir.radiance = canonical_reservoir.radiance;
        combined_reservoir.light_sample = canonical_reservoir.light_sample;

        let inverse_target_function = select(0.0, 1.0 / canonical_sample_at_canonical.target_function, canonical_sample_at_canonical.target_function > 0.0);
        combined_reservoir.unbiased_contribution_weight = weight_sum * inverse_target_function;

        return ReservoirMergeResult(combined_reservoir, canonical_sample_at_canonical.brdf_radiance);
    }
}

struct ReservoirContribution {
    brdf_radiance: vec3<f32>,
    target_function: f32,
    sample_world_position: vec4<f32>,
}

fn reservoir_contribution(reservoir: Reservoir, resolved: ResolvedLightSample, world_position: vec3<f32>, world_normal: vec3<f32>, wo: vec3<f32>, material: ResolvedMaterial, F_ab: vec2<f32>) -> ReservoirContribution {
    if reservoir.light_sample.light_id != NULL_LIGHT_ID {
        let light_contribution = calculate_resolved_light_contribution(resolved, world_position, world_normal);

        // MIS weight against the bounce-0 BRDF-emissive strategy, recomputed from this surface's
        // brdf and material rather than baked into the unbiased contribution weight at generation. Mirrors the bounce-0
        // nee_mis_weight in generate_nee_candidate and generate_emissive_candidate, which puts the same factor in the target.
        var nee_mis_weight = 1.0;
        if light_contribution.brdf_rays_can_hit && light_contribution.inverse_solid_angle_pdf > 0.0 {
            let light_count = arrayLength(&light_sources);
            let inverse_solid_angle_pdf = light_contribution.inverse_solid_angle_pdf * f32(light_count);
            let p_nee = mix(1.0, material.perceptual_roughness, material.metallic);
            let p_nee_strategy = f32(constants.primary_di_samples) * (1.0 / inverse_solid_angle_pdf) * p_nee;
            let p_brdf_at_nee = brdf_pdf(wo, light_contribution.wi, world_normal, material, F_ab);
            nee_mis_weight = power_heuristic(p_nee_strategy, p_brdf_at_nee);
        }

        let brdf_radiance = light_contribution.radiance * evaluate_brdf(wo, light_contribution.wi, world_normal, material, F_ab) * nee_mis_weight;
        return ReservoirContribution(brdf_radiance, luminance(brdf_radiance), resolved.world_position);
    } else if any(reservoir.radiance != vec3(0.0)) {
        let delta = reservoir.sample_point_world_position - (world_position + world_normal * RAY_T_MIN);
        let sample_distance = length(delta);
        let wi = delta / sample_distance;
        var brdf_radiance = reservoir.radiance * evaluate_brdf(wo, wi, world_normal, material, F_ab);

        // Bounce-0 BRDF-emissive sample (directly-visible light). The seed field carries the light
        // triangle's bitcast area pdf, and the stored radiance is the raw emission. Rebuild the MIS
        // weight against this surface's NEE strategy, the dual of nee_mis_weight above and a mirror
        // of the emissive candidate in generate_initial_reservoir.
        if reservoir.light_sample.seed != 0u {
            let area_pdf = bitcast<f32>(reservoir.light_sample.seed);
            let light_normal = octahedral_decode(reservoir.sample_point_world_normal);
            let cos_theta_light = dot(-wi, light_normal);
            if cos_theta_light <= 0.0 {
                brdf_radiance = vec3(0.0);
            } else {
                let p_light = area_pdf * sample_distance * sample_distance / cos_theta_light;
                let p_nee = mix(1.0, material.perceptual_roughness, material.metallic);
                let p_brdf = brdf_pdf(wo, wi, world_normal, material, F_ab);
                brdf_radiance *= power_heuristic(p_brdf, p_light * p_nee * f32(constants.primary_di_samples));
            }
        }

        return ReservoirContribution(brdf_radiance, luminance(brdf_radiance), vec4(reservoir.sample_point_world_position, 1.0));
    } else {
        return ReservoirContribution(vec3(0.0), 0.0, vec4(reservoir.sample_point_world_position, 1.0));
    }
}
