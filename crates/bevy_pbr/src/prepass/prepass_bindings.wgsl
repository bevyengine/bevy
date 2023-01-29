#define_import_path bevy_pbr::prepass_bindings

#import bevy_pbr::mesh_view_types
#import bevy_pbr::mesh_types

@group(0) @binding(0)
var<uniform> view: bevy_pbr::mesh_view_types::View;

// Material bindings will be in @group(1)

@group(2) @binding(0)
var<uniform> mesh: bevy_pbr::mesh_types::Mesh;

#ifdef SKINNED
@group(2) @binding(1)
var<uniform> joint_matrices: bevy_pbr::mesh_types::SkinnedMesh;
#import bevy_pbr::skinning
#endif
