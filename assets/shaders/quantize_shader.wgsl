#import bevy_pbr::pbr_fragment          pbr_fragment
#import bevy_pbr::mesh_vertex_output    MeshVertexOutput

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
    // call to the standard pbr fragment shader
    var output_color = pbr_fragment(in, is_front);

    // we can then modify the results using the extended material data
    output_color = vec4<f32>(vec4<u32>(output_color * f32(my_extended_material.quantize_steps))) / f32(my_extended_material.quantize_steps);
    return output_color;
}
