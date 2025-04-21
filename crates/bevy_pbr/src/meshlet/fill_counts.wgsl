/// Copies the counts of meshlets in the hardware and software buckets, resetting the counters in the process.

struct DispatchIndirectArgs {
    x: u32,
    y: u32,
    z: u32,
}

struct DrawIndirectArgs {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

@group(0) @binding(0) var<storage, read_write> meshlet_software_raster_indirect_args: DispatchIndirectArgs;
@group(0) @binding(1) var<storage, read_write> meshlet_hardware_raster_indirect_args: DrawIndirectArgs;
@group(0) @binding(2) var<storage, read_write> meshlet_previous_raster_counts: array<u32>;

@compute
@workgroup_size(1, 1, 1)
fn fill_counts() {
    meshlet_previous_raster_counts[0] += meshlet_software_raster_indirect_args.x;
    meshlet_previous_raster_counts[1] += meshlet_hardware_raster_indirect_args.instance_count;
    meshlet_software_raster_indirect_args.x = 0;
    meshlet_hardware_raster_indirect_args.instance_count = 0;
}
