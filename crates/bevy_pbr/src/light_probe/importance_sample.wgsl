#import bevy_render::maths::{PI_2};
#import bevy_pbr::lighting::perceptualRoughnessToRoughness;
// Tonemap before filtering to reduce fireflies?
// Pre-expose?

const SAMPLE_COUNT = 32u;

@compute
@workgroup_size(8, 8, 1)
fn generate_radiance_map() {
    let mip_level = 0.0;

    let perceptual_roughness = (0.113875 * mip_level) + 0.089;
    let a = perceptualRoughnessToRoughness(perceptual_roughness);

    var radiance = vec3(0.0);
    var weight = 0.0;
    for (var i = 0u; i < SAMPLE_COUNT; i++) {
        let xi = fract(0.5 + f32(i) * vec2(0.75487766624669276005, 0.5698402909980532659114));

        let phi = PI_2 * xi.x;
        let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
        let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
        let h = vec3(sin_theta * cos(phi), sin_theta * sin(phi), cos_theta);
        // TODO: Use the normal from the light probe
        let n = vec3(0.0, 0.0, 1.0);
        let up_vector = select(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), abs(n.z) < 0.999);
        let tangent_x = normalize(cross(up_vector, n));
        let tangent_y = cross(n, tangent_x);
        // tangent_x * h.x + tangent_y * h.y + n * h.z;
    }

    // radiance / weight;
}

@compute
@workgroup_size(8, 8, 1)
fn generate_irradiance_map() {
}

// Workgroup size 8x8

// 128x128 - 16x16 dispatch
// 64x64 - 8x8 dispatch
// 32x32 - 4x4 dispatch
// 16x16 - 2x2 dispatch
// 8x8 - 1x1 dispatch

// 4x4 -]
// 2x2 -]>  1x1 dispatch
// 1x1 -]

// 16 + 8 + 4 + 2 + 1 + 1 = 32x32 workgroups total


// # SPD:
// # Dispatch with mips = log2(input_mip_0_resolution), inverse_input_size = 1.0 / input_mip_0_resolution
// # If input texture does not have `mips` amount of mips, copy mip 0 to new texture of the correct size