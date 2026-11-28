//! Ray cones track the footprint of a pixel along a path, which is used to pick texture mip levels.
//!
//! Improved Shader and Texture Level of Detail Using Ray Cones, Akenine-Moller et al. 2021
//! https://jcgt.org/published/0010/01/01
//!
//! Texture Level of Detail Strategies for Real-Time Ray Tracing, Akenine-Moller et al. 2019 (Ray Tracing Gems, chapter 20)
//! https://doi.org/10.1007/978-1-4842-4427-2_20

import bevy_render::maths::PI;

/// Diffuse surfaces are treated as fully rough, but the roughness spread diverges at a roughness of 1.0.
const RAY_CONE_MAX_ROUGHNESS = 0.99f;

/// Grazing angles make the curvature spread diverge.
const RAY_CONE_MIN_NDOTD = 0.01f;

/// Equation 5 relies on a small angle approximation, which breaks down for wide cones hitting tightly curved surfaces.
/// Left unbounded, a few such bounces can overflow the cone to infinity.
const RAY_CONE_MAX_CURVATURE_SPREAD = PI;

struct RayCone {
    /// Width of the cone at the current point. Never negative.
    width: f32,
    /// Change in width per unit of distance traveled.
    spread_angle: f32,
}

/// The cone of a camera ray through a single pixel.
///
/// `mip_bias` is the view's texture mip bias, so that ray traced hits are filtered like rasterized ones.
fn ray_cone_from_camera(clip_from_view: mat4x4<f32>, viewport_height: f32, mip_bias: f32) -> RayCone {
    // Ray Tracing Gems chapter 20, equation 30: atan(2 * tan(fov_y / 2) / height)
    let pixel_spread_angle = atan(2.0 / (clip_from_view[1][1] * viewport_height));
    return RayCone(0.0, pixel_spread_angle * exp2(mip_bias));
}

/// Grow the cone along a ray of length `t`.
fn ray_cone_propagate(ray_cone: RayCone, t: f32) -> RayCone {
    let width = ray_cone.width + ray_cone.spread_angle * t;
    return RayCone(abs(width), select(ray_cone.spread_angle, -ray_cone.spread_angle, width < 0.0));
}

/// Change the spread of a cone scattering off a surface, from both its curvature and its roughness.
/// See `ray_cone_apply_curvature` and `ray_cone_apply_roughness`.
fn ray_cone_scatter(
    ray_cone: RayCone,
    ray_direction: vec3<f32>,
    normal: vec3<f32>,
    curvature: f32,
    roughness: f32,
    diffuse_selected: bool,
    first_hit: bool,
) -> RayCone {
    let curved_ray_cone = ray_cone_apply_curvature(ray_cone, ray_direction, normal, curvature);
    return ray_cone_apply_roughness(curved_ray_cone, roughness, diffuse_selected, first_hit);
}

/// Change the spread of a cone reflecting off a curved surface. The cone must already be propagated to the hit point.
///
/// `curvature` must be positive when the surface is convex as seen from the side the ray arrived on,
/// see `ResolvedRayHitFull::curvature`.
///
/// Section 3.2, equation 5.
fn ray_cone_apply_curvature(ray_cone: RayCone, ray_direction: vec3<f32>, normal: vec3<f32>, curvature: f32) -> RayCone {
    let NdotD = max(abs(dot(normal, ray_direction)), RAY_CONE_MIN_NDOTD);
    let curvature_spread = 2.0 * curvature * ray_cone.width / NdotD;
    let clamped_curvature_spread = clamp(curvature_spread, -RAY_CONE_MAX_CURVATURE_SPREAD, RAY_CONE_MAX_CURVATURE_SPREAD);
    return RayCone(ray_cone.width, ray_cone.spread_angle + clamped_curvature_spread);
}

/// Widen a cone scattering off a rough surface.
///
/// A path tracer averages over the lobe with random directions anyway, so the spread is kept below the bounds of the lobe
/// to avoid overblurring, and is kept especially narrow at the first hit.
///
/// Section 4 and appendix A.
fn ray_cone_apply_roughness(ray_cone: RayCone, roughness: f32, diffuse_selected: bool, first_hit: bool) -> RayCone {
    let clamped_roughness = select(min(roughness, RAY_CONE_MAX_ROUGHNESS), RAY_CONE_MAX_ROUGHNESS, diffuse_selected);
    let a2 = clamped_roughness * clamped_roughness;

    // Standard deviation of a Gaussian fitted to the GGX lobe, in the tangent plane of the reflected direction
    let sigma = sqrt(0.5 * a2 / (1.0 - a2));
    let roughness_spread = select(sigma, 0.25 * sigma, first_hit);

    return RayCone(ray_cone.width, ray_cone.spread_angle + roughness_spread);
}

/// A texture LOD that samples mip 0.
const TEXTURE_LOD_MIP_0 = -1000.0f;

/// Isotropic mip level of a ray cone's footprint on a triangle, without the texture's own resolution
/// (`sample_texture` adds it). Sampled as a single level with `textureSampleLevel`.
///
/// Texture Level of Detail Strategies for Real-Time Ray Tracing, Ray Tracing Gems chapter 20, equation 34:
/// 0.5 * log2(uv area / world area) + log2(cone width / |n . d|).
/// https://doi.org/10.1007/978-1-4842-4427-2_20
///
/// The cone width is its full width rather than its radius, so that a camera ray picks the same mip level as the
/// screen-space derivatives of rasterization would.
fn ray_cone_texture_lod(world_vertices: array<vec3<f32>, 3>, uvs: array<vec2<f32>, 3>, ray_direction: vec3<f32>, ray_cone_width: f32) -> f32 {
    if ray_cone_width == 0.0 {
        return TEXTURE_LOD_MIP_0;
    }

    let face_normal_unnormalized = cross(world_vertices[1] - world_vertices[0], world_vertices[2] - world_vertices[0]);
    let world_double_area = length(face_normal_unnormalized);
    let uv_e1 = uvs[1] - uvs[0];
    let uv_e2 = uvs[2] - uvs[0];
    let uv_double_area = abs(uv_e1.x * uv_e2.y - uv_e1.y * uv_e2.x);
    if world_double_area == 0.0 || uv_double_area == 0.0 {
        return TEXTURE_LOD_MIP_0;
    }
    let NdotD = max(abs(dot(face_normal_unnormalized, ray_direction)) / world_double_area, 0.0001);
    return 0.5 * log2(uv_double_area / world_double_area) + log2(ray_cone_width / NdotD);
}

/// Approximate mean curvature of a triangle, averaged from the curvature along each of its edges.
///
/// Improved Shader and Texture Level of Detail Using Ray Cones, Akenine-Moller et al. 2021, equations 6 and 7.
/// https://jcgt.org/published/0010/01/01
///
/// Positive for surfaces that are convex when viewed from the side the normals point towards.
fn triangle_curvature(world_vertices: array<vec3<f32>, 3>, world_vertex_normals: array<vec3<f32>, 3>) -> f32 {
    let k01 = edge_curvature(world_vertices[0], world_vertices[1], world_vertex_normals[0], world_vertex_normals[1]);
    let k12 = edge_curvature(world_vertices[1], world_vertices[2], world_vertex_normals[1], world_vertex_normals[2]);
    let k20 = edge_curvature(world_vertices[2], world_vertices[0], world_vertex_normals[2], world_vertex_normals[0]);
    return (k01 + k12 + k20) / 3.0;
}

fn edge_curvature(p0: vec3<f32>, p1: vec3<f32>, n0: vec3<f32>, n1: vec3<f32>) -> f32 {
    let edge = p1 - p0;
    return dot(n1 - n0, edge) / max(dot(edge, edge), 1e-12);
}
