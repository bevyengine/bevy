import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance;
import bevy_pbr::render::utils::{rand_f, rand_range_u};
import bevy_render::maths::PI;
import bevy_render::utils::octahedral_encode;
import bevy_solari::scene::brdf::{brdf_pdf, evaluate_and_sample_brdf, evaluate_brdf, F_AB};
@if(DLSS_RR_GUIDE_BUFFERS)
import bevy_solari::scene::brdf::{evaluate_specular_brdf, lobe_reflectances};
import bevy_solari::realtime::presample_light_tiles::unpack_resolved_light_sample;
import bevy_solari::realtime::bindings::{empty_reservoir, light_tile_resolved_samples, light_tile_samples, Reservoir, constants, view, SPECULAR_GUIDE_ROUGHNESS_THRESHOLD};
import bevy_solari::scene::sampling::{calculate_resolved_light_contribution, isinf, LightSample, NULL_LIGHT_ID, power_heuristic, trace_visibility};
import bevy_solari::scene::bindings::{light_sources, MIRROR_ROUGHNESS_THRESHOLD, RAY_T_MAX, RAY_T_MIN, resolve_ray_hit_full, ResolvedMaterial, ResolvedRayHitFull, trace_ray};
import bevy_solari::realtime::world_cache_query::{get_cell_size, query_world_cache, WORLD_CACHE_CELL_LIFETIME};
@if(DLSS_RR_GUIDE_BUFFERS)
import bevy_pbr::render::pbr_functions::{calculate_diffuse_color, calculate_F0, calculate_F0_dielectric};
@if(DLSS_RR_GUIDE_BUFFERS)
import bevy_pbr::deferred::types::unpack_24bit_normal;
@if(DLSS_RR_GUIDE_BUFFERS)
import bevy_render::utils::octahedral_decode;
@if(DLSS_RR_GUIDE_BUFFERS)
import bevy_solari::realtime::bindings::{diffuse_albedo, dlss_rr_depth, dlss_rr_motion_vectors, gbuffer, normal_roughness, previous_view, specular_albedo, specular_motion_vectors};
@if(DLSS_RR_GUIDE_BUFFERS)
import bevy_solari::realtime::resolve_dlss_rr_textures::env_brdf_approx2;

enable wgpu_ray_query;

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

    let wo = normalize(view.world_position - world_position);

    @if(DLSS_RR_GUIDE_BUFFERS)
    var psr = psr_init(world_position, world_normal, material, pixel_id);

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
        if ray.kind == RAY_QUERY_INTERSECTION_NONE {
            @if(DLSS_RR_GUIDE_BUFFERS)
            // A mirror ray that leaves the scene still has something true to say: the reflection
            // is the environment, effectively at infinity. Saying so beats leaving this pixel the
            // surface's own motion vector, which claims the reflection is painted on and slides
            // with the mirror.
            if !psr.finished && !next_bounce.diffuse_selected {
                psr_environment_miss(pixel_id, psr);
            }
            break;
        }
        let ray_hit = resolve_ray_hit_full(ray);
        let p_brdf = next_bounce.pdf;

        @if(DLSS_RR_GUIDE_BUFFERS)
        if !psr.finished {
            if next_bounce.diffuse_selected {
                // The diffuse lobe was drawn, so this frame has no reflection distance to report.
                // Keep the guide-buffer defaults - which for a glossy pixel means keeping whatever
                // `resolve_dlss_rr_textures` deliberately left in place, see the note there.
                psr.finished = true;
            } else {
                psr.throughput *= psr_bounce_reflectance(path.wo, next_bounce.wi, path.normal, path.material, F_ab, isinf(p_brdf));
                psr.path_length += ray.t;

                // A glossy surface never chains. Its ray is one draw from a lobe with real width, and
                // following a mirror on from it would describe a reflection of a reflection with a
                // direction that was already only a sample. One bounce, then describe what it found.
                if !psr.glossy && is_delta_mirror(ray_hit.material) {
                    // Still in the mirror chain, fold this mirror's reflection into the chain
                    psr.mirror_rotations = psr.mirror_rotations * reflection_matrix(ray_hit.world_normal);
                } else {
                    psr.finished = true;
                    replace_primary_surface(pixel_id, psr, ray_hit, material, world_normal, wo, primary_F_ab);
                }
            }
        }

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

// State the mirror chain carries alongside the path walk. Bundled into a struct so the whole thing
// costs one `@if` at the declaration site instead of one per variable.
struct PsrState {
    // Set once the chain has ended, whether it wrote guide buffers or gave up. Nothing below runs after.
    finished: bool,
    // The primary surface's lobe has measurable width, so its reflection is an integral rather than a
    // direction. Decided once from the surface alone: re-deriving it per bounce invites the two halves
    // of the walk to disagree.
    glossy: bool,
    // Whether this pixel's surface is being swapped out for the reflected one, or is only having its
    // reflection's motion described. Keyed on the primary surface only, and false for anything glossy.
    //
    // This is the whole difference in what gets written: true means every guide buffer, false means
    // the specular motion vector and nothing else. There is no partial case - see the early return in
    // `replace_primary_surface` for why a half-replaced pixel is worse than an unreplaced one.
    replace_fully: bool,
    // Product of each mirror's Householder reflection, in chain order. Only used to rotate the
    // terminating surface's normal and its frame-to-frame motion into the mirror's virtual space -
    // the virtual *position* comes from the path length instead, see `replace_primary_surface`.
    mirror_rotations: mat3x3<f32>,
    // Distance from the camera to the primary surface.
    primary_distance: f32,
    // Total distance travelled so far, camera through every mirror. Equals `primary_distance` until
    // the chain takes its first bounce.
    path_length: f32,
    // Direction from the camera to the primary surface. The unfolded path is a straight line along it.
    primary_direction: vec3<f32>,
    // Product of the Fresnel term at each mirror in the chain: what the reflected surface's colour is
    // multiplied by on its way back to the eye.
    throughput: vec3<f32>,
    // Screen-space rate of change of the shading normal at the primary surface, in radians per pixel.
    curvature: f32,
}

// A surface is replaced outright only when nearly all of its reflectance leaves through the delta
// lobe. Below that the chain would fire on so few frames that the guide buffers would flicker between
// the mirror and its reflection, which is worse than never replacing at all.
//
// Replaces the old `metallic > 0.9999` test: what matters is not whether a surface is a metal but
// whether its mirror lobe carries the energy, which is a question a polished dielectric can also
// answer yes to.
const FULL_REPLACEMENT_FRACTION = 0.9;

// How far a reflector's virtual image may shift between neighbouring pixels, as a fraction of the
// virtual distance itself, before the surface is treated as too curved to replace.
//
// Being a ratio is the point. An angle-per-pixel threshold silently changes meaning with render
// resolution, which is untenable when DLSS renders below output resolution and the quality mode would
// then decide which surfaces get replaced.
const MAX_VIRTUAL_DEPTH_JITTER = 0.002;

// Even a perfectly flat surface measures a small nonzero normal change, because G-buffer normals are
// 24-bit octahedral and neighbouring texels round differently. Subtract that floor: without it a flat
// mirror reflecting something far away fails the curvature test purely on encoding noise, since the
// jitter term scales with reflection length.
const NORMAL_QUANTIZATION_FLOOR = 0.002;

// Distance, in metres, standing in for "infinitely far" when a mirror ray finds nothing.
//
// Large enough that camera translation moves the image by well under a pixel at any sane scene scale,
// so what remains is the rotation term - which is the whole content of a distant reflection's motion.
// Not larger, because the virtual position still goes through a `clip_from_world` multiply, and
// pushing it out to 1e30 spends float precision to say the same thing.
const ENVIRONMENT_VIRTUAL_DISTANCE = 10000.0;

// Whether a surface's delta lobe carries enough of its reflectance to stand in for the pixel.
//
// Evaluated at normal incidence on purpose. Specular reflectance climbs toward 1 at grazing angles, so
// a view-dependent test would flip a polished floor between replaced and not as the camera moved,
// reintroducing exactly the frame-to-frame guide instability this is meant to remove. At normal
// incidence the answer is a property of the material and cannot change under motion.
@if(DLSS_RR_GUIDE_BUFFERS)
fn is_delta_mirror(material: ResolvedMaterial) -> bool {
    return material.roughness <= MIRROR_ROUGHNESS_THRESHOLD
        && delta_fraction(material, F_AB(material.perceptual_roughness, 1.0)) >= FULL_REPLACEMENT_FRACTION;
}

// What the reflection is multiplied by at one bounce, on its way back to the eye.
//
// Two different quantities depending on the lobe, and this is the distinction that makes a glossy
// chain safe. For a delta lobe `evaluate_specular_brdf` returns exactly the Fresnel term, a
// reflectance in 0..1. For a lobe with width it returns the full `D * V * F * NdotL`, which is a
// *density* - at alpha 0.05 the GGX peak is around 130, and multiplying an albedo guide by that
// saturates it to white. The hemispherical reflectance is the right factor there, and it is already
// what `lobe_reflectances` computes for the sampling weights.
@if(DLSS_RR_GUIDE_BUFFERS)
fn psr_bounce_reflectance(wo: vec3<f32>, wi: vec3<f32>, normal: vec3<f32>, material: ResolvedMaterial, F_ab: vec2<f32>, delta: bool) -> vec3<f32> {
    if delta {
        return evaluate_specular_brdf(wo, wi, normal, material, F_ab);
    }
    return lobe_reflectances(material.base_color, calculate_F0_dielectric(vec3(material.reflectance)), material, F_ab).specular;
}

// Share of a surface's total reflectance that leaves through the specular lobe, in 0..1.
@if(DLSS_RR_GUIDE_BUFFERS)
fn delta_fraction(material: ResolvedMaterial, F_ab: vec2<f32>) -> f32 {
    let rho = lobe_reflectances(material.base_color, calculate_F0_dielectric(vec3(material.reflectance)), material, F_ab);
    let specular_luminance = luminance(rho.specular);
    return specular_luminance / max(specular_luminance + luminance(rho.diffuse), 0.0001);
}

@if(DLSS_RR_GUIDE_BUFFERS)
fn psr_init(world_position: vec3<f32>, world_normal: vec3<f32>, material: ResolvedMaterial, pixel_id: vec2<u32>) -> PsrState {
    var psr: PsrState;
    psr.mirror_rotations = reflection_matrix(world_normal);
    let camera_to_primary = world_position - view.world_position;
    psr.primary_distance = length(camera_to_primary);
    psr.path_length = psr.primary_distance;
    psr.primary_direction = camera_to_primary / psr.primary_distance;
    psr.throughput = vec3(1.0);
    psr.curvature = 0.0;

    // Eligibility is roughness alone, at the loose threshold. What the chain is then allowed to *do*
    // with the result is the separate question `replace_fully` answers - and that one keeps the strict
    // delta test, because a virtual position is only meaningful for a lobe that is one direction.
    //
    // Splitting these is the whole point. A polished dielectric floor puts about 4% of its energy
    // through the specular lobe, so it can never pass `is_delta_mirror`; asking that question about it
    // at all was the reason glossy surfaces got nothing.
    psr.finished = material.roughness > SPECULAR_GUIDE_ROUGHNESS_THRESHOLD;
    psr.glossy = material.roughness > MIRROR_ROUGHNESS_THRESHOLD;
    psr.replace_fully = !psr.glossy && is_delta_mirror(material);
    // Only the replacement path is gated on curvature, so only it pays for the two G-buffer loads.
    if psr.replace_fully {
        psr.curvature = reflector_curvature(pixel_id, world_normal);
    }
    return psr;
}

@if(DLSS_RR_GUIDE_BUFFERS)
fn load_gbuffer_normal(pixel_id: vec2<u32>) -> vec3<f32> {
    return octahedral_decode(unpack_24bit_normal(textureLoad(gbuffer, pixel_id, 0).a));
}

// Angle, in radians, that the shading normal turns over one pixel - a screen-space stand-in for
// curvature. Noise floor removed, so a flat surface reads as exactly zero.
//
// Shading normals rather than a geometric normal reconstructed from depth: reflections follow the
// interpolated shading normal, so that is what has to be measured. The cost is that normal-mapped
// surface detail reads as some curvature.
//
// Also fires at object boundaries, where the neighbour sits on unrelated geometry. That costs a
// one-pixel border around every mirror and errs toward not replacing, the safe way to be wrong.
@if(DLSS_RR_GUIDE_BUFFERS)
fn reflector_curvature(pixel_id: vec2<u32>, normal: vec3<f32>) -> f32 {
    let max_pixel_id = vec2<u32>(view.main_pass_viewport.zw) - vec2(1u);
    let right = load_gbuffer_normal(min(pixel_id + vec2(1u, 0u), max_pixel_id));
    let down = load_gbuffer_normal(min(pixel_id + vec2(0u, 1u), max_pixel_id));

    // For unit vectors |a - b| is the angle between them to first order, and |a - b|^2 = 2(1 - a.b).
    let min_cos = min(dot(normal, right), dot(normal, down));
    let angle = sqrt(max(0.0, 2.0 * (1.0 - min_cos)));
    return max(0.0, angle - NORMAL_QUANTIZATION_FLOOR);
}

// The chain's ray left the scene, so the reflection is the environment and sits effectively at
// infinity. That is a real answer, not a failure: a distant image does not parallax, so it shifts only
// with camera rotation, and saying so is much better than the surface motion vector this pixel would
// otherwise keep - which claims the reflection is painted on and slides with the mirror.
//
// Only the motion vector is written. There is no reflected surface to describe, so the albedo and
// normal guides keep the mirror's own, which is what the default pass already put there.
@if(DLSS_RR_GUIDE_BUFFERS)
fn psr_environment_miss(pixel_id: vec2<u32>, psr: PsrState) {
    let far_position = view.world_position + (psr.primary_direction * ENVIRONMENT_VIRTUAL_DISTANCE);
    textureStore(specular_motion_vectors, pixel_id, vec4(calculate_motion_vector(far_position, far_position), vec2(0.0)));
}

// https://en.wikipedia.org/wiki/Householder_transformation
@if(DLSS_RR_GUIDE_BUFFERS)
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

@if(DLSS_RR_GUIDE_BUFFERS)
// Primary surface replacement for perfect mirrors. The reflection chain has been followed to its
// first non-mirror hit; write that surface's attributes, reflected into the mirror's virtual space,
// to the DLSS RR guide buffers so the denoiser treats this pixel as directly seeing it.
// https://developer.nvidia.com/blog/rendering-perfect-reflections-and-refractions-in-path-traced-games/#primary_surface_replacement
fn replace_primary_surface(
    pixel_id: vec2<u32>,
    psr: PsrState,
    ray_hit: ResolvedRayHitFull,
    primary_material: ResolvedMaterial,
    primary_normal: vec3<f32>,
    primary_wo: vec3<f32>,
    primary_F_ab: vec2<f32>,
) {
    // A normal turning by some angle turns the reflected ray by twice that, and the virtual image sits
    // at the reflection distance, so neighbouring pixels' virtual images differ by roughly
    // 2 * curvature * reflection_length. Once that is a meaningful fraction of the distance itself the
    // denoiser reads a shattered surface and blurs instead of reprojecting, which is worse than never
    // having replaced the surface at all.
    //
    // Only knowable here, after the chain has run: the same reflector is fine looking at something
    // close and unusable looking at something far, and curvature alone cannot tell those apart.
    //
    // Glossy pixels are exempt, because the gate is about *depth*. All they write is a specular
    // motion vector, which the denoiser applies to an already-blurry signal with no
    // neighbour-to-neighbour continuity requirement. Refusing them would cost every curved polished
    // surface its motion vector to protect a buffer those pixels never touch.
    let reflection_length = psr.path_length - psr.primary_distance;
    if psr.replace_fully && 2.0 * psr.curvature * reflection_length > MAX_VIRTUAL_DEPTH_JITTER * psr.path_length { return; }

    // Unfolding a specular path about a chain of planes straightens it into a single straight ray from
    // the camera, so the virtual image sits along the primary direction at the total path length.
    // Exact for planar mirrors at any chain length, and only a scalar to carry - where reflecting the
    // hit position about the first mirror's plane is exact for one mirror and drifts for two, because
    // later mirrors reflect about their own plane.
    //
    // Curvature is deliberately ignored: a convex reflector's image really sits nearer than this.
    // NVIDIA's own DLSS-RR sample does the same, and NRD notes it costs reprojection artefacts on
    // curved surfaces rather than breaking outright - which is what the gate above bounds.
    let virtual_position = view.world_position + (psr.primary_direction * psr.path_length);

    // The position comes from path length, so only the object's frame-to-frame *motion* needs the
    // chain applied to it. Mirroring the delta is what makes a moving reflection track properly; note
    // this assumes the mirrors themselves are static, since a moving mirror would drag the whole
    // virtual world with it.
    let world_motion = ray_hit.previous_frame_world_position - ray_hit.world_position;
    let virtual_previous_position = virtual_position + (psr.mirror_rotations * world_motion);
    let specular_motion_vector = calculate_motion_vector(virtual_position, virtual_previous_position);

    // The one guide every eligible pixel gets, replaced or not: where the reflection is heading.
    textureStore(specular_motion_vectors, pixel_id, vec4(specular_motion_vector, vec2(0.0)));

    // And for a glossy surface that is *all* it gets. Everything below describes a pixel as though it
    // were a direct view of the reflected surface, which is only true when the lobe is a delta.
    //
    // Anything short of that is still mostly itself, and the guides have to agree with each other or
    // they are worse than none. Writing the reflection's albedo while keeping the reflector's normal
    // told the denoiser "this pixel's reflectance is that distant wall, and its normal is this floor"
    // - two statements about different surfaces, from which nothing coherent can be reconstructed.
    // A glossy metal made that worst, since its specular fraction pins at 1 and the albedo went over
    // entirely to the reflection while the normal never moved at all.
    //
    // So no blend. The motion vector is the one thing that is unambiguously true about a glossy
    // reflection - it says how the blurry highlight travels, which is a statement about the reflection
    // alone and needs no agreement with the normal or the depth to be useful.
    if !psr.replace_fully {
        return;
    }

    let wo = normalize(view.world_position - virtual_position);
    let virtual_normal = normalize(psr.mirror_rotations * ray_hit.world_normal);

    // Depth of the reflection rather than of the mirror, so the denoiser reprojects it at its true
    // optical distance, paired with the motion vector that describes that same reflection.
    //
    // Jittered `clip_from_world` here to match the prepass depth that `resolve_dlss_rr_textures`
    // copies for every other pixel, while `calculate_motion_vector` uses the unjittered matrices -
    // also matching the prepass convention. Mixing the two would make replaced pixels disagree with
    // their neighbours by a subpixel offset that changes every frame.
    {
        let virtual_clip_position = view.clip_from_world * vec4(virtual_position, 1.0);
        textureStore(dlss_rr_depth, pixel_id, vec4(virtual_clip_position.z / virtual_clip_position.w));
        textureStore(dlss_rr_motion_vectors, pixel_id, vec4(specular_motion_vector, vec2(0.0)));
    }

    // The albedo guides are documented as this pixel's reflectance, and that is exactly what this is:
    // the chain's tint times what the chain found, plus whatever the mirror reflects on its own
    // account. The tint is what makes a gold mirror's reflection read as gold rather than as the wall
    // behind it, and for a polished dielectric it is also why the reflection is only a few percent of
    // the pixel. NVIDIA's DLSS-RR sample applies the same product to both albedo guides.
    //
    // Still a blend rather than an outright swap, because `FULL_REPLACEMENT_FRACTION` admits surfaces
    // at 0.9: a polished dielectric right at the boundary keeps a tenth of its own reflectance, and
    // dropping that would make it read as a perfect mirror to the denoiser when it is not one.
    let reflected_diffuse = calculate_diffuse_color(ray_hit.material.base_color, ray_hit.material.metallic, 0.0, 0.0);
    let reflected_F0 = calculate_F0(ray_hit.material.base_color, ray_hit.material.metallic, vec3(ray_hit.material.reflectance));
    let reflected_specular = env_brdf_approx2(reflected_F0, ray_hit.material.roughness, virtual_normal, wo);

    let own_diffuse = calculate_diffuse_color(primary_material.base_color, primary_material.metallic, 0.0, 0.0);
    let own_F0 = calculate_F0(primary_material.base_color, primary_material.metallic, vec3(primary_material.reflectance));
    let own_specular = env_brdf_approx2(own_F0, primary_material.roughness, primary_normal, primary_wo);

    // View-dependent here, unlike the replacement decision: this is a reflectance being reported, not
    // a branch that has to stay stable under camera motion.
    let specular_fraction = delta_fraction(primary_material, primary_F_ab);

    textureStore(diffuse_albedo, pixel_id, vec4(psr.throughput * reflected_diffuse + own_diffuse, 0.0));
    // The mirror's own specular reflectance is already the chain's first factor, so adding it back
    // would count it twice - only the reflected surface's own specular term is new here.
    textureStore(specular_albedo, pixel_id, vec4(mix(own_specular, psr.throughput * reflected_specular, specular_fraction), 0.0));
    // Normals cannot be summed the way reflectance can, and averaging a floor's normal with a
    // reflected object's produces one that describes neither - so this is a swap, not a blend, and
    // only where the reflection really is what this pixel shows.
    //
    textureStore(normal_roughness, pixel_id, vec4(virtual_normal, ray_hit.material.perceptual_roughness));
}

@if(DLSS_RR_GUIDE_BUFFERS)
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
