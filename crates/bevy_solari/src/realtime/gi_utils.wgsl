#define_import_path bevy_solari::gi_utils

#import bevy_pbr::utils::rand_f
#import bevy_solari::realtime_bindings::{view, constants}

fn gi_resolution() -> vec2<u32> {
    if bool(constants.quarter_resolution_indirect_lighting) {
        return quarter_resolution_dimensions();
    } else {
        return vec2u(view.main_pass_viewport.zw);
    }
}

fn gi_thread_to_full_resolution_pixel(thread_xy: vec2<u32>) -> vec2<u32> {
    if bool(constants.quarter_resolution_indirect_lighting) {
        return quarter_to_full_resolution_pixel(thread_xy, constants.frame_index);
    } else {
        return thread_xy;
    }
}

fn gi_snap_to_quad_pixel(full_xy: vec2<u32>) -> vec2<u32> {
    if bool(constants.quarter_resolution_indirect_lighting) {
        return quarter_to_full_resolution_pixel(full_xy / 2u, constants.frame_index);
    } else {
        return full_xy;
    }
}

fn gi_snap_to_quad_pixel_previous_frame(full_xy: vec2<u32>) -> vec2<u32> {
    if bool(constants.quarter_resolution_indirect_lighting) {
        return quarter_to_full_resolution_pixel(full_xy / 2u, constants.frame_index - 5782582u);
    } else {
        return full_xy;
    }
}

fn gi_reservoir_index(full_xy: vec2<u32>) -> u32 {
    if bool(constants.quarter_resolution_indirect_lighting) {
        return quarter_resolution_index(full_xy / 2u);
    } else {
        return full_xy.x + full_xy.y * u32(view.main_pass_viewport.z);
    }
}

fn quarter_resolution_dimensions() -> vec2<u32> {
    return (vec2u(view.main_pass_viewport.zw) + 1u) / 2u;
}

fn quarter_resolution_index(quarter_xy: vec2<u32>) -> u32 {
    return quarter_xy.x + quarter_xy.y * quarter_resolution_dimensions().x;
}

fn quarter_to_full_resolution_pixel(quarter_xy: vec2<u32>, frame: u32) -> vec2<u32> {
    var rng = quarter_resolution_index(quarter_xy) * 0x9E3779B9u + frame;
    let qi = u32(rand_f(&rng) * 4.0);
    return min(quarter_xy * 2u + vec2(qi / 2u, qi % 2u), vec2u(view.main_pass_viewport.zw) - 1u);
}
