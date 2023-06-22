#define_import_path bevy_pbr::mesh_bindings

#import bevy_pbr::mesh_types

@group(2) @binding(0)
var<uniform> mesh: Mesh;

#ifdef SKINNED
@group(2) @binding(1)
var<uniform> joint_matrices: SkinnedMesh;
#import bevy_pbr::skinning
#endif

#ifdef MORPH_TARGETS
@group(2) @binding(2)
var<uniform> morph_weights: MorphWeights;
@group(2) @binding(3)
var morph_targets: texture_3d<f32>;
#import bevy_pbr::morph
#endif
