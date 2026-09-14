import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance;
import bevy_pbr::deferred::types::unpack_24bit_normal;
import bevy_pbr::render::pbr_functions::{calculate_diffuse_color, calculate_F0, calculate_F0_dielectric};
import bevy_render::utils::octahedral_decode;
import package::realtime::gbuffer_utils::gpixel_resolve;
import package::realtime::bindings::{gbuffer, depth_buffer, motion_vectors, previous_view, view, diffuse_albedo, specular_albedo, normal_roughness, dlss_rr_depth, specular_motion_vectors};
import package::scene::brdf::{evaluate_specular_brdf, F_AB, lobe_reflectances};
import package::scene::bindings::{MIRROR_ROUGHNESS_THRESHOLD, ResolvedMaterial, ResolvedRayHitFull};

enable wgpu_ray_query;

/// A surface is replaced outright only when nearly all of its reflectance leaves through the delta
/// lobe.
const FULL_REPLACEMENT_FRACTION = 0.9;

/// How far a reflector's virtual image may shift between neighboring pixels, as a fraction of the
/// virtual distance itself, before the surface is treated as too curved to replace.
const MAX_VIRTUAL_DEPTH_JITTER = 0.002;

/// Even a perfectly flat surface measures a small nonzero normal change, because G-buffer normals are
/// 24-bit octahedral and neighboring texels round differently.
const NORMAL_QUANTIZATION_FLOOR = 0.002;

/// Distance, in meters, standing in for "infinitely far" when a mirror ray finds nothing.
const ENVIRONMENT_VIRTUAL_DISTANCE = 10000.0;

@compute @workgroup_size(8, 8, 1)
fn resolve_dlss_rr_textures(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_id = global_id.xy;
    if any(pixel_id >= vec2u(view.main_pass_viewport.zw)) { return; }

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    let surface_motion_vector = textureLoad(motion_vectors, pixel_id).xy;

    textureStore(dlss_rr_depth, pixel_id, vec4(depth));

    if depth == 0.0 {
        textureStore(specular_motion_vectors, pixel_id, vec4(surface_motion_vector, vec2(0.0)));
        textureStore(diffuse_albedo, pixel_id, vec4(0.0));
        textureStore(specular_albedo, pixel_id, vec4(0.5));
        textureStore(normal_roughness, pixel_id, vec4(0.0, 0.0, 1.0, 0.0));
        return;
    }

    let surface = gpixel_resolve(textureLoad(gbuffer, pixel_id, 0), depth, pixel_id, view.main_pass_viewport.zw, view.world_from_clip);
    let F0 = calculate_F0(surface.material.base_color, surface.material.metallic, vec3(surface.material.reflectance));
    let wo = normalize(view.world_position - surface.world_position);

    textureStore(diffuse_albedo, pixel_id, vec4(calculate_diffuse_color(surface.material.base_color, surface.material.metallic, 0.0, 0.0), 0.0));
    textureStore(specular_albedo, pixel_id, vec4(env_brdf_approx2(F0, surface.material.roughness, surface.world_normal, wo), 0.0));
    textureStore(normal_roughness, pixel_id, vec4(surface.world_normal, surface.material.perceptual_roughness));

    // Resetting to the specular motion vector on the frames we draw a diffuse lobe means we
    // wouldn't be writing the motion vectors that frame, so it would alternate the guide between
    // to values, and the denoiser blurs the reflection rather than reprojecting it.
    if !needs_specular_motion_vector(surface.material.roughness) {
        textureStore(specular_motion_vectors, pixel_id, vec4(surface_motion_vector, vec2(0.0)));
    }
}

fn needs_specular_motion_vector(roughness: f32) -> bool {
    return roughness <= 0.0625;
}

/// Carried across every bounce of the path walk, so anything derivable from a value the walk already
/// holds is derived at the end instead of stored. What is left is only what the walk itself produces.
struct PsrState {
    finished: bool,
    /// Whether this pixel's surface is being swapped out for the reflected one, or is only having its
    /// reflection's motion described. Keyed on the primary surface only, and false for anything glossy.
    replace_fully: bool,
    /// The chain's cumulative transform - the product of one Householder reflection per mirror, in
    /// chain order - stored as a rotor plus the parity of the reflection count rather than as the 3x3
    /// matrix it denotes.
    mirror_rotor: vec4<f32>,
    mirror_parity_odd: bool,
    /// Distance traveled since the primary surface, camera through every mirror. Not the total path
    /// length: keeping the reflection's own share means the primary distance is only ever added, never
    /// subtracted back out, so it can be recovered at the end without the two disagreeing.
    reflection_length: f32,
    /// Product of the Fresnel term at each mirror in the chain: what the reflected surface's color is
    /// multiplied by on its way back to the eye.
    throughput: vec3<f32>,
}

fn psr_init(world_normal: vec3<f32>, material: ResolvedMaterial) -> PsrState {
    var psr: PsrState;
    psr.finished = !needs_specular_motion_vector(material.roughness);
    psr.replace_fully = is_delta_mirror(material);
    // The chain opens with the primary surface's own reflection, so parity starts odd.
    psr.mirror_rotor = mirror_chain_rotor(world_normal, false);
    psr.mirror_parity_odd = true;
    psr.reflection_length = 0.0;
    psr.throughput = vec3(1.0);
    return psr;
}

/// Fold one more mirror into the chain, equivalent to right-multiplying the chain's matrix by this
/// mirror's Householder reflection.
/// https://en.wikipedia.org/wiki/Householder_transformation
///
/// A product of k Householder reflections is orthogonal with determinant (-1)^k, so it is a rotation
/// when k is even, and a rotation composed with any one fixed reflection when k is odd.
fn psr_chain_mirror(psr: PsrState, plane_normal: vec3<f32>) -> PsrState {
    var chained = psr;
    chained.mirror_rotor = quaternion_multiply(psr.mirror_rotor, mirror_chain_rotor(plane_normal, psr.mirror_parity_odd));
    chained.mirror_parity_odd = !psr.mirror_parity_odd;
    return chained;
}

fn psr_environment_miss(pixel_id: vec2<u32>, primary_wo: vec3<f32>) {
    let far_position = view.world_position - (primary_wo * ENVIRONMENT_VIRTUAL_DISTANCE);
    textureStore(specular_motion_vectors, pixel_id, vec4(calculate_motion_vector(far_position, far_position), vec2(0.0)));
}

/// Primary surface replacement for perfect mirrors. The reflection chain has been followed to its
/// first non-mirror hit; write that surface's attributes, reflected into the mirror's virtual space,
/// to the DLSS RR guide buffers so the denoiser treats this pixel as directly seeing it.
/// https://developer.nvidia.com/blog/rendering-perfect-reflections-and-refractions-in-path-traced-games/#primary_surface_replacement
fn replace_primary_surface(
    pixel_id: vec2<u32>,
    psr: PsrState,
    ray_hit: ResolvedRayHitFull,
    primary_material: ResolvedMaterial,
    primary_world_position: vec3<f32>,
    primary_normal: vec3<f32>,
    primary_wo: vec3<f32>,
    primary_F_ab: vec2<f32>,
) {
    let primary_distance = length(primary_world_position - view.world_position);
    let path_length = primary_distance + psr.reflection_length;

    // Too curved to track the reflection; fall back to the surface's own motion so the guide is not stale
    if psr.replace_fully && 2.0 * reflector_curvature(pixel_id, primary_normal) * psr.reflection_length > MAX_VIRTUAL_DEPTH_JITTER * path_length {
        textureStore(specular_motion_vectors, pixel_id, vec4(textureLoad(motion_vectors, pixel_id).xy, vec2(0.0)));
        return;
    }

    let virtual_position = primary_world_position - (primary_wo * psr.reflection_length);
    let world_motion = ray_hit.previous_frame_world_position - ray_hit.world_position;
    let virtual_previous_position = virtual_position + apply_mirror_chain(psr, world_motion);
    let specular_motion_vector = calculate_motion_vector(virtual_position, virtual_previous_position);

    textureStore(specular_motion_vectors, pixel_id, vec4(specular_motion_vector, vec2(0.0)));

    if !psr.replace_fully {
        return;
    }

    let wo = normalize(view.world_position - virtual_position);
    let virtual_normal = normalize(apply_mirror_chain(psr, ray_hit.world_normal));

    let primary_depth = textureLoad(depth_buffer, pixel_id, 0);
    textureStore(dlss_rr_depth, pixel_id, vec4(primary_depth * primary_distance / path_length));
    textureStore(motion_vectors, pixel_id, vec4(specular_motion_vector, vec2(0.0)));

    let reflected_diffuse = calculate_diffuse_color(ray_hit.material.base_color, ray_hit.material.metallic, 0.0, 0.0);
    let reflected_F0 = calculate_F0(ray_hit.material.base_color, ray_hit.material.metallic, vec3(ray_hit.material.reflectance));
    let reflected_specular = env_brdf_approx2(reflected_F0, ray_hit.material.roughness, virtual_normal, wo);

    let own_diffuse = calculate_diffuse_color(primary_material.base_color, primary_material.metallic, 0.0, 0.0);
    let own_F0 = calculate_F0(primary_material.base_color, primary_material.metallic, vec3(primary_material.reflectance));
    let own_specular = env_brdf_approx2(own_F0, primary_material.roughness, primary_normal, primary_wo);

    let specular_fraction = specular_fraction(primary_material, primary_F_ab);

    textureStore(diffuse_albedo, pixel_id, vec4(psr.throughput * reflected_diffuse + own_diffuse, 0.0));
    textureStore(specular_albedo, pixel_id, vec4(mix(own_specular, psr.throughput * reflected_specular, specular_fraction), 0.0));
    textureStore(normal_roughness, pixel_id, vec4(virtual_normal, ray_hit.material.perceptual_roughness));
}

fn primary_surface_glossy(material: ResolvedMaterial) -> bool {
    return material.roughness > MIRROR_ROUGHNESS_THRESHOLD;
}

fn is_delta_mirror(material: ResolvedMaterial) -> bool {
    return material.roughness <= MIRROR_ROUGHNESS_THRESHOLD
        && specular_fraction(material, F_AB(material.perceptual_roughness, 1.0)) >= FULL_REPLACEMENT_FRACTION;
}

/// Share of a surface's total reflectance that leaves through the specular lobe, in 0..1.
fn specular_fraction(material: ResolvedMaterial, F_ab: vec2<f32>) -> f32 {
    let rho = lobe_reflectances(material.base_color, calculate_F0_dielectric(vec3(material.reflectance)), material, F_ab);
    let specular_luminance = luminance(rho.specular);
    return specular_luminance / max(specular_luminance + luminance(rho.diffuse), 0.0001);
}

fn psr_bounce_reflectance(wo: vec3<f32>, wi: vec3<f32>, normal: vec3<f32>, material: ResolvedMaterial, F_ab: vec2<f32>, delta: bool) -> vec3<f32> {
    if delta {
        return evaluate_specular_brdf(wo, wi, normal, material, F_ab);
    }
    return lobe_reflectances(material.base_color, calculate_F0_dielectric(vec3(material.reflectance)), material, F_ab).specular;
}

/// Angle, in radians, that the shading normal changes over one pixel in screen space.
fn reflector_curvature(pixel_id: vec2<u32>, normal: vec3<f32>) -> f32 {
    let max_pixel_id = vec2<u32>(view.main_pass_viewport.zw) - vec2(1u);
    let right = load_gbuffer_normal(min(pixel_id + vec2(1u, 0u), max_pixel_id));
    let down = load_gbuffer_normal(min(pixel_id + vec2(0u, 1u), max_pixel_id));
    let min_cos = min(dot(normal, right), dot(normal, down));
    let angle = sqrt(max(0.0, 2.0 * (1.0 - min_cos)));
    return max(0.0, angle - NORMAL_QUANTIZATION_FLOOR);
}

fn load_gbuffer_normal(pixel_id: vec2<u32>) -> vec3<f32> {
    return octahedral_decode(unpack_24bit_normal(textureLoad(gbuffer, pixel_id, 0).a));
}

fn apply_mirror_chain(psr: PsrState, v: vec3<f32>) -> vec3<f32> {
    var reflected = v;
    if psr.mirror_parity_odd {
        reflected.z = -reflected.z;
    }
    let t = 2.0 * cross(psr.mirror_rotor.xyz, reflected);
    return reflected + psr.mirror_rotor.w * t + cross(psr.mirror_rotor.xyz, t);
}

/// The rotation half of `H(n) * F` at even parity, or of `F * H(n)` at odd, where `H(n)` is the
/// Householder reflection about `plane_normal` and `F` the fixed z-axis one.
fn mirror_chain_rotor(plane_normal: vec3<f32>, parity_odd: bool) -> vec4<f32> {
    if parity_odd {
        return vec4(plane_normal.y, -plane_normal.x, 0.0, plane_normal.z);
    }
    return vec4(-plane_normal.y, plane_normal.x, 0.0, plane_normal.z);
}

fn quaternion_multiply(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    return vec4(a.w * b.xyz + b.w * a.xyz + cross(a.xyz, b.xyz), a.w * b.w - dot(a.xyz, b.xyz));
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

fn env_brdf_approx2(specular_color: vec3<f32>, alpha: f32, N: vec3<f32>, V: vec3<f32>) -> vec3<f32> {
    let NoV = abs(dot(N, V));

    var X: vec4<f32>;
    X.x = 1.0;
    X.y = NoV;
    X.z = NoV * NoV;
    X.w = NoV * X.z;

    var Y: vec4<f32>;
    Y.x = 1.0;
    Y.y = alpha;
    Y.z = alpha * alpha;
    Y.w = alpha * Y.z;

    let M1 = mat2x2<f32>(0.99044, 1.29678, -1.28514, -0.755907);
    let M2 = mat3x3<f32>(1.0, 20.3225, 121.563, 2.92338, -27.0302, 626.13, 59.4188, 222.592, 316.627);
    let M3 = mat2x2<f32>(0.0365463, 9.0632, 3.32707, -9.04756);
    let M4 = mat3x3<f32>(1.0, 9.04401, 5.56589, 3.59685, -16.3174, 19.7886, -1.36772, 9.22949, -20.2123);

    var bias = dot(M1 * X.xy, Y.xy) / dot(M2 * X.xyw, Y.xyw);
    let scale = dot(M3 * X.xy, Y.xy) / dot(M4 * X.xzw, Y.xyw);

    bias *= saturate(specular_color.g * 50.0);

    return fma(specular_color, vec3(max(0.0, scale)), vec3(max(0.0, bias)));
}
