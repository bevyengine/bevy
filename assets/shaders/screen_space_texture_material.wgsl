#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_bindings::base_color_texture,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing}
}

struct ScreenSpaceTextureMaterial {
    screen_rect: vec4<f32>,
}

@group(2) @binding(100) var<uniform> material: ScreenSpaceTextureMaterial;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let screen_rect = material.screen_rect;

    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = textureLoad(
        base_color_texture,
        vec2<i32>(floor(in.position.xy) - screen_rect.xy),
        0
    );

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
