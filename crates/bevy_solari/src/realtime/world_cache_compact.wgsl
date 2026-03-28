enable wgpu_ray_query;

#import bevy_solari::world_cache::WORLD_CACHE_EMPTY_CELL
#import bevy_solari::realtime_bindings::world_cache

var<workgroup> w1: array<u32, 1024u>;
var<workgroup> w2: array<u32, 1024u>;

@compute @workgroup_size(1024, 1, 1)
fn decay_world_cache(@builtin(global_invocation_id) global_id: vec3<u32>) {
#ifdef WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER
    var life = world_cache.life[global_id.x];
    if life > 0u {
        life -= 1u;
        world_cache.life[global_id.x] = life;
        if life == 0u {
            atomicStore(&world_cache.checksums[global_id.x], WORLD_CACHE_EMPTY_CELL);
            world_cache.radiance[global_id.x] = vec4(0.0);
            world_cache.luminance_deltas[global_id.x] = 0.0;
        }
    }
#else
    var life = atomicLoad(&world_cache.life[global_id.x]);
    if life > 0u {
        life -= 1u;
        atomicStore(&world_cache.life[global_id.x], life);
        if life == 0u {
            atomicStore(&world_cache.checksums[global_id.x], WORLD_CACHE_EMPTY_CELL);
            world_cache.radiance[global_id.x] = vec4(0.0);
            world_cache.luminance_deltas[global_id.x] = 0.0;
        }
    }
#endif
}

@compute @workgroup_size(1024, 1, 1)
fn compact_world_cache_single_block(
    @builtin(global_invocation_id) cell_id: vec3<u32>,
    @builtin(local_invocation_index) t: u32,
) {
#ifdef WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER
    if t == 0u { w1[0u] = 0u; } else { w1[t] = u32(world_cache.life[cell_id.x - 1u] != 0u); }; workgroupBarrier();
#else
    if t == 0u { w1[0u] = 0u; } else { w1[t] = u32(atomicLoad(&world_cache.life[cell_id.x - 1u]) != 0u); }; workgroupBarrier();
#endif
    if t < 1u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 1u]; } workgroupBarrier();
    if t < 2u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 2u]; } workgroupBarrier();
    if t < 4u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 4u]; } workgroupBarrier();
    if t < 8u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 8u]; } workgroupBarrier();
    if t < 16u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 16u]; } workgroupBarrier();
    if t < 32u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 32u]; } workgroupBarrier();
    if t < 64u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 64u]; } workgroupBarrier();
    if t < 128u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 128u]; } workgroupBarrier();
    if t < 256u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 256u]; } workgroupBarrier();
    if t < 512u { world_cache.a[cell_id.x] = w2[t]; } else { world_cache.a[cell_id.x] = w2[t] + w2[t - 512u]; }
}

@compute @workgroup_size(1024, 1, 1)
fn compact_world_cache_blocks(@builtin(local_invocation_index) t: u32) {
    if t == 0u { w1[0u] = 0u; } else { w1[t] = world_cache.a[t * 1024u - 1u]; }; workgroupBarrier();
    if t < 1u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 1u]; } workgroupBarrier();
    if t < 2u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 2u]; } workgroupBarrier();
    if t < 4u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 4u]; } workgroupBarrier();
    if t < 8u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 8u]; } workgroupBarrier();
    if t < 16u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 16u]; } workgroupBarrier();
    if t < 32u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 32u]; } workgroupBarrier();
    if t < 64u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 64u]; } workgroupBarrier();
    if t < 128u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 128u]; } workgroupBarrier();
    if t < 256u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 256u]; } workgroupBarrier();
    if t < 512u { world_cache.b[t] = w2[t]; } else { world_cache.b[t] = w2[t] + w2[t - 512u]; }
}

@compute @workgroup_size(1024, 1, 1)
fn compact_world_cache_write_active_cells(
    @builtin(global_invocation_id) cell_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_index) thread_index: u32,
) {
    let compacted_index = world_cache.a[cell_id.x] + world_cache.b[workgroup_id.x];
#ifdef WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER
    let cell_active = world_cache.life[cell_id.x] != 0u;
#else
    let cell_active = atomicLoad(&world_cache.life[cell_id.x]) != 0u;
#endif

    if cell_active {
        world_cache.active_cell_indices[compacted_index] = cell_id.x;
    }

    if thread_index == 1023u && workgroup_id.x == 1023u {
        let active_cell_count = compacted_index + u32(cell_active);
        world_cache.active_cells_count = active_cell_count;
        world_cache.indirect_dispatch_x = (active_cell_count + 63u) / 64u;
        world_cache.indirect_dispatch_y = 1u;
        world_cache.indirect_dispatch_z = 1u;
    }
}
