enable wgpu_ray_query;

#define_import_path bevy_solari::initial_path

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::{rand_f, rand_range_u}
#import bevy_render::maths::PI
#import bevy_render::utils::octahedral_encode
#import bevy_solari::brdf::{brdf_pdf, evaluate_and_sample_brdf, evaluate_brdf, F_AB}
#import bevy_solari::presample_light_tiles::unpack_resolved_light_sample
#import bevy_solari::realtime_bindings::{empty_reservoir, light_tile_resolved_samples, light_tile_samples, Reservoir, constants, view}
#import bevy_solari::sampling::{calculate_resolved_light_contribution, isinf, LightSample, NULL_LIGHT_ID, power_heuristic, trace_visibility}
#import bevy_solari::scene_bindings::{light_sources, MIRROR_ROUGHNESS_THRESHOLD, RAY_T_MAX, RAY_T_MIN, resolve_ray_hit_full, ResolvedMaterial, ResolvedRayHitFull, trace_ray}
#import bevy_solari::world_cache::{get_cell_size, query_world_cache, WORLD_CACHE_CELL_LIFETIME}
#ifdef DLSS_RR_GUIDE_BUFFERS
#import bevy_pbr::pbr_functions::{calculate_diffuse_color, calculate_F0}
#import bevy_solari::realtime_bindings::{diffuse_albedo, normal_roughness, previous_view, specular_albedo, specular_motion_vectors}
#import bevy_solari::resolve_dlss_rr_textures::env_brdf_approx2
#endif

const RECONNECTION_FOOTPRINT_KAPPA = 0.02;
const RECONNECTION_ROUGHNESS_MIN = 0.6;
const RECONNECTION_RELAX_DISTANCE = 1.0;

const CACHE_TERMINATION_MIN_SOLID_ANGLE = PI;

struct InitialSamplingResult {
    reservoir: Reservoir,
    non_resampled_radiance: vec3<f32>,
}

// Path vertices use the following convention: x0 = camera, x1 = primary ray hit (the G-buffer
// surface), x2 = first BRDF-sampled hit (the reconnection vertex).
struct PathState {
    ray_origin: vec3<f32>,
    normal: vec3<f32>,
    wo: vec3<f32>,
    material: ResolvedMaterial,
    // Throughput past x1, excluding brdf*cos at x1
    throughput_past_first_hit: vec3<f32>,
    // Reconnection vertex x2, the first BRDF-sampled hit shared by every length >= 2 candidate
    x2_position: vec3<f32>,
    x2_normal: vec3<f32>,
    // If false, candidates built on x2 are shaded directly into non_resampled_radiance instead of
    // published to the reservoir
    x2_reusable: bool,
    // brdf*cos at x1 for the direction toward x2
    x1_brdf: vec3<f32>,
}

fn generate_initial_reservoir(world_position: vec3<f32>, world_normal: vec3<f32>, material: ResolvedMaterial, workgroup_id: vec2<u32>, pixel_id: vec2<u32>, rng: ptr<function, u32>) -> InitialSamplingResult {
    var reservoir = empty_reservoir();
    reservoir.confidence_weight = 1.0;

    var non_resampled_radiance = vec3(0.0);
    var weight_sum = 0.0;
    var selected_target_function = 0.0;

#ifdef DLSS_RR_GUIDE_BUFFERS
    var mirror_rotations = reflection_matrix(world_normal);
    var psr_finished = material.roughness > MIRROR_ROUGHNESS_THRESHOLD || material.metallic <= 0.9999;
#endif

    let wo = normalize(view.world_position - world_position);
    let primary_NdotV = max(dot(world_normal, wo), 0.0001);
    let primary_F_ab = F_AB(material.perceptual_roughness, primary_NdotV);

    var path: PathState;
    path.ray_origin = world_position + (world_normal * RAY_T_MIN);
    path.normal = world_normal;
    path.wo = wo;
    path.material = material;
    path.throughput_past_first_hit = vec3(1.0);
    path.x2_position = vec3(0.0);
    path.x2_normal = vec3(0.0);
    path.x2_reusable = false;
    path.x1_brdf = vec3(0.0);

    for (var bounce = 0u; bounce < constants.max_bounces; bounce++) {
        let NdotV = max(dot(path.normal, path.wo), 0.0001);
        let F_ab = F_AB(path.material.perceptual_roughness, NdotV);

        // Stochastic NEE, with probability proportional to how diffuse the vertex is. Mirror-like
        // metals have too narrow a lobe for NEE to help, so mostly skip it there and let
        // BRDF-sampled emissive do the work. Pure dielectrics always run NEE.
        let p_nee = mix(1.0, path.material.perceptual_roughness, path.material.metallic);
        let di_samples = select(constants.secondary_di_samples, constants.primary_di_samples, bounce == 0u);
        generate_nee_candidate(&reservoir, &weight_sum, &selected_target_function, &non_resampled_radiance,
            path, F_ab, p_nee, di_samples, workgroup_id, bounce, rng);

        // Sample the BRDF and trace the next ray
        let next_bounce = evaluate_and_sample_brdf(path.wo, path.normal, path.material, F_ab, rng);
        if next_bounce.pdf == 0.0 { break; }
        let ray = trace_ray(path.ray_origin, next_bounce.wi, RAY_T_MIN, RAY_T_MAX, RAY_FLAG_NONE);
        if ray.kind == RAY_QUERY_INTERSECTION_NONE { break; }
        let ray_hit = resolve_ray_hit_full(ray);
        let p_brdf = next_bounce.pdf;

#ifdef DLSS_RR_GUIDE_BUFFERS
        if !psr_finished {
            if !isinf(p_brdf) {
                // Took the non-delta lobe, so not a mirror reflection. Keep the guide-buffer defaults.
                psr_finished = true;
            } else if ray_hit.material.roughness <= MIRROR_ROUGHNESS_THRESHOLD && ray_hit.material.metallic > 0.9999 {
                // Still in the mirror chain, fold this mirror's reflection into the chain
                mirror_rotations = mirror_rotations * reflection_matrix(ray_hit.world_normal);
            } else {
                psr_finished = true;
                replace_primary_surface(pixel_id, ray_hit, mirror_rotations, world_position);
            }
        }
#endif

        // Capture x2, the first BRDF-sampled hit
        if bounce == 0u {
            path.x2_position = ray_hit.world_position;
            path.x2_normal = ray_hit.world_normal;

            path.x1_brdf = evaluate_brdf(wo, next_bounce.wi, world_normal, material, primary_F_ab);

            path.x2_reusable = reconnection_reusable(ray.t, p_brdf, next_bounce.wi, next_bounce.diffuse_selected, ray_hit, world_position, material.perceptual_roughness, primary_NdotV);

            // The primary brdf*cos is applied at shade time, so divide it out of next_bounce.throughput
            // to leave 1/pdf (or 1/specular_weight for mirrors, avoiding the 1/INF = 0 that would kill
            // mirror GI).
            path.throughput_past_first_hit *= next_bounce.throughput / max(path.x1_brdf, vec3(0.0001));
        } else {
            // Later bounces keep the full brdf*cos/pdf for L_at_reconnection.
            path.throughput_past_first_hit *= next_bounce.throughput;
        }

        // Resample emissive hits
        if any(ray_hit.material.emissive > vec3(0.0)) && dot(ray_hit.world_normal, -next_bounce.wi) > 0.0 {
            generate_emissive_candidate(&reservoir, &weight_sum, &selected_target_function, &non_resampled_radiance,
                path, ray_hit, next_bounce.wi, p_brdf, ray.t, p_nee, di_samples, bounce, rng);
        }

        // Try terminating into the world cache
        if terminate_into_cache(&reservoir, &weight_sum, &selected_target_function, &non_resampled_radiance, path, ray_hit, ray.t, p_brdf, bounce, rng) {
            break;
        }

        // Advance to the next vertex
        path.ray_origin = ray_hit.world_position + (ray_hit.geometric_world_normal * RAY_T_MIN);
        path.normal = ray_hit.world_normal;
        path.wo = -next_bounce.wi;
        path.material = ray_hit.material;

        // Russian roulette for early termination
        if bounce > 0u {
            // throughput_past_first_hit has the primary brdf*cos divided out (so it can be re-applied at shade
            // time), which inflates it. Multiply x1_brdf back in to get the true energy-bounded path
            // throughput, which is the correct quantity for the RR survival probability.
            let full_throughput = path.throughput_past_first_hit * max(path.x1_brdf, vec3(0.0001));
            let rr = saturate(luminance(full_throughput));
            if rand_f(rng) >= rr { break; }
            path.throughput_past_first_hit /= rr;
        }
    }

    if selected_target_function > 0.0 {
        reservoir.unbiased_contribution_weight = weight_sum / selected_target_function;
    }

    return InitialSamplingResult(reservoir, non_resampled_radiance);
}

fn generate_nee_candidate(
    reservoir: ptr<function, Reservoir>,
    weight_sum: ptr<function, f32>,
    selected_target_function: ptr<function, f32>,
    non_resampled_radiance: ptr<function, vec3<f32>>,
    path: PathState,
    F_ab: vec2<f32>,
    p_nee: f32,
    di_samples: u32,
    workgroup_id: vec2<u32>,
    bounce: u32,
    rng: ptr<function, u32>,
) {
    if rand_f(rng) >= p_nee { return; }

    let di = sample_light_ris(path.ray_origin, path.normal, path.wo, path.material, F_ab, di_samples, workgroup_id, bounce, rng);
    let di_target_function = luminance(di.brdf_radiance);
    if di_target_function <= 0.0 { return; }

    // MIS against the BRDF strategy. RIS over N candidates makes the effective NEE pdf at the
    // winner roughly N * light_pdf(winner), so scale by p_nee for the stochastic gate.
    var nee_mis_weight = 1.0;
    if di.brdf_rays_can_hit && di.inverse_solid_angle_pdf > 0.0 {
        let p_nee_strategy = f32(di_samples) * (1.0 / di.inverse_solid_angle_pdf) * p_nee;
        let p_brdf_at_nee = brdf_pdf(path.wo, di.wi, path.normal, path.material, F_ab);
        nee_mis_weight = power_heuristic(p_nee_strategy, p_brdf_at_nee);
    }

    if bounce == 0u {
        // Bounce 0: Candidate is the light sample, stored by reference and re-resolved each frame
        // nee_mis_weight goes into the target function since it gets recomputed per-pixel during reuse
        let target_function = di_target_function * nee_mis_weight;
        let resampling_weight = target_function * di.unbiased_contribution_weight / p_nee;

        *weight_sum += resampling_weight;
        if rand_f(rng) * (*weight_sum) < resampling_weight {
            (*reservoir).light_sample = di.light_sample;
            *selected_target_function = target_function;
        }
    } else {
        // Deeper bounces: Candidate is the reconnection radiance at x2
        let L_at_reconnection = path.throughput_past_first_hit * di.brdf_radiance * di.unbiased_contribution_weight * nee_mis_weight / p_nee;
        if !path.x2_reusable {
            // x1 -> x2 not reuse-safe: shade directly at this pixel instead of publishing.
            *non_resampled_radiance += path.x1_brdf * L_at_reconnection;
        } else {
            let target_function = luminance(path.x1_brdf * L_at_reconnection);
            let resampling_weight = target_function;

            *weight_sum += resampling_weight;
            if rand_f(rng) * (*weight_sum) < resampling_weight {
                (*reservoir).light_sample = LightSample(NULL_LIGHT_ID, 0u);
                (*reservoir).sample_point_world_position = path.x2_position;
                (*reservoir).sample_point_world_normal = octahedral_encode(path.x2_normal);
                (*reservoir).radiance = L_at_reconnection;
                *selected_target_function = target_function;
            }
        }
    }
}

struct DiSample {
    unbiased_contribution_weight: f32,
    light_sample: LightSample,
    wi: vec3<f32>,
    brdf_radiance: vec3<f32>,
    inverse_solid_angle_pdf: f32,
    brdf_rays_can_hit: bool,
}

fn sample_light_ris(ray_origin: vec3<f32>, normal: vec3<f32>, wo: vec3<f32>, material: ResolvedMaterial, F_ab: vec2<f32>, di_samples: u32, workgroup_id: vec2<u32>, bounce: u32, rng: ptr<function, u32>) -> DiSample {
    var workgroup_rng = (workgroup_id.x * 5782582u) + workgroup_id.y + bounce;
    let light_tile_start = rand_range_u(128u, &workgroup_rng) * 1024u;

    var weight_sum = 0.0;
    var selected_target_function = 0.0;
    var selected_tile_sample = 0u;
    var selected_world_position = vec4(0.0);
    var selected_wi = vec3(0.0);
    var selected_brdf_radiance = vec3(0.0);
    var selected_inverse_solid_angle_pdf = 0.0;
    var selected_brdf_rays_can_hit = false;
    let mis_weight = 1.0 / f32(di_samples);
    for (var i = 0u; i < di_samples; i++) {
        let tile_sample = light_tile_start + rand_range_u(1024u, rng);
        let resolved_light_sample = unpack_resolved_light_sample(light_tile_resolved_samples[tile_sample], view.exposure);
        let light_contribution = calculate_resolved_light_contribution(resolved_light_sample, ray_origin, normal);
        let brdf_current = evaluate_brdf(wo, light_contribution.wi, normal, material, F_ab);
        let brdf_radiance = brdf_current * light_contribution.radiance;

        let target_function = luminance(brdf_radiance);
        let resampling_weight = mis_weight * (target_function * light_contribution.inverse_pdf);

        weight_sum += resampling_weight;

        if rand_f(rng) * weight_sum < resampling_weight {
            selected_target_function = target_function;
            selected_tile_sample = tile_sample;
            selected_world_position = resolved_light_sample.world_position;
            selected_wi = light_contribution.wi;
            selected_inverse_solid_angle_pdf = light_contribution.inverse_solid_angle_pdf;
            selected_brdf_rays_can_hit = light_contribution.brdf_rays_can_hit;
            selected_brdf_radiance = brdf_radiance;
        }
    }

    var unbiased_contribution_weight = 0.0;
    if selected_target_function > 0.0 {
        unbiased_contribution_weight = weight_sum / selected_target_function;
        unbiased_contribution_weight *= trace_visibility(ray_origin, selected_world_position);
    }

    return DiSample(unbiased_contribution_weight, light_tile_samples[selected_tile_sample], selected_wi, selected_brdf_radiance, selected_inverse_solid_angle_pdf, selected_brdf_rays_can_hit);
}

fn generate_emissive_candidate(
    reservoir: ptr<function, Reservoir>,
    weight_sum: ptr<function, f32>,
    selected_target_function: ptr<function, f32>,
    non_resampled_radiance: ptr<function, vec3<f32>>,
    path: PathState,
    ray_hit: ResolvedRayHitFull,
    wi: vec3<f32>,
    p_brdf: f32,
    ray_t: f32,
    p_nee: f32,
    di_samples: u32,
    bounce: u32,
    rng: ptr<function, u32>,
) {
    let NdotV_hit = max(dot(ray_hit.world_normal, -wi), 0.0001);
    let light_count = arrayLength(&light_sources);
    let area_pdf = 1.0 / (f32(light_count) * f32(ray_hit.triangle_count) * ray_hit.triangle_area);
    let p_light = area_pdf * ray_t * ray_t / NdotV_hit;
    let emissive_mis_weight = power_heuristic(p_brdf, p_light * p_nee * f32(di_samples));

    if !path.x2_reusable {
        // x1 -> x2 not reuse-safe (mirror/sharp lobe or failed gate): shade directly at this pixel
        // instead of publishing, since a reuse shift would waste it or make a firefly. Mirror lobes
        // always land here (p_brdf = INF, footprint 0), where emissive_mis_weight is 1.
        *non_resampled_radiance += path.x1_brdf * path.throughput_past_first_hit * ray_hit.material.emissive * emissive_mis_weight;
        return;
    }

    if bounce == 0u {
        // Bounce 0: Candidate is the emissive hit
        let target_function = luminance(path.x1_brdf * ray_hit.material.emissive) * emissive_mis_weight;
        let resampling_weight = luminance(path.x1_brdf * path.throughput_past_first_hit * ray_hit.material.emissive) * emissive_mis_weight;

        *weight_sum += resampling_weight;
        if rand_f(rng) * (*weight_sum) < resampling_weight {
            (*reservoir).light_sample = LightSample(NULL_LIGHT_ID, bitcast<u32>(area_pdf));
            (*reservoir).sample_point_world_position = path.x2_position;
            (*reservoir).sample_point_world_normal = octahedral_encode(path.x2_normal);
            (*reservoir).radiance = ray_hit.material.emissive;
            *selected_target_function = target_function;
        }
    } else {
        // Deeper bounces: Candidate is the reconnection radiance at x2
        let emissive_L_at_reconnection = path.throughput_past_first_hit * ray_hit.material.emissive * emissive_mis_weight;
        let target_function = luminance(path.x1_brdf * emissive_L_at_reconnection);
        let resampling_weight = target_function;

        *weight_sum += resampling_weight;
        if rand_f(rng) * (*weight_sum) < resampling_weight {
            (*reservoir).light_sample = LightSample(NULL_LIGHT_ID, 0u);
            (*reservoir).sample_point_world_position = path.x2_position;
            (*reservoir).sample_point_world_normal = octahedral_encode(path.x2_normal);
            (*reservoir).radiance = emissive_L_at_reconnection;
            *selected_target_function = target_function;
        }
    }
}

fn terminate_into_cache(
    reservoir: ptr<function, Reservoir>,
    weight_sum: ptr<function, f32>,
    selected_target_function: ptr<function, f32>,
    non_resampled_radiance: ptr<function, vec3<f32>>,
    path: PathState,
    ray_hit: ResolvedRayHitFull,
    ray_t: f32,
    p_brdf: f32,
    bounce: u32,
    rng: ptr<function, u32>,
) -> bool {
    // Only terminate into the world cache when the bounce was from a wide-enough BRDF sample
    // because the cache is less noisy than continuing the path for rough surfaces,
    // but less accurate for smooth surfaces
    let lobe_solid_angle = 1.0 / p_brdf;
    let broad_enough_to_terminate = lobe_solid_angle >= CACHE_TERMINATION_MIN_SOLID_ANGLE;
    let forced_terminate = bounce == constants.max_bounces - 1u;
    if !(broad_enough_to_terminate || forced_terminate) { return false; }

    // Only use the cache when the ray cleared the cache cell (diagonal = sqrt(3) * cell_size). Short
    // rays land in a cell that may straddle occluders and leak light through corners.
    var rng_copy = *rng;
    let world_cache_cell_size = get_cell_size(ray_hit.world_position, view.world_position, ray_t, &rng_copy);
    if ray_t <= sqrt(3.0) * world_cache_cell_size { return false; }

    let cached_radiance = query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal, view.world_position, ray_t, WORLD_CACHE_CELL_LIFETIME, rng);

    let cache_outgoing = (ray_hit.material.base_color / PI) * cached_radiance;
    let cache_L_at_reconnection = path.throughput_past_first_hit * cache_outgoing;
    if !path.x2_reusable {
        *non_resampled_radiance += path.x1_brdf * cache_L_at_reconnection;
        return true;
    }

    let target_function = luminance(path.x1_brdf * cache_L_at_reconnection);
    let resampling_weight = target_function;
    *weight_sum += resampling_weight;
    if rand_f(rng) * (*weight_sum) < resampling_weight {
        (*reservoir).light_sample = LightSample(NULL_LIGHT_ID, 0u);
        (*reservoir).sample_point_world_position = path.x2_position;
        (*reservoir).sample_point_world_normal = octahedral_encode(path.x2_normal);
        (*reservoir).radiance = cache_L_at_reconnection;
        *selected_target_function = target_function;
    }

    return true;
}

// ReSTIR PT Enhanced: Algorithmic Advances for Faster and More Robust ReSTIR Path Tracing
// Section 4 (sorta)
// https://research.nvidia.com/labs/rtr/publication/lin2026restirptenhanced/lin2026restirptenhanced.pdf
fn reconnection_reusable(ray_t: f32, p_brdf: f32, wi: vec3<f32>, diffuse_selected: bool, ray_hit: ResolvedRayHitFull, world_position: vec3<f32>, x1_perceptual_roughness: f32, primary_NdotV: f32) -> bool {
    // ray_footprint = t^2 / (p_brdf * cos_x2) is the area a sample represents at x2. It goes to 0 for
    // mirror lobes (p_brdf = INF) and shrinks for sharp lobes or short segments. Compared against a
    // uniform 1/(4*PI) primary footprint, so the test trades roughness against distance.
    let cos_x2 = max(dot(ray_hit.world_normal, -wi), 0.0001);
    let ray_footprint = (ray_t * ray_t) / (p_brdf * cos_x2);
    let primary_dist = length(view.world_position - world_position);
    let primary_footprint = 4.0 * PI * primary_dist * primary_dist / primary_NdotV;
    let footprint_ok = ray_footprint >= (RECONNECTION_FOOTPRINT_KAPPA / 100.0) * primary_footprint;

    // Roughness floor at x1, only for specular lobes (a diffuse bounce is always rough). Guards
    // low-roughness specular lobes that resample with poorly-conditioned MIS/jacobian. The footprint
    // test alone is too permissive here.
    let x1_lobe_ok = diffuse_selected || x1_perceptual_roughness >= RECONNECTION_ROUGHNESS_MIN;

    // Guard at x2. A sharp reflector there makes the stored radiance view-dependent and wrong to
    // reuse from a neighbor's direction. The roughness floor relaxes with segment length: a distant
    // glossy x2 is seen by neighbors from nearly the same direction, so the view-dependence washes out.
    // Diffuse, rough, and emissive vertices are always reuse-safe.
    let x2_is_light = any(ray_hit.material.emissive > vec3(0.0));
    let x2_roughness = mix(1.0, ray_hit.material.perceptual_roughness, ray_hit.material.metallic);
    let x2_roughness_floor = RECONNECTION_ROUGHNESS_MIN * saturate(RECONNECTION_RELAX_DISTANCE / ray_t);
    let x2_end_ok = x2_is_light || x2_roughness >= x2_roughness_floor;

    return footprint_ok && x1_lobe_ok && x2_end_ok;
}

#ifdef DLSS_RR_GUIDE_BUFFERS
// https://en.wikipedia.org/wiki/Householder_transformation
fn reflection_matrix(plane_normal: vec3<f32>) -> mat3x3<f32> {
    // N times Nᵀ
    let n_nt = mat3x3<f32>(
        plane_normal * plane_normal.x,
        plane_normal * plane_normal.y,
        plane_normal * plane_normal.z,
    );
    let identity_matrix = mat3x3<f32>(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
    return identity_matrix - n_nt * 2.0;
}

// Primary surface replacement for perfect mirrors. Follow the reflection chain to the first
// non-mirror hit and write its attributes, reflected into the mirror's virtual space, to the
// DLSS RR guide buffers so the denoiser treats this pixel as directly seeing that surface.
// https://developer.nvidia.com/blog/rendering-perfect-reflections-and-refractions-in-path-traced-games/#primary_surface_replacement
fn replace_primary_surface(pixel_id: vec2<u32>, ray_hit: ResolvedRayHitFull, mirror_rotations: mat3x3<f32>, primary_surface_world_position: vec3<f32>) {
    // Approximation: apply the whole chain's rotations around the first mirror, not each around its own
    let virtual_position = (mirror_rotations * (ray_hit.world_position - primary_surface_world_position)) + primary_surface_world_position;
    let virtual_previous_frame_position = (mirror_rotations * (ray_hit.previous_frame_world_position - primary_surface_world_position)) + primary_surface_world_position;
    let specular_motion_vector = calculate_motion_vector(virtual_position, virtual_previous_frame_position);

    let F0 = calculate_F0(ray_hit.material.base_color, ray_hit.material.metallic, vec3(ray_hit.material.reflectance));
    let wo = normalize(view.world_position - virtual_position);
    let virtual_normal = normalize(mirror_rotations * ray_hit.world_normal);

    textureStore(specular_motion_vectors, pixel_id, vec4(specular_motion_vector, vec2(0.0)));
    textureStore(diffuse_albedo, pixel_id, vec4(calculate_diffuse_color(ray_hit.material.base_color, ray_hit.material.metallic, 0.0, 0.0), 0.0));
    textureStore(specular_albedo, pixel_id, vec4(env_brdf_approx2(F0, ray_hit.material.roughness, virtual_normal, wo), 0.0));
    textureStore(normal_roughness, pixel_id, vec4(virtual_normal, ray_hit.material.perceptual_roughness));
}

fn calculate_motion_vector(world_position: vec3<f32>, previous_world_position: vec3<f32>) -> vec2<f32> {
    let clip_position_t = view.unjittered_clip_from_world * vec4(world_position, 1.0);
    let clip_position = clip_position_t.xy / clip_position_t.w;
    let previous_clip_position_t = previous_view.unjittered_clip_from_world * vec4(previous_world_position, 1.0);
    let previous_clip_position = previous_clip_position_t.xy / previous_clip_position_t.w;
    // Motion vectors are UV-space offsets in [-1, 1], from one corner to the diagonally-opposite one.
    // A clip-space diagonal difference is in [-2, 2], so scale by 0.5, and flip y since V goes down
    // where clip-space y goes up.
    return (clip_position - previous_clip_position) * vec2(0.5, -0.5);
}
#endif
