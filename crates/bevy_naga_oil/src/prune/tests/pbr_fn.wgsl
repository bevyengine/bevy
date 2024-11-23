struct pbr_types__StandardMaterial {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    perceptual_roughness: f32,
    metallic: f32,
    reflectance: f32,
    flags: u32,
    alpha_cutoff: f32,
}

struct pbr_functions__PbrInput {
    material: pbr_types__StandardMaterial,
    occlusion: f32,
    frag_coord: vec4<f32>,
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    N: vec3<f32>,
    V: vec3<f32>,
    is_orthographic: bool,
}

fn pbr_functions__pbr(in: pbr_functions__PbrInput) -> vec4<f32> {
    var output_color_2: vec4<f32> = in.material.base_color;

    if ((in.material.flags & 64u) != 0u) {
        output_color_2.w = 1.0;
    } else {
        if ((in.material.flags & 128u) != 0u) {
            let _e52: f32 = output_color_2.w;
            if (_e52 >= in.material.alpha_cutoff) {
                output_color_2.w = 1.0;
            } else {
                discard;
            }
        }
    }

    return output_color_2;
}