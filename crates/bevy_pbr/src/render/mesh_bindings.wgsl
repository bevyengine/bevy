#define_import_path bevy_pbr::mesh_bindings

#import bevy_pbr::mesh_types Mesh

#ifdef PER_OBJECT_BUFFER_BATCH_SIZE

    @group(#{MESH_BINDGROUP}) @binding(0)
    var<uniform> mesh: array<Mesh, #{PER_OBJECT_BUFFER_BATCH_SIZE}u>;
#else

    @group(#{MESH_BINDGROUP}) @binding(0)
    var<storage> mesh: array<Mesh>;

#endif // PER_OBJECT_BUFFER_BATCH_SIZE
