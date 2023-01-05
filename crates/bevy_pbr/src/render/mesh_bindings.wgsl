#define_import_path bevy_pbr::mesh_bindings

#import bevy_pbr::mesh_types

@group(2) @binding(0)
var<uniform> mesh: Mesh;
#ifdef SKINNED
@group(2) @binding(1)
var<uniform> joint_matrices: SkinnedMesh;
#import bevy_pbr::skinning
#endif
