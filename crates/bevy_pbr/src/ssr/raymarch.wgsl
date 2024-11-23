// Copyright (c) 2023 Tomasz Stachowiak
//
// This contribution is dual licensed under EITHER OF
//
//     Apache License, Version 2.0, (http://www.apache.org/licenses/LICENSE-2.0)
//     MIT license (http://opensource.org/licenses/MIT)
//
// at your option.
//
// This is a port of the original [`raymarch.hlsl`] to WGSL. It's deliberately
// kept as close as possible so that patches to the original `raymarch.hlsl`
// have the greatest chances of applying to this version.
//
// [`raymarch.hlsl`]:
// https://gist.github.com/h3r2tic/9c8356bdaefbe80b1a22ae0aaee192db

#define_import_path bevy_pbr::raymarch

#import bevy_pbr::mesh_view_bindings::depth_prepass_texture
#import bevy_pbr::view_transformations::{
    direction_world_to_clip,
    ndc_to_uv,
    perspective_camera_near,
    position_world_to_ndc,
}

// Allows us to sample from the depth buffer with bilinear filtering.
@group(1) @binding(2) var depth_linear_sampler: sampler;

// Allows us to sample from the depth buffer with nearest-neighbor filtering.
@group(1) @binding(3) var depth_nearest_sampler: sampler;

// Main code

struct HybridRootFinder {
    linear_steps: u32,
    bisection_steps: u32,
    use_secant: bool,
    linear_march_exponent: f32,

    jitter: f32,
    min_t: f32,
    max_t: f32,
}

fn hybrid_root_finder_new_with_linear_steps(v: u32) -> HybridRootFinder {
    var res: HybridRootFinder;
    res.linear_steps = v;
    res.bisection_steps = 0u;
    res.use_secant = false;
    res.linear_march_exponent = 1.0;
    res.jitter = 1.0;
    res.min_t = 0.0;
    res.max_t = 1.0;
    return res;
}

fn hybrid_root_finder_find_root(
    root_finder: ptr<function, HybridRootFinder>,
    start: vec3<f32>,
    end: vec3<f32>,
    distance_fn: ptr<function, DepthRaymarchDistanceFn>,
    hit_t: ptr<function, f32>,
    miss_t: ptr<function, f32>,
    hit_d: ptr<function, DistanceWithPenetration>,
) -> bool {
    let dir = end - start;

    var min_t = (*root_finder).min_t;
    var max_t = (*root_finder).max_t;

    var min_d = DistanceWithPenetration(0.0, false, 0.0);
    var max_d = DistanceWithPenetration(0.0, false, 0.0);

    let step_size = (max_t - min_t) / f32((*root_finder).linear_steps);

    var intersected = false;

    //
    // Ray march using linear steps

    if ((*root_finder).linear_steps > 0u) {
        let candidate_t = mix(
            min_t,
            max_t,
            pow(
                (*root_finder).jitter / f32((*root_finder).linear_steps),
                (*root_finder).linear_march_exponent
            )
        );

        let candidate = start + dir * candidate_t;
        let candidate_d = depth_raymarch_distance_fn_evaluate(distance_fn, candidate);
        intersected = candidate_d.distance < 0.0 && candidate_d.valid;

        if (intersected) {
            max_t = candidate_t;
            max_d = candidate_d;
            // The `[min_t .. max_t]` interval contains an intersection. End the linear search.
        } else {
            // No intersection yet. Carry on.
            min_t = candidate_t;
            min_d = candidate_d;

            for (var step = 1u; step < (*root_finder).linear_steps; step += 1u) {
                let candidate_t = mix(
                    (*root_finder).min_t,
                    (*root_finder).max_t,
                    pow(
                        (f32(step) + (*root_finder).jitter) / f32((*root_finder).linear_steps),
                        (*root_finder).linear_march_exponent
                    )
                );

                let candidate = start + dir * candidate_t;
                let candidate_d = depth_raymarch_distance_fn_evaluate(distance_fn, candidate);
                intersected = candidate_d.distance < 0.0 && candidate_d.valid;

                if (intersected) {
                    max_t = candidate_t;
                    max_d = candidate_d;
                    // The `[min_t .. max_t]` interval contains an intersection.
                    // End the linear search.
                    break;
                } else {
                    // No intersection yet. Carry on.
                    min_t = candidate_t;
                    min_d = candidate_d;
                }
            }
        }
    }

    *miss_t = min_t;
    *hit_t = min_t;

    //
    // Refine the hit using bisection

    if (intersected) {
        for (var step = 0u; step < (*root_finder).bisection_steps; step += 1u) {
            let mid_t = (min_t + max_t) * 0.5;
            let candidate = start + dir * mid_t;
            let candidate_d = depth_raymarch_distance_fn_evaluate(distance_fn, candidate);

            if (candidate_d.distance < 0.0 && candidate_d.valid) {
                // Intersection at the mid point. Refine the first half.
                max_t = mid_t;
                max_d = candidate_d;
            } else {
                // No intersection yet at the mid point. Refine the second half.
                min_t = mid_t;
                min_d = candidate_d;
            }
        }

        if ((*root_finder).use_secant) {
            // Finish with one application of the secant method
            let total_d = min_d.distance + -max_d.distance;

            let mid_t = mix(min_t, max_t, min_d.distance / total_d);
            let candidate = start + dir * mid_t;
            let candidate_d = depth_raymarch_distance_fn_evaluate(distance_fn, candidate);

            // Only accept the result of the secant method if it improves upon
            // the previous result.
            //
            // Technically root_finder should be `abs(candidate_d.distance) <
            // min(min_d.distance, -max_d.distance) * frac`, but root_finder seems
            // sufficient.
            if (abs(candidate_d.distance) < min_d.distance * 0.9 && candidate_d.valid) {
                *hit_t = mid_t;
                *hit_d = candidate_d;
            } else {
                *hit_t = max_t;
                *hit_d = max_d;
            }

            return true;
        } else {
            *hit_t = max_t;
            *hit_d = max_d;
            return true;
        }
    } else {
        // Mark the conservative miss distance.
        *hit_t = min_t;
        return false;
    }
}

struct DistanceWithPenetration {
    /// Distance to the surface of which a root we're trying to find
    distance: f32,

    /// Whether to consider this sample valid for intersection.
    /// Mostly relevant for allowing the ray marcher to travel behind surfaces,
    /// as it will mark surfaces it travels under as invalid.
    valid: bool,

    /// Conservative estimate of depth to which the ray penetrates the marched surface.
    penetration: f32,
}

struct DepthRaymarchDistanceFn {
    depth_tex_size: vec2<f32>,

    march_behind_surfaces: bool,
    depth_thickness: f32,

    use_sloppy_march: bool,
}

fn depth_raymarch_distance_fn_evaluate(
    distance_fn: ptr<function, DepthRaymarchDistanceFn>,
    ray_point_cs: vec3<f32>,
) -> DistanceWithPenetration {
    let interp_uv = ndc_to_uv(ray_point_cs.xy);

    let ray_depth = 1.0 / ray_point_cs.z;

    // We're using both point-sampled and bilinear-filtered values from the depth buffer.
    //
    // That's really stupid but works like magic. For samples taken near the ray origin,
    // the discrete nature of the depth buffer becomes a problem. It's not a land of continuous surfaces,
    // but a bunch of stacked duplo bricks.
    //
    // Technically we should be taking discrete steps in distance_fn duplo land, but then we're at the mercy
    // of arbitrary quantization of our directions -- and sometimes we'll take a step which would
    // claim that the ray is occluded -- even though the underlying smooth surface wouldn't occlude it.
    //
    // If we instead take linear taps from the depth buffer, we reconstruct the linear surface.
    // That fixes acne, but introduces false shadowing near object boundaries, as we now pretend
    // that everything is shrink-wrapped by distance_fn continuous 2.5D surface, and our depth thickness
    // heuristic ends up falling apart.
    //
    // The fix is to consider both the smooth and the discrete surfaces, and only claim occlusion
    // when the ray descends below both.
    //
    // The two approaches end up fixing each other's artifacts:
    // * The false occlusions due to duplo land are rejected because the ray stays above the smooth surface.
    // * The shrink-wrap surface is no longer continuous, so it's possible for rays to miss it.

    let linear_depth =
        1.0 / textureSampleLevel(depth_prepass_texture, depth_linear_sampler, interp_uv, 0.0);
    let unfiltered_depth =
        1.0 / textureSampleLevel(depth_prepass_texture, depth_nearest_sampler, interp_uv, 0.0);

    var max_depth: f32;
    var min_depth: f32;

    if ((*distance_fn).use_sloppy_march) {
        max_depth = unfiltered_depth;
        min_depth = unfiltered_depth;
    } else {
        max_depth = max(linear_depth, unfiltered_depth);
        min_depth = min(linear_depth, unfiltered_depth);
    }

    let bias = 0.000002;

    var res: DistanceWithPenetration;
    res.distance = max_depth * (1.0 + bias) - ray_depth;

    // distance_fn will be used at the end of the ray march to potentially discard the hit.
    res.penetration = ray_depth - min_depth;

    if ((*distance_fn).march_behind_surfaces) {
        res.valid = res.penetration < (*distance_fn).depth_thickness;
    } else {
        res.valid = true;
    }

    return res;
}

struct DepthRayMarchResult {
    /// True if the raymarch hit something.
    hit: bool,

    /// In case of a hit, the normalized distance to it.
    ///
    /// In case of a miss, the furthest the ray managed to travel, which could either be
    /// exceeding the max range, or getting behind a surface further than the depth thickness.
    ///
    /// Range: `0..=1` as a lerp factor over `ray_start_cs..=ray_end_cs`.
    hit_t: f32,

    /// UV correspindong to `hit_t`.
    hit_uv: vec2<f32>,

    /// The distance that the hit point penetrates into the hit surface.
    /// Will normally be non-zero due to limited precision of the ray march.
    ///
    /// In case of a miss: undefined.
    hit_penetration: f32,

    /// Ditto, within the range `0..DepthRayMarch::depth_thickness_linear_z`
    ///
    /// In case of a miss: undefined.
    hit_penetration_frac: f32,
}

struct DepthRayMarch {
    /// Number of steps to be taken at regular intervals to find an initial intersection.
    /// Must not be zero.
    linear_steps: u32,

    /// Exponent to be applied in the linear part of the march.
    ///
    /// A value of 1.0 will result in equidistant steps, and higher values will compress
    /// the earlier steps, and expand the later ones. This might be desirable in order
    /// to get more detail close to objects in SSR or SSGI.
    ///
    /// For optimal performance, this should be a small compile-time unsigned integer,
    /// such as 1 or 2.
    linear_march_exponent: f32,

    /// Number of steps in a bisection (binary search) to perform once the linear search
    /// has found an intersection. Helps narrow down the hit, increasing the chance of
    /// the secant method finding an accurate hit point.
    ///
    /// Useful when sampling color, e.g. SSR or SSGI, but pointless for contact shadows.
    bisection_steps: u32,

    /// Approximate the root position using the secant method -- by solving for line-line
    /// intersection between the ray approach rate and the surface gradient.
    ///
    /// Useful when sampling color, e.g. SSR or SSGI, but pointless for contact shadows.
    use_secant: bool,

    /// Jitter to apply to the first step of the linear search; 0..=1 range, mapping
    /// to the extent of a single linear step in the first phase of the search.
    /// Use 1.0 if you don't want jitter.
    jitter: f32,

    /// Clip space coordinates (w=1) of the ray.
    ray_start_cs: vec3<f32>,
    ray_end_cs: vec3<f32>,

    /// Should be used for contact shadows, but not for any color bounce, e.g. SSR.
    ///
    /// For SSR etc. this can easily create leaks, but with contact shadows it allows the rays
    /// to pass over invalid occlusions (due to thickness), and find potentially valid ones ahead.
    ///
    /// Note that this will cause the linear search to potentially miss surfaces,
    /// because when the ray overshoots and ends up penetrating a surface further than
    /// `depth_thickness_linear_z`, the ray marcher will just carry on.
    ///
    /// For this reason, this may require a lot of samples, or high depth thickness,
    /// so that `depth_thickness_linear_z >= world space ray length / linear_steps`.
    march_behind_surfaces: bool,

    /// If `true`, the ray marcher only performs nearest lookups of the depth buffer,
    /// resulting in aliasing and false occlusion when marching tiny detail.
    /// It should work fine for longer traces with fewer rays though.
    use_sloppy_march: bool,

    /// When marching the depth buffer, we only have 2.5D information, and don't know how
    /// thick surfaces are. We shall assume that the depth buffer fragments are little squares
    /// with a constant thickness defined by this parameter.
    depth_thickness_linear_z: f32,

    /// Size of the depth buffer we're marching in, in pixels.
    depth_tex_size: vec2<f32>,
}

fn depth_ray_march_new_from_depth(depth_tex_size: vec2<f32>) -> DepthRayMarch {
    var res: DepthRayMarch;
    res.jitter = 1.0;
    res.linear_steps = 4u;
    res.bisection_steps = 0u;
    res.linear_march_exponent = 1.0;
    res.depth_tex_size = depth_tex_size;
    res.depth_thickness_linear_z = 1.0;
    res.march_behind_surfaces = false;
    res.use_sloppy_march = false;
    return res;
}

fn depth_ray_march_to_cs_dir_impl(
    raymarch: ptr<function, DepthRayMarch>,
    dir_cs: vec4<f32>,
    infinite: bool,
) {
    var end_cs = vec4((*raymarch).ray_start_cs, 1.0) + dir_cs;

    // Perform perspective division, but avoid dividing by zero for rays
    // heading directly towards the eye.
    end_cs /= select(-1.0, 1.0, end_cs.w >= 0.0) * max(1e-10, abs(end_cs.w));

    // Clip ray start to the view frustum
    var delta_cs = end_cs.xyz - (*raymarch).ray_start_cs;
    let near_edge = select(vec3(-1.0, -1.0, 0.0), vec3(1.0, 1.0, 1.0), delta_cs < vec3(0.0));
    let dist_to_near_edge = (near_edge - (*raymarch).ray_start_cs) / delta_cs;
    let max_dist_to_near_edge = max(dist_to_near_edge.x, dist_to_near_edge.y);
    (*raymarch).ray_start_cs += delta_cs * max(0.0, max_dist_to_near_edge);

    // Clip ray end to the view frustum

    delta_cs = end_cs.xyz - (*raymarch).ray_start_cs;
    let far_edge = select(vec3(-1.0, -1.0, 0.0), vec3(1.0, 1.0, 1.0), delta_cs >= vec3(0.0));
    let dist_to_far_edge = (far_edge - (*raymarch).ray_start_cs) / delta_cs;
    let min_dist_to_far_edge = min(
        min(dist_to_far_edge.x, dist_to_far_edge.y),
        dist_to_far_edge.z
    );

    if (infinite) {
        delta_cs *= min_dist_to_far_edge;
    } else {
        // If unbounded, would make the ray reach the end of the frustum
        delta_cs *= min(1.0, min_dist_to_far_edge);
    }

    (*raymarch).ray_end_cs = (*raymarch).ray_start_cs + delta_cs;
}

/// March from a clip-space position (w = 1)
fn depth_ray_march_from_cs(raymarch: ptr<function, DepthRayMarch>, v: vec3<f32>) {
    (*raymarch).ray_start_cs = v;
}

/// March to a clip-space position (w = 1)
///
/// Must be called after `from_cs`, as it will clip the world-space ray to the view frustum.
fn depth_ray_march_to_cs(raymarch: ptr<function, DepthRayMarch>, end_cs: vec3<f32>) {
    let dir = vec4(end_cs - (*raymarch).ray_start_cs, 0.0) * sign(end_cs.z);
    depth_ray_march_to_cs_dir_impl(raymarch, dir, false);
}

/// March towards a clip-space direction. Infinite (ray is extended to cover the whole view frustum).
///
/// Must be called after `from_cs`, as it will clip the world-space ray to the view frustum.
fn depth_ray_march_to_cs_dir(raymarch: ptr<function, DepthRayMarch>, dir: vec4<f32>) {
    depth_ray_march_to_cs_dir_impl(raymarch, dir, true);
}

/// March to a world-space position.
///
/// Must be called after `from_cs`, as it will clip the world-space ray to the view frustum.
fn depth_ray_march_to_ws(raymarch: ptr<function, DepthRayMarch>, end: vec3<f32>) {
    depth_ray_march_to_cs(raymarch, position_world_to_ndc(end));
}

/// March towards a world-space direction. Infinite (ray is extended to cover the whole view frustum).
///
/// Must be called after `from_cs`, as it will clip the world-space ray to the view frustum.
fn depth_ray_march_to_ws_dir(raymarch: ptr<function, DepthRayMarch>, dir: vec3<f32>) {
    depth_ray_march_to_cs_dir_impl(raymarch, direction_world_to_clip(dir), true);
}

/// Perform the ray march.
fn depth_ray_march_march(raymarch: ptr<function, DepthRayMarch>) -> DepthRayMarchResult {
    var res = DepthRayMarchResult(false, 0.0, vec2(0.0), 0.0, 0.0);

    let ray_start_uv = ndc_to_uv((*raymarch).ray_start_cs.xy);
    let ray_end_uv = ndc_to_uv((*raymarch).ray_end_cs.xy);

    let ray_uv_delta = ray_end_uv - ray_start_uv;
    let ray_len_px = ray_uv_delta * (*raymarch).depth_tex_size;

    let min_px_per_step = 1u;
    let step_count = max(
        2,
        min(i32((*raymarch).linear_steps), i32(floor(length(ray_len_px) / f32(min_px_per_step))))
    );

    let linear_z_to_scaled_linear_z = 1.0 / perspective_camera_near();
    let depth_thickness = (*raymarch).depth_thickness_linear_z * linear_z_to_scaled_linear_z;

    var distance_fn: DepthRaymarchDistanceFn;
    distance_fn.depth_tex_size = (*raymarch).depth_tex_size;
    distance_fn.march_behind_surfaces = (*raymarch).march_behind_surfaces;
    distance_fn.depth_thickness = depth_thickness;
    distance_fn.use_sloppy_march = (*raymarch).use_sloppy_march;

    var hit: DistanceWithPenetration;

    var hit_t = 0.0;
    var miss_t = 0.0;
    var root_finder = hybrid_root_finder_new_with_linear_steps(u32(step_count));
    root_finder.bisection_steps = (*raymarch).bisection_steps;
    root_finder.use_secant = (*raymarch).use_secant;
    root_finder.linear_march_exponent = (*raymarch).linear_march_exponent;
    root_finder.jitter = (*raymarch).jitter;
    let intersected = hybrid_root_finder_find_root(
        &root_finder,
        (*raymarch).ray_start_cs,
        (*raymarch).ray_end_cs,
        &distance_fn,
        &hit_t,
        &miss_t,
        &hit
    );

    res.hit_t = hit_t;

    if (intersected && hit.penetration < depth_thickness && hit.distance < depth_thickness) {
        res.hit = true;
        res.hit_uv = mix(ray_start_uv, ray_end_uv, res.hit_t);
        res.hit_penetration = hit.penetration / linear_z_to_scaled_linear_z;
        res.hit_penetration_frac = hit.penetration / depth_thickness;
        return res;
    }

    res.hit_t = miss_t;
    res.hit_uv = mix(ray_start_uv, ray_end_uv, res.hit_t);

    return res;
}
