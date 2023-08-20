// Importance samples (Lambertian distribution) a skybox to produce a diffuse lighting cubemap
// Based on https://github.com/KhronosGroup/glTF-IBL-Sampler/blob/master/lib/source/shaders/filter.frag

#import bevy_pbr::utils PI

@group(0) @binding(0) var skybox: texture_cube<f32>;
#ifdef RG11B10FLOAT
@group(0) @binding(1) var diffuse_map: texture_storage_2d_array<rg11b10float, write>;
#else
@group(0) @binding(1) var diffuse_map: texture_storage_2d_array<rgba16float, write>;
#endif
@group(0) @binding(2) var bilinear: sampler;

fn get_dir(u: f32, v: f32, face: u32) -> vec3<f32> {
    switch face {
        case 0u: { return vec3(1.0, v, -u); }
        case 1u: { return vec3(-1.0, v, u); }
        case 2u: { return vec3(u, 1.0, -v); }
        case 3u: { return vec3(u, -1.0, v); }
        case 4u: { return vec3(u, v, 1.0); }
        default { return vec3(-u, v, -1.0); }
    }
}

fn generate_tbn(normal: vec3<f32>) -> mat3x3<f32> {
    var bitangent = vec3(0.0, 1.0, 0.0);

    let n_dot_up = dot(normal, bitangent);
    if 1.0 - abs(n_dot_up) <= 0.0000001 {
        if n_dot_up > 0.0 {
            bitangent = vec3(0.0, 0.0, 1.0);
        } else {
            bitangent = vec3(0.0, 0.0, -1.0);
        }
    }

    let tangent = normalize(cross(bitangent, normal));
    bitangent = cross(normal, tangent);

    return mat3x3(tangent, bitangent, normal);
}

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let u = (f32(id.x) * 2.0 + 1.0) / 64.0 - 1.0;
    let v = -(f32(id.y) * 2.0 + 1.0) / 64.0 + 1.0;

    let normal = normalize(get_dir(u, v, id.z));

    var color = vec3(0.0);
    for (var sample_i = 0u; sample_i < 32u; sample_i++) {
        // R2 sequence - http://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences
        let r = fract(0.5 + f32(sample_i) * vec2<f32>(0.75487766624669276005, 0.5698402909980532659114));

        let cos_theta = sqrt(1.0 - f32(r.y));
        let sin_theta = sqrt(r.y);
        let phi = 2.0 * PI * r.x;

        let local_space_direction = normalize(vec3(
            sin_theta * cos(phi),
            sin_theta * sin(phi),
            cos_theta,
        ));
        let direction = generate_tbn(normal) * local_space_direction;

        color += textureSampleLevel(skybox, bilinear, direction, 0.0).rgb;
    }
    color /= 32.0;

    textureStore(diffuse_map, id.xy, id.z, vec4(color, 1.0));
}
