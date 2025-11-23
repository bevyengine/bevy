#import bevy_solari::realtime_bindings::{ 
    world_cache_checksums, 
    world_cache_life, 
    world_cache_radiance, 
    world_cache_light_data, 
    world_cache_light_data_new_lights, 
    WORLD_CACHE_CELL_LIGHT_COUNT,
    WorldCacheSingleLightData, 
}
#import bevy_solari::world_cache::WORLD_CACHE_EMPTY_CELL

@compute @workgroup_size(64, 1, 1)
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
            let new_count = min(WORLD_CACHE_CELL_LIGHT_COUNT, world_cache_light_data_new_lights[global_id.x].visible_light_count);
            world_cache_light_data_new_lights[global_id.x].visible_light_count = 0u;
            var out_i = 0u;
            var out_lights: array<WorldCacheSingleLightData, WORLD_CACHE_CELL_LIGHT_COUNT>;

            for (var i = 0u; i < new_count; i++) {
                let data = world_cache_light_data_new_lights[global_id.x].visible_lights[i];
                world_cache_light_data_new_lights[global_id.x].visible_lights[i] = WorldCacheSingleLightData(0, 0.0);
                if data.weight == 0.0 { 
                    break; 
                }

                var exist_index = 0u;
                if is_light_in_array(out_lights, out_i, data.light, &exist_index) {
                    out_lights[exist_index].weight = max(out_lights[exist_index].weight, data.weight);
                } else {
                    out_lights[out_i] = data;
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
