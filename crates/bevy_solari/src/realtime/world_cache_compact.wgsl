#import bevy_solari::realtime_bindings::{ 
    world_cache_checksums, 
    world_cache_life, 
    world_cache_radiance, 
    world_cache_a, 
    world_cache_b, 
    world_cache_active_cell_indices,
    world_cache_active_cells_count, 
    world_cache_light_data, 
    world_cache_light_data_new_lights, 
    WORLD_CACHE_CELL_LIGHT_COUNT,
    WorldCacheSingleLightData, 
}
#import bevy_solari::world_cache::WORLD_CACHE_EMPTY_CELL

@group(2) @binding(0) var<storage, read_write> world_cache_active_cells_dispatch: vec3<u32>;

var<workgroup> w1: array<u32, 1024u>;
var<workgroup> w2: array<u32, 1024u>;

@compute @workgroup_size(1024, 1, 1)
fn decay_world_cache(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var life = world_cache_life[global_id.x];
    if life > 0u {
        life -= 1u;
        world_cache_life[global_id.x] = life;

        if life == 0u {
            world_cache_checksums[global_id.x] = WORLD_CACHE_EMPTY_CELL;
            world_cache_radiance[global_id.x] = vec4(0.0);
        } else {
            let old_count = world_cache_light_data[global_id.x].visible_light_count;
            let old_lights = world_cache_light_data[global_id.x].visible_lights;
            let new_count = min(WORLD_CACHE_CELL_LIGHT_COUNT, atomicLoad(&world_cache_light_data_new_lights[global_id.x].visible_light_count));
            atomicStore(&world_cache_light_data_new_lights[global_id.x].visible_light_count, 0u);
            var out_i = 0u;
            var out_lights: array<WorldCacheSingleLightData, WORLD_CACHE_CELL_LIGHT_COUNT>;

            for (var i = 0u; i < new_count; i++) {
                let data = atomicLoad(&world_cache_light_data_new_lights[global_id.x].visible_lights[i]);
                atomicStore(&world_cache_light_data_new_lights[global_id.x].visible_lights[i], 0);
                if data == 0 { 
                    break; 
                }

                let light = u32(data & 0xffffffff);
                let weight = bitcast<f32>(u32(data >> 32u));
                var exist_index = 0u;
                if is_light_in_array(out_lights, out_i, light, &exist_index) {
                    out_lights[exist_index].weight = max(out_lights[exist_index].weight, weight);
                } else {
                    out_lights[out_i] = WorldCacheSingleLightData(light, weight);
                    out_i++;
                }
            }
            for (var i = 0u; i < old_count && out_i < WORLD_CACHE_CELL_LIGHT_COUNT; i++) {
                var exist_index = 0u;
                if is_light_in_array(out_lights, out_i, old_lights[i].light, &exist_index) {
                    out_lights[exist_index].weight = max(out_lights[exist_index].weight, old_lights[i].weight);
                } else {
                    out_lights[out_i] = old_lights[i];
                    out_i++;
                }
            }
            world_cache_light_data[global_id.x].visible_light_count = out_i;
            world_cache_light_data[global_id.x].visible_lights = out_lights;
        }
    }
}

fn is_light_in_array(arr: array<WorldCacheSingleLightData, WORLD_CACHE_CELL_LIGHT_COUNT>, len: u32, light: u32, out_index: ptr<function, u32>) -> bool {
    for (var i = 0u; i < len; i++) {
        if arr[i].light == light {
            *out_index = i;
            return true;
        }
    }
    return false;
}

@compute @workgroup_size(1024, 1, 1)
fn compact_world_cache_single_block(
    @builtin(global_invocation_id) cell_id: vec3<u32>,
    @builtin(local_invocation_index) t: u32,
) {
    if t == 0u { w1[0u] = 0u; } else { w1[t] = u32(world_cache_life[cell_id.x - 1u] != 0u); }; workgroupBarrier();
    if t < 1u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 1u]; } workgroupBarrier();
    if t < 2u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 2u]; } workgroupBarrier();
    if t < 4u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 4u]; } workgroupBarrier();
    if t < 8u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 8u]; } workgroupBarrier();
    if t < 16u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 16u]; } workgroupBarrier();
    if t < 32u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 32u]; } workgroupBarrier();
    if t < 64u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 64u]; } workgroupBarrier();
    if t < 128u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 128u]; } workgroupBarrier();
    if t < 256u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 256u]; } workgroupBarrier();
    if t < 512u { world_cache_a[cell_id.x] = w2[t]; } else { world_cache_a[cell_id.x] = w2[t] + w2[t - 512u]; }
}

@compute @workgroup_size(1024, 1, 1)
fn compact_world_cache_blocks(@builtin(local_invocation_index) t: u32) {
    if t == 0u { w1[0u] = 0u; } else { w1[t] = world_cache_a[t * 1024u - 1u]; }; workgroupBarrier();
    if t < 1u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 1u]; } workgroupBarrier();
    if t < 2u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 2u]; } workgroupBarrier();
    if t < 4u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 4u]; } workgroupBarrier();
    if t < 8u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 8u]; } workgroupBarrier();
    if t < 16u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 16u]; } workgroupBarrier();
    if t < 32u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 32u]; } workgroupBarrier();
    if t < 64u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 64u]; } workgroupBarrier();
    if t < 128u { w1[t] = w2[t]; } else { w1[t] = w2[t] + w2[t - 128u]; } workgroupBarrier();
    if t < 256u { w2[t] = w1[t]; } else { w2[t] = w1[t] + w1[t - 256u]; } workgroupBarrier();
    if t < 512u { world_cache_b[t] = w2[t]; } else { world_cache_b[t] = w2[t] + w2[t - 512u]; }
}

@compute @workgroup_size(1024, 1, 1)
fn compact_world_cache_write_active_cells(
    @builtin(global_invocation_id) cell_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_index) thread_index: u32,
) {
    let compacted_index = world_cache_a[cell_id.x] + world_cache_b[workgroup_id.x];
    if world_cache_life[cell_id.x] != 0u {
        world_cache_active_cell_indices[compacted_index] = cell_id.x;
    }

    if thread_index == 1023u && workgroup_id.x == 1023u {
        world_cache_active_cells_count = compacted_index + 1u; // TODO: This is 1 even when there are zero active entries in the cache
        world_cache_active_cells_dispatch = vec3((world_cache_active_cells_count + 1023u) / 1024u, 1u, 1u);
    }
}
