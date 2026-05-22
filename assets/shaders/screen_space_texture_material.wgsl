// A shader that samples its emissive texture at screen space positions rather
// than UVs.
//
// This is used for the mirror example: `examples/3d/mirror.rs`.

#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_bindings::emissive_texture,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing}
}

// This is unused, but it's here to satisfy `ExtendedMaterial` requirements.
// See the comment in `ScreenSpaceTextureExtension` in `examples/3d/mirror.rs`
// for more information.
struct ScreenSpaceTextureMaterial {
    // An unused value.
    dummy: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> material: ScreenSpaceTextureMaterial;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Use zero alpha to avoid multiplying the emissive by the view exposure.
    pbr_input.material.emissive = vec4(
        textureLoad(emissive_texture, vec2<i32>(floor(in.position.xy)), 0).rgb,
        0.0
    );

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
