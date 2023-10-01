#import bevy_pbr::mesh_vertex_output    MeshVertexOutput
#import bevy_pbr::pbr_functions         PbrInput, apply_pbr_lighting
#import bevy_pbr::pbr_fragment          pbr_input_from_standard_material, in_shader_post_processing

struct MyExtendedMaterial {
    quantize_steps: u32,
}

@group(1) @binding(100)
var<uniform> my_extended_material: MyExtendedMaterial;

@fragment
fn fragment(
    in: MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // we can optionally modify the input before lighting is applied
    pbr_input.material.base_color.b = pbr_input.material.base_color.r;

    // apply lighting
    let lit_color = apply_pbr_lighting(pbr_input);

    // we can optionally modify the lit color before post-processing is applied
    let modified_lit_color = vec4<f32>(vec4<u32>(lit_color * f32(my_extended_material.quantize_steps))) / f32(my_extended_material.quantize_steps);

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    let output_color = in_shader_post_processing(pbr_input, modified_lit_color);

    // we can optionally modify the final result here
    let modified_output_color = output_color * 2.0;

    return modified_output_color;
}
