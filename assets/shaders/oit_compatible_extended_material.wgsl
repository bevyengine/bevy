//! Example of a fragment shader for an ExtendedMaterial<StandardMaterial, E> setup to
//! correctly handle order independent transparency.
//! See examples/3d/order_independent_transpatency.rs

// In our case we're always transparent and so forward rendered, but
// this is the general setup if it's not always the case.
#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
}
#else // PREPASS_PIPELINE
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
}
#endif // PREPASS_PIPELINE

#ifdef OIT_ENABLED
#import bevy_core_pipeline::oit::oit_draw
#import bevy_pbr::pbr_types
#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#endif // OIT_ENABLED

// The material parameters
struct Colors {
    color1: vec4<f32>,
    color2: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> material: Colors;

@fragment
fn fragment(
    in: VertexOutput,
) -> FragmentOutput {
    var out: FragmentOutput;

    // This produces a checkered pattern along the uv isolines filled with the two colors
    let grid = vec2u(in.uv * 30.0f) % 2;
    out.color = select(material.color1, material.color2, (grid.x + grid.y == 1));

#ifdef OIT_ENABLED
    let flags = pbr_input_from_standard_material(in, false).flags;
    let alpha_mode = flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode != pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, out.color);
        discard;
    }
#endif // OIT_ENABLED

    return out;
}
