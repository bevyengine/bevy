/// Remaps an indirect 1d to 2d dispatch for devices with low dispatch size limit.

struct DispatchIndirectArgs {
    x: u32,
    y: u32,
    z: u32,
}

@group(0) @binding(0) var<storage, read_write> meshlet_software_raster_indirect_args: DispatchIndirectArgs;
@group(0) @binding(1) var<storage, read_write> meshlet_software_raster_cluster_count: u32;

@compute
@workgroup_size(1, 1, 1)
fn remap_dispatch() {
    meshlet_software_raster_cluster_count = meshlet_software_raster_indirect_args.x;

    let n = u32(ceil(sqrt(f32(meshlet_software_raster_indirect_args.x))));
    meshlet_software_raster_indirect_args.x = n;
    meshlet_software_raster_indirect_args.y = n;
}
