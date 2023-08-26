#import bevy_pbr::pbr_fragment          pbr_fragment
#import bevy_pbr::mesh_vertex_output    MeshVertexOutput

@fragment
fn fragment(
    in: MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    return pbr_fragment(in, is_front);
}
