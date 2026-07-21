//! Example of a fragment shader for an ExtendedMaterial<StandardMaterial, E> setup to
//! correctly handle order independent transparency.
//! See examples/3d/order_independent_transpatency.rs

// In our case we're always transparent and so forward rendered, but
// this is the general setup if it's not always the case.
@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::{VertexOutput, FragmentOutput},
};
@else // PREPASS_PIPELINE
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
};

@if(MATERIAL_OIT_ENABLED)
import bevy_core_pipeline::oit::draw::oit_draw;
@if(MATERIAL_OIT_ENABLED)
import bevy_pbr::render::pbr_types;
@if(MATERIAL_OIT_ENABLED)
import bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material;

// The material parameters
struct Colors {
    color1: vec4<f32>,
    color2: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> material: Colors;

@fragment
fn fragment(
    in: VertexOutput,
) -> FragmentOutput {
    var out: FragmentOutput;

    // This produces a checkered pattern along the uv isolines filled with the two colors
    let grid = vec2u(in.uv * 30.0f) % 2;
    out.color = select(material.color1, material.color2, (grid.x + grid.y == 1));

@if(MATERIAL_OIT_ENABLED) {
    let pbr_input = pbr_input_from_standard_material(in, false);
    let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, vec4(out.color.rgb * out.color.a, out.color.a));
        discard;
    }
    // Both `Premultiplied` and `Add` colors are premultiplied in `premultiply_alpha()`
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED
        || alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, out.color);
        discard;
    }
}

    return out;
}
