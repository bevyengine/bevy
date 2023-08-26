#import bevy_pbr::mesh_vertex_output    MeshVertexOutput
#import bevy_pbr::pbr_functions         PbrInput, apply_pbr_lighting, alpha_discard
#import bevy_pbr::pbr_fragment          standard_material_pbr_input, in_shader_post_processing
#import bevy_pbr::pbr_types             STANDARD_MATERIAL_FLAGS_UNLIT_BIT

@fragment
fn fragment(
    in: MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    // generate a PbrInput struct from the StandardMaterial bindings
    let pbr_input = standard_material_pbr_input(in, is_front);

    // apply lighting
    var lit_color: vec4<f32>;

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if (pbr_input.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        lit_color = apply_pbr_lighting(pbr_input);
    } else {
        lit_color = alpha_discard(pbr_input.material, lit_color);
    }

    // apply in-shader post processing (fog, tonemapping, debanding)
    lit_color = in_shader_post_processing(pbr_input, lit_color);

    return lit_color;
}
