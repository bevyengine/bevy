#define_import_path bevy_pbr::prepass_bindings
#import bevy_pbr::mesh_types

#ifdef MOTION_VECTOR_PREPASS
@group(0) @binding(2) var<uniform> previous_view_proj: mat4x4<f32>;
#endif // MOTION_VECTOR_PREPASS

// Material bindings will be in @group(1)
#import bevy_pbr::mesh_bindings   mesh
