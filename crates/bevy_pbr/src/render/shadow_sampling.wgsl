#define_import_path bevy_pbr::shadow_sampling

#import bevy_pbr::{
    mesh_view_bindings as view_bindings,
    utils::interleaved_gradient_noise,
    utils,
}
#import bevy_render::maths::{orthonormalize, PI}

// Do the lookup, using HW 2x2 PCF and comparison
fn sample_shadow_map_hardware(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
#ifdef NO_ARRAY_TEXTURES_SUPPORT
    return textureSampleCompare(
        view_bindings::directional_shadow_textures,
        view_bindings::directional_shadow_textures_sampler,
        light_local,
        depth,
    );
#else
    return textureSampleCompareLevel(
        view_bindings::directional_shadow_textures,
        view_bindings::directional_shadow_textures_sampler,
        light_local,
        array_index,
        depth,
    );
#endif
}

// Numbers determined by trial and error that gave nice results.
const SPOT_SHADOW_TEXEL_SIZE: f32 = 0.0134277345;
const POINT_SHADOW_SCALE: f32 = 0.003;
const POINT_SHADOW_TEMPORAL_OFFSET_SCALE: f32 = 0.5;

// These are the standard MSAA sample point positions from D3D. They were chosen
// to get a reasonable distribution that's not too regular.
//
// https://learn.microsoft.com/en-us/windows/win32/api/d3d11/ne-d3d11-d3d11_standard_multisample_quality_levels?redirectedfrom=MSDN
const D3D_SAMPLE_POINT_POSITIONS: array<vec2<f32>, 8> = array(
    vec2( 0.125, -0.375),
    vec2(-0.125,  0.375),
    vec2( 0.625,  0.125),
    vec2(-0.375, -0.625),
    vec2(-0.625,  0.625),
    vec2(-0.875, -0.125),
    vec2( 0.375,  0.875),
    vec2( 0.875, -0.875),
);

// And these are the coefficients corresponding to the probability distribution
// function of a 2D Gaussian lobe with zero mean and the identity covariance
// matrix at those points.
const D3D_SAMPLE_POINT_COEFFS: array<f32, 8> = array(
    0.157112,
    0.157112,
    0.138651,
    0.130251,
    0.114946,
    0.114946,
    0.107982,
    0.079001,
);

// https://web.archive.org/web/20230210095515/http://the-witness.net/news/2013/09/shadow-mapping-summary-part-1
fn sample_shadow_map_castano_thirteen(light_local: vec2<f32>, depth: f32, array_index: i32) -> f32 {
    let shadow_map_size = vec2<f32>(textureDimensions(view_bindings::directional_shadow_textures));
    let inv_shadow_map_size = 1.0 / shadow_map_size;

    let uv = light_local * shadow_map_size;
    var base_uv = floor(uv + 0.5);
    let s = (uv.x + 0.5 - base_uv.x);
    let t = (uv.y + 0.5 - base_uv.y);
    base_uv -= 0.5;
    base_uv *= inv_shadow_map_size;

    let uw0 = (4.0 - 3.0 * s);
    let uw1 = 7.0;
    let uw2 = (1.0 + 3.0 * s);

    let u0 = (3.0 - 2.0 * s) / uw0 - 2.0;
    let u1 = (3.0 + s) / uw1;
    let u2 = s / uw2 + 2.0;

    let vw0 = (4.0 - 3.0 * t);
    let vw1 = 7.0;
    let vw2 = (1.0 + 3.0 * t);

    let v0 = (3.0 - 2.0 * t) / vw0 - 2.0;
    let v1 = (3.0 + t) / vw1;
    let v2 = t / vw2 + 2.0;

    var sum = 0.0;

    sum += uw0 * vw0 * sample_shadow_map_hardware(base_uv + (vec2(u0, v0) * inv_shadow_map_size), depth, array_index);
    sum += uw1 * vw0 * sample_shadow_map_hardware(base_uv + (vec2(u1, v0) * inv_shadow_map_size), depth, array_index);
    sum += uw2 * vw0 * sample_shadow_map_hardware(base_uv + (vec2(u2, v0) * inv_shadow_map_size), depth, array_index);

    sum += uw0 * vw1 * sample_shadow_map_hardware(base_uv + (vec2(u0, v1) * inv_shadow_map_size), depth, array_index);
    sum += uw1 * vw1 * sample_shadow_map_hardware(base_uv + (vec2(u1, v1) * inv_shadow_map_size), depth, array_index);
    sum += uw2 * vw1 * sample_shadow_map_hardware(base_uv + (vec2(u2, v1) * inv_shadow_map_size), depth, array_index);

    sum += uw0 * vw2 * sample_shadow_map_hardware(base_uv + (vec2(u0, v2) * inv_shadow_map_size), depth, array_index);
    sum += uw1 * vw2 * sample_shadow_map_hardware(base_uv + (vec2(u1, v2) * inv_shadow_map_size), depth, array_index);
    sum += uw2 * vw2 * sample_shadow_map_hardware(base_uv + (vec2(u2, v2) * inv_shadow_map_size), depth, array_index);

    return sum * (1.0 / 144.0);
}

fn map(min1: f32, max1: f32, min2: f32, max2: f32, value: f32) -> f32 {
    return min2 + (value - min1) * (max2 - min2) / (max1 - min1);
}

// Creates a random rotation matrix using interleaved gradient noise.
//
// See: https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare/
fn random_rotation_matrix(scale: vec2<f32>) -> mat2x2<f32> {
    let random_angle = 2.0 * PI * interleaved_gradient_noise(
        scale, view_bindings::globals.frame_count);
    let m = vec2(sin(random_angle), cos(random_angle));
    return mat2x2(
        m.y, -m.x,
        m.x, m.y
    );
}

fn sample_shadow_map_jimenez_fourteen(light_local: vec2<f32>, depth: f32, array_index: i32, texel_size: f32) -> f32 {
    let shadow_map_size = vec2<f32>(textureDimensions(view_bindings::directional_shadow_textures));
    let rotation_matrix = random_rotation_matrix(light_local * shadow_map_size);

    // Empirically chosen fudge factor to make PCF look better across different CSM cascades
    let f = map(0.00390625, 0.022949219, 0.015, 0.035, texel_size);
    let uv_offset_scale = f / (texel_size * shadow_map_size);

    // https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare (slides 120-135)
    let sample_offset0 = (rotation_matrix * utils::SPIRAL_OFFSET_0_) * uv_offset_scale;
    let sample_offset1 = (rotation_matrix * utils::SPIRAL_OFFSET_1_) * uv_offset_scale;
    let sample_offset2 = (rotation_matrix * utils::SPIRAL_OFFSET_2_) * uv_offset_scale;
    let sample_offset3 = (rotation_matrix * utils::SPIRAL_OFFSET_3_) * uv_offset_scale;
    let sample_offset4 = (rotation_matrix * utils::SPIRAL_OFFSET_4_) * uv_offset_scale;
    let sample_offset5 = (rotation_matrix * utils::SPIRAL_OFFSET_5_) * uv_offset_scale;
    let sample_offset6 = (rotation_matrix * utils::SPIRAL_OFFSET_6_) * uv_offset_scale;
    let sample_offset7 = (rotation_matrix * utils::SPIRAL_OFFSET_7_) * uv_offset_scale;

    var sum = 0.0;
    sum += sample_shadow_map_hardware(light_local + sample_offset0, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset1, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset2, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset3, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset4, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset5, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset6, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + sample_offset7, depth, array_index);
    return sum / 8.0;
}

fn sample_shadow_map(light_local: vec2<f32>, depth: f32, array_index: i32, texel_size: f32) -> f32 {
#ifdef SHADOW_FILTER_METHOD_GAUSSIAN
    return sample_shadow_map_castano_thirteen(light_local, depth, array_index);
#else ifdef SHADOW_FILTER_METHOD_TEMPORAL
    return sample_shadow_map_jimenez_fourteen(light_local, depth, array_index, texel_size);
#else ifdef SHADOW_FILTER_METHOD_HARDWARE_2X2
    return sample_shadow_map_hardware(light_local, depth, array_index);
#else
    // This needs a default return value to avoid shader compilation errors if it's compiled with no SHADOW_FILTER_METHOD_* defined.
    // (eg. if the normal prepass is enabled it ends up compiling this due to the normal prepass depending on pbr_functions, which depends on shadows)
    // This should never actually get used, as anyone using bevy's lighting/shadows should always have a SHADOW_FILTER_METHOD defined.
    // Set to 0 to make it obvious that something is wrong.
    return 0.0;
#endif
}

// NOTE: Due to the non-uniform control flow in `shadows::fetch_point_shadow`,
// we must use the Level variant of textureSampleCompare to avoid undefined
// behavior due to some of the fragments in a quad (2x2 fragments) being
// processed not being sampled, and this messing with mip-mapping functionality.
// The shadow maps have no mipmaps so Level just samples from LOD 0.
fn sample_shadow_cubemap_hardware(light_local: vec3<f32>, depth: f32, light_id: u32) -> f32 {
#ifdef NO_CUBE_ARRAY_TEXTURES_SUPPORT
    return textureSampleCompare(view_bindings::point_shadow_textures, view_bindings::point_shadow_textures_sampler, light_local, depth);
#else
    return textureSampleCompareLevel(view_bindings::point_shadow_textures, view_bindings::point_shadow_textures_sampler, light_local, i32(light_id), depth);
#endif
}

fn sample_shadow_cubemap_at_offset(
    position: vec2<f32>,
    coeff: f32,
    x_basis: vec3<f32>,
    y_basis: vec3<f32>,
    light_local: vec3<f32>,
    depth: f32,
    light_id: u32,
) -> f32 {
    return sample_shadow_cubemap_hardware(
        light_local + position.x * x_basis + position.y * y_basis,
        depth,
        light_id
    ) * coeff;
}

// This more or less does what Castano13 does, but in 3D space. Castano13 is
// essentially an optimized 2D Gaussian filter that takes advantage of the
// bilinear filtering hardware to reduce the number of samples needed. This
// trick doesn't apply to cubemaps, so we manually apply a Gaussian filter over
// the standard 8xMSAA pattern instead.
fn sample_shadow_cubemap_gaussian(
    light_local: vec3<f32>,
    depth: f32,
    scale: f32,
    distance_to_light: f32,
    light_id: u32,
) -> f32 {
    // Create an orthonormal basis so we can apply a 2D sampling pattern to a
    // cubemap.
    var up = vec3(0.0, 1.0, 0.0);
    if (dot(up, normalize(light_local)) > 0.99) {
        up = vec3(1.0, 0.0, 0.0);   // Avoid creating a degenerate basis.
    }
    let basis = orthonormalize(light_local, up) * scale * distance_to_light;

    var sum: f32 = 0.0;
    sum += sample_shadow_cubemap_at_offset(
        D3D_SAMPLE_POINT_POSITIONS[0], D3D_SAMPLE_POINT_COEFFS[0],
        basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        D3D_SAMPLE_POINT_POSITIONS[1], D3D_SAMPLE_POINT_COEFFS[1],
        basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        D3D_SAMPLE_POINT_POSITIONS[2], D3D_SAMPLE_POINT_COEFFS[2],
        basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        D3D_SAMPLE_POINT_POSITIONS[3], D3D_SAMPLE_POINT_COEFFS[3],
        basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        D3D_SAMPLE_POINT_POSITIONS[4], D3D_SAMPLE_POINT_COEFFS[4],
        basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        D3D_SAMPLE_POINT_POSITIONS[5], D3D_SAMPLE_POINT_COEFFS[5],
        basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        D3D_SAMPLE_POINT_POSITIONS[6], D3D_SAMPLE_POINT_COEFFS[6],
        basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        D3D_SAMPLE_POINT_POSITIONS[7], D3D_SAMPLE_POINT_COEFFS[7],
        basis[0], basis[1], light_local, depth, light_id);
    return sum;
}

// This is a port of the Jimenez14 filter above to the 3D space. It jitters the
// points in the spiral pattern after first creating a 2D orthonormal basis
// along the principal light direction.
fn sample_shadow_cubemap_temporal(
    light_local: vec3<f32>,
    depth: f32,
    scale: f32,
    distance_to_light: f32,
    light_id: u32,
) -> f32 {
    // Create an orthonormal basis so we can apply a 2D sampling pattern to a
    // cubemap.
    var up = vec3(0.0, 1.0, 0.0);
    if (dot(up, normalize(light_local)) > 0.99) {
        up = vec3(1.0, 0.0, 0.0);   // Avoid creating a degenerate basis.
    }
    let basis = orthonormalize(light_local, up) * scale * distance_to_light;

    let rotation_matrix = random_rotation_matrix(vec2(1.0));

    let sample_offset0 = rotation_matrix * utils::SPIRAL_OFFSET_0_ *
        POINT_SHADOW_TEMPORAL_OFFSET_SCALE;
    let sample_offset1 = rotation_matrix * utils::SPIRAL_OFFSET_1_ *
        POINT_SHADOW_TEMPORAL_OFFSET_SCALE;
    let sample_offset2 = rotation_matrix * utils::SPIRAL_OFFSET_2_ *
        POINT_SHADOW_TEMPORAL_OFFSET_SCALE;
    let sample_offset3 = rotation_matrix * utils::SPIRAL_OFFSET_3_ *
        POINT_SHADOW_TEMPORAL_OFFSET_SCALE;
    let sample_offset4 = rotation_matrix * utils::SPIRAL_OFFSET_4_ *
        POINT_SHADOW_TEMPORAL_OFFSET_SCALE;
    let sample_offset5 = rotation_matrix * utils::SPIRAL_OFFSET_5_ *
        POINT_SHADOW_TEMPORAL_OFFSET_SCALE;
    let sample_offset6 = rotation_matrix * utils::SPIRAL_OFFSET_6_ *
        POINT_SHADOW_TEMPORAL_OFFSET_SCALE;
    let sample_offset7 = rotation_matrix * utils::SPIRAL_OFFSET_7_ *
        POINT_SHADOW_TEMPORAL_OFFSET_SCALE;

    var sum: f32 = 0.0;
    sum += sample_shadow_cubemap_at_offset(
        sample_offset0, 0.125, basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        sample_offset1, 0.125, basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        sample_offset2, 0.125, basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        sample_offset3, 0.125, basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        sample_offset4, 0.125, basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        sample_offset5, 0.125, basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        sample_offset6, 0.125, basis[0], basis[1], light_local, depth, light_id);
    sum += sample_shadow_cubemap_at_offset(
        sample_offset7, 0.125, basis[0], basis[1], light_local, depth, light_id);
    return sum;
}

fn sample_shadow_cubemap(
    light_local: vec3<f32>,
    distance_to_light: f32,
    depth: f32,
    light_id: u32,
) -> f32 {
#ifdef SHADOW_FILTER_METHOD_GAUSSIAN
    return sample_shadow_cubemap_gaussian(
        light_local, depth, POINT_SHADOW_SCALE, distance_to_light, light_id);
#else ifdef SHADOW_FILTER_METHOD_TEMPORAL
    return sample_shadow_cubemap_temporal(
        light_local, depth, POINT_SHADOW_SCALE, distance_to_light, light_id);
#else ifdef SHADOW_FILTER_METHOD_HARDWARE_2X2
    return sample_shadow_cubemap_hardware(light_local, depth, light_id);
#else
    // This needs a default return value to avoid shader compilation errors if it's compiled with no SHADOW_FILTER_METHOD_* defined.
    // (eg. if the normal prepass is enabled it ends up compiling this due to the normal prepass depending on pbr_functions, which depends on shadows)
    // This should never actually get used, as anyone using bevy's lighting/shadows should always have a SHADOW_FILTER_METHOD defined.
    // Set to 0 to make it obvious that something is wrong.
    return 0.0;
#endif
}
