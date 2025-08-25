#define_import_path bevy_pbr::view_transformations

#import bevy_pbr::mesh_view_bindings as view_bindings
#import bevy_pbr::prepass_bindings
#import bevy_render::view

/// World space:
/// +y is up

/// View space:
/// -z is forward, +x is right, +y is up
/// Forward is from the camera position into the scene.
/// (0.0, 0.0, -1.0) is linear distance of 1.0 in front of the camera's view relative to the camera's rotation
/// (0.0, 1.0, 0.0) is linear distance of 1.0 above the camera's view relative to the camera's rotation

/// NDC (normalized device coordinate):
/// https://www.w3.org/TR/webgpu/#coordinate-systems
/// (-1.0, -1.0) in NDC is located at the bottom-left corner of NDC
/// (1.0, 1.0) in NDC is located at the top-right corner of NDC
/// Z is depth where: 
///    1.0 is near clipping plane
///    Perspective projection: 0.0 is inf far away
///    Orthographic projection: 0.0 is far clipping plane

/// Clip space:
/// This is NDC before the perspective divide, still in homogenous coordinate space.
/// Dividing a clip space point by its w component yields a point in NDC space.

/// UV space:
/// 0.0, 0.0 is the top left
/// 1.0, 1.0 is the bottom right


// -----------------
// TO WORLD --------
// -----------------

/// Convert a view space position to world space
/// DEPRECATED: use bevy_render::view::position_view_to_world instead
fn position_view_to_world(view_pos: vec3<f32>) -> vec3<f32> {
    return view::position_view_to_world(view_pos, view_bindings::view.world_from_view);
}

/// Convert a clip space position to world space
/// DEPRECATED: use bevy_render::view::position_clip_to_world instead
fn position_clip_to_world(clip_pos: vec4<f32>) -> vec3<f32> {
    return view::position_clip_to_world(clip_pos, view_bindings::view.world_from_clip);
}

/// Convert a ndc space position to world space
/// DEPRECATED: use bevy_render::view::position_ndc_to_world instead
fn position_ndc_to_world(ndc_pos: vec3<f32>) -> vec3<f32> {
    return view::position_ndc_to_world(ndc_pos, view_bindings::view.world_from_clip);
}

/// Convert a view space direction to world space
/// DEPRECATED: use bevy_render::view::direction_view_to_world instead
fn direction_view_to_world(view_dir: vec3<f32>) -> vec3<f32> {
    return view::direction_view_to_world(view_dir, view_bindings::view.world_from_view);
}

/// Convert a clip space direction to world space
/// DEPRECATED: use bevy_render::view::direction_clip_to_world instead
fn direction_clip_to_world(clip_dir: vec4<f32>) -> vec3<f32> {
    return view::direction_clip_to_world(clip_dir, view_bindings::view.world_from_clip);
}

// -----------------
// TO VIEW ---------
// -----------------

/// Convert a world space position to view space
/// DEPRECATED: use bevy_render::view::position_world_to_view instead
fn position_world_to_view(world_pos: vec3<f32>) -> vec3<f32> {
    return view::position_world_to_view(world_pos, view_bindings::view.view_from_world);
}

/// Convert a clip space position to view space
/// DEPRECATED: use bevy_render::view::position_clip_to_view instead
fn position_clip_to_view(clip_pos: vec4<f32>) -> vec3<f32> {
    return view::position_clip_to_view(clip_pos, view_bindings::view.view_from_clip);
}

/// Convert a ndc space position to view space
/// DEPRECATED: use bevy_render::view::position_ndc_to_view instead
fn position_ndc_to_view(ndc_pos: vec3<f32>) -> vec3<f32> {
    return view::position_ndc_to_view(ndc_pos, view_bindings::view.view_from_clip);
}

/// Convert a world space direction to view space
/// DEPRECATED: use bevy_render::view::direction_world_to_view instead
fn direction_world_to_view(world_dir: vec3<f32>) -> vec3<f32> {
    return view::direction_world_to_view(world_dir, view_bindings::view.view_from_world);
}

/// Convert a clip space direction to view space
/// DEPRECATED: use bevy_render::view::direction_clip_to_view instead
fn direction_clip_to_view(clip_dir: vec4<f32>) -> vec3<f32> {
    return view::direction_clip_to_view(clip_dir, view_bindings::view.view_from_clip);
}

// -----------------
// TO PREV. VIEW ---
// -----------------

/// DEPRECATED: use bevy_render::view::position_world_to_view instead
fn position_world_to_prev_view(world_pos: vec3<f32>) -> vec3<f32> {
    return view::position_world_to_view(world_pos, prepass_bindings::previous_view_uniforms.view_from_world);
}

/// DEPRECATED: use bevy_render::view::position_world_to_ndc instead
fn position_world_to_prev_ndc(world_pos: vec3<f32>) -> vec3<f32> {
    return view::position_world_to_ndc(world_pos, prepass_bindings::previous_view_uniforms.clip_from_world);
}

// -----------------
// TO CLIP ---------
// -----------------

/// Convert a world space position to clip space
/// DEPRECATED: use bevy_render::view::position_world_to_clip instead
fn position_world_to_clip(world_pos: vec3<f32>) -> vec4<f32> {
    return view::position_world_to_clip(world_pos, view_bindings::view.clip_from_world);
}

/// Convert a view space position to clip space
/// DEPRECATED: use bevy_render::view::position_view_to_clip instead
fn position_view_to_clip(view_pos: vec3<f32>) -> vec4<f32> {
    return view::position_view_to_clip(view_pos, view_bindings::view.clip_from_view);
}

/// Convert a world space direction to clip space
/// DEPRECATED: use bevy_render::view::direction_world_to_clip instead
fn direction_world_to_clip(world_dir: vec3<f32>) -> vec4<f32> {
    return view::direction_world_to_clip(world_dir, view_bindings::view.clip_from_world);
}

/// Convert a view space direction to clip space
/// DEPRECATED: use bevy_render::view::direction_view_to_clip instead
fn direction_view_to_clip(view_dir: vec3<f32>) -> vec4<f32> {
    return view::direction_view_to_clip(view_dir, view_bindings::view.clip_from_view);
}

// -----------------
// TO NDC ----------
// -----------------

/// Convert a world space position to ndc space
/// DEPRECATED: use bevy_render::view::position_world_to_ndc instead
fn position_world_to_ndc(world_pos: vec3<f32>) -> vec3<f32> {
    return view::position_world_to_ndc(world_pos, view_bindings::view.clip_from_world);
}

/// Convert a view space position to ndc space
/// DEPRECATED: use bevy_render::view::position_view_to_ndc instead
fn position_view_to_ndc(view_pos: vec3<f32>) -> vec3<f32> {
    return view::position_view_to_ndc(view_pos, view_bindings::view.clip_from_view);
}

// -----------------
// DEPTH -----------
// -----------------

/// Retrieve the perspective camera near clipping plane
/// DEPRECATED: use bevy_render::view::perspective_camera_near instead
fn perspective_camera_near() -> f32 {
    return view::perspective_camera_near(view_bindings::view.clip_from_view);
}

/// Convert ndc depth to linear view z. 
/// Note: Depth values in front of the camera will be negative as -z is forward
/// DEPRECATED: use bevy_render::view::depth_ndc_to_view_z instead
fn depth_ndc_to_view_z(ndc_depth: f32) -> f32 {
    return view::depth_ndc_to_view_z(ndc_depth, view_bindings::view.clip_from_view, view_bindings::view.view_from_clip);
}

/// Convert linear view z to ndc depth. 
/// Note: View z input should be negative for values in front of the camera as -z is forward
/// DEPRECATED: use bevy_render::view::view_z_to_depth_ndc instead
fn view_z_to_depth_ndc(view_z: f32) -> f32 {
    return view::view_z_to_depth_ndc(view_z, view_bindings::view.clip_from_view);
}

/// DEPRECATED: use bevy_render::view::prev_view_z_to_depth_ndc instead
fn prev_view_z_to_depth_ndc(view_z: f32) -> f32 {
    return view::view_z_to_depth_ndc(view_z, prepass_bindings::previous_view_uniforms.clip_from_view);
}

// -----------------
// UV --------------
// -----------------

/// Convert ndc space xy coordinate [-1.0 .. 1.0] to uv [0.0 .. 1.0]
/// DEPRECATED: use bevy_render::view::ndc_to_uv instead
fn ndc_to_uv(ndc: vec2<f32>) -> vec2<f32> {
    return view::ndc_to_uv(ndc);
}

/// Convert uv [0.0 .. 1.0] coordinate to ndc space xy [-1.0 .. 1.0]
/// DEPRECATED: use bevy_render::view::uv_to_ndc instead
fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return view::uv_to_ndc(uv);
}

/// returns the (0.0, 0.0) .. (1.0, 1.0) position within the viewport for the current render target
/// [0 .. render target viewport size] eg. [(0.0, 0.0) .. (1280.0, 720.0)] to [(0.0, 0.0) .. (1.0, 1.0)]
/// DEPRECATED: use bevy_render::view::frag_coord_to_uv instead
fn frag_coord_to_uv(frag_coord: vec2<f32>) -> vec2<f32> {
    return view::frag_coord_to_uv(frag_coord, view_bindings::view.viewport);
}

/// Convert frag coord to ndc
/// DEPRECATED: use bevy_render::view::frag_coord_to_ndc instead
fn frag_coord_to_ndc(frag_coord: vec4<f32>) -> vec3<f32> {
    return view::frag_coord_to_ndc(frag_coord, view_bindings::view.viewport);
}

/// Convert ndc space xy coordinate [-1.0 .. 1.0] to [0 .. render target
/// viewport size]
/// DEPRECATED: use bevy_render::view::ndc_to_frag_coord instead
fn ndc_to_frag_coord(ndc: vec2<f32>) -> vec2<f32> {
    return view::ndc_to_frag_coord(ndc, view_bindings::view.viewport);
}
