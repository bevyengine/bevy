#define_import_path bevy_sprite::mesh2d_bindings

#import bevy_sprite::mesh2d_types::Mesh2d

#ifdef PER_OBJECT_BUFFER_BATCH_SIZE
@group(1) @binding(0) var<uniform> mesh: array<Mesh2d, #{PER_OBJECT_BUFFER_BATCH_SIZE}u>;
#else
@group(1) @binding(0) var<storage> mesh: array<Mesh2d>;
#endif // PER_OBJECT_BUFFER_BATCH_SIZE
