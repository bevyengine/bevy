// This shader shows how to enable the gpu picking feature for a material

// You'll need the mesh binding because that's where the entity index is
#import bevy_pbr::mesh_bindings mesh
#import bevy_pbr::mesh_vertex_output MeshVertexOutput

@group(1) @binding(0)
var<uniform> color: vec4<f32>;

// Gpu picking uses multiple fragment output
struct FragmentOutput {
    @location(0) color: vec4<f32>,
// You can detect the feature with this flag
#ifdef GPU_PICKING
    @location(1) mesh_id: u32,
#endif
};

@fragment
fn fragment(in: MeshVertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = color;
// make sure to output the entity index for gpu picking to work correctly
#ifdef GPU_PICKING
    out.mesh_id = mesh[in.instance_index].id;
#endif
    return out;
}
