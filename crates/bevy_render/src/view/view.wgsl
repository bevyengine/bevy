#define_import_path bevy_render::view

struct ColorGrading {
    balance: mat3x3<f32>,
    saturation: vec3<f32>,
    contrast: vec3<f32>,
    gamma: vec3<f32>,
    gain: vec3<f32>,
    lift: vec3<f32>,
    midtone_range: vec2<f32>,
    exposure: f32,
    hue: f32,
    post_saturation: f32,
}

struct View {
    clip_from_world: mat4x4<f32>,
    unjittered_clip_from_world: mat4x4<f32>,
    world_from_clip: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
    view_from_world: mat4x4<f32>,
    // Typically a column-major right-handed projection matrix, one of either:
    //
    // Perspective (infinite reverse z)
    // ```
    // f = 1 / tan(fov_y_radians / 2)
    //
    // ⎡ f / aspect  0   0     0 ⎤
    // ⎢          0  f   0     0 ⎥
    // ⎢          0  0   0  near ⎥
    // ⎣          0  0  -1     0 ⎦
    // ```
    //
    // Orthographic
    // ```
    // w = right - left
    // h = top - bottom
    // d = far - near
    // cw = -right - left
    // ch = -top - bottom
    //
    // ⎡ 2 / w      0      0   cw / w ⎤
    // ⎢     0  2 / h      0   ch / h ⎥
    // ⎢     0      0  1 / d  far / d ⎥
    // ⎣     0      0      0        1 ⎦
    // ```
    //
    // `clip_from_view[3][3] == 1.0` is the standard way to check if a projection is orthographic
    //
    // Wgsl matrices are column major, so for example getting the near plane of a perspective projection is `clip_from_view[3][2]`
    //
    // Custom projections are also possible however.
    clip_from_view: mat4x4<f32>,
    view_from_clip: mat4x4<f32>,
    world_position: vec3<f32>,
    exposure: f32,
    // viewport(x_origin, y_origin, width, height)
    viewport: vec4<f32>,
    main_pass_viewport: vec4<f32>,
    // 6 world-space half spaces (normal: vec3, distance: f32) ordered left, right, top, bottom, near, far.
    // The normal vectors point towards the interior of the frustum.
    // A half space contains `p` if `normal.dot(p) + distance > 0.`
    frustum: array<vec4<f32>, 6>,
    color_grading: ColorGrading,
    mip_bias: f32,
    frame_count: u32,
};

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

/// UV space:
/// 0.0, 0.0 is the top left
/// 1.0, 1.0 is the bottom right


// -----------------
// TO WORLD --------
// -----------------

/// Convert a view space position to world space
fn position_view_to_world(view_pos: vec3<f32>, world_from_view: mat4x4<f32>) -> vec3<f32> {
    let world_pos = world_from_view * vec4(view_pos, 1.0);
    return world_pos.xyz;
}

/// Convert a clip space position to world space
fn position_clip_to_world(clip_pos: vec4<f32>, world_from_clip: mat4x4<f32>) -> vec3<f32> {
    let world_pos = world_from_clip * clip_pos;
    return world_pos.xyz;
}

/// Convert a ndc space position to world space
fn position_ndc_to_world(ndc_pos: vec3<f32>, world_from_clip: mat4x4<f32>) -> vec3<f32> {
    let world_pos = world_from_clip * vec4(ndc_pos, 1.0);
    return world_pos.xyz / world_pos.w;
}

/// Convert a view space direction to world space
fn direction_view_to_world(view_dir: vec3<f32>, world_from_view: mat4x4<f32>) -> vec3<f32> {
    let world_dir = world_from_view * vec4(view_dir, 0.0);
    return world_dir.xyz;
}

/// Convert a clip space direction to world space
fn direction_clip_to_world(clip_dir: vec4<f32>, world_from_clip: mat4x4<f32>) -> vec3<f32> {
    let world_dir = world_from_clip * clip_dir;
    return world_dir.xyz;
}

// -----------------
// TO VIEW ---------
// -----------------

/// Convert a world space position to view space
fn position_world_to_view(world_pos: vec3<f32>, view_from_world: mat4x4<f32>) -> vec3<f32> {
    let view_pos = view_from_world * vec4(world_pos, 1.0);
    return view_pos.xyz;
}

/// Convert a clip space position to view space
fn position_clip_to_view(clip_pos: vec4<f32>, view_from_clip: mat4x4<f32>) -> vec3<f32> {
    let view_pos = view_from_clip * clip_pos;
    return view_pos.xyz;
}

/// Convert a ndc space position to view space
fn position_ndc_to_view(ndc_pos: vec3<f32>, view_from_clip: mat4x4<f32>) -> vec3<f32> {
    let view_pos = view_from_clip * vec4(ndc_pos, 1.0);
    return view_pos.xyz / view_pos.w;
}

/// Convert a world space direction to view space
fn direction_world_to_view(world_dir: vec3<f32>, view_from_world: mat4x4<f32>) -> vec3<f32> {
    let view_dir = view_from_world * vec4(world_dir, 0.0);
    return view_dir.xyz;
}

/// Convert a clip space direction to view space
fn direction_clip_to_view(clip_dir: vec4<f32>, view_from_clip: mat4x4<f32>) -> vec3<f32> {
    let view_dir = view_from_clip * clip_dir;
    return view_dir.xyz;
}

// -----------------
// TO CLIP ---------
// -----------------

/// Convert a world space position to clip space
fn position_world_to_clip(world_pos: vec3<f32>, clip_from_world: mat4x4<f32>) -> vec4<f32> {
    let clip_pos = clip_from_world * vec4(world_pos, 1.0);
    return clip_pos;
}

/// Convert a view space position to clip space
fn position_view_to_clip(view_pos: vec3<f32>, clip_from_view: mat4x4<f32>) -> vec4<f32> {
    let clip_pos = clip_from_view * vec4(view_pos, 1.0);
    return clip_pos;
}

/// Convert a world space direction to clip space
fn direction_world_to_clip(world_dir: vec3<f32>, clip_from_world: mat4x4<f32>) -> vec4<f32> {
    let clip_dir = clip_from_world * vec4(world_dir, 0.0);
    return clip_dir;
}

/// Convert a view space direction to clip space
fn direction_view_to_clip(view_dir: vec3<f32>, clip_from_view: mat4x4<f32>) -> vec4<f32> {
    let clip_dir = clip_from_view * vec4(view_dir, 0.0);
    return clip_dir;
}

// -----------------
// TO NDC ----------
// -----------------

/// Convert a world space position to ndc space
fn position_world_to_ndc(world_pos: vec3<f32>, clip_from_world: mat4x4<f32>) -> vec3<f32> {
    let ndc_pos = clip_from_world * vec4(world_pos, 1.0);
    return ndc_pos.xyz / ndc_pos.w;
}

/// Convert a view space position to ndc space
fn position_view_to_ndc(view_pos: vec3<f32>, clip_from_view: mat4x4<f32>) -> vec3<f32> {
    let ndc_pos = clip_from_view * vec4(view_pos, 1.0);
    return ndc_pos.xyz / ndc_pos.w;
}

// -----------------
// DEPTH -----------
// -----------------

/// Retrieve the perspective camera near clipping plane
fn perspective_camera_near(clip_from_view: mat4x4<f32>) -> f32 {
    return clip_from_view[3][2];
}

/// Convert ndc depth to linear view z.
/// Note: Depth values in front of the camera will be negative as -z is forward
fn depth_ndc_to_view_z(ndc_depth: f32, clip_from_view: mat4x4<f32>, view_from_clip: mat4x4<f32>) -> f32 {
#ifdef VIEW_PROJECTION_PERSPECTIVE
    return -perspective_camera_near(clip_from_view) / ndc_depth;
#else ifdef VIEW_PROJECTION_ORTHOGRAPHIC
    return -(clip_from_view[3][2] - ndc_depth) / clip_from_view[2][2];
#else
    let view_pos = view_from_clip * vec4(0.0, 0.0, ndc_depth, 1.0);
    return view_pos.z / view_pos.w;
#endif
}

/// Convert linear view z to ndc depth.
/// Note: View z input should be negative for values in front of the camera as -z is forward
fn view_z_to_depth_ndc(view_z: f32, clip_from_view: mat4x4<f32>) -> f32 {
#ifdef VIEW_PROJECTION_PERSPECTIVE
    return -perspective_camera_near(clip_from_view) / view_z;
#else ifdef VIEW_PROJECTION_ORTHOGRAPHIC
    return clip_from_view[3][2] + view_z * clip_from_view[2][2];
#else
    let ndc_pos = clip_from_view * vec4(0.0, 0.0, view_z, 1.0);
    return ndc_pos.z / ndc_pos.w;
#endif
}

// -----------------
// UV --------------
// -----------------

/// Convert ndc space xy coordinate [-1.0 .. 1.0] to uv [0.0 .. 1.0]
fn ndc_to_uv(ndc: vec2<f32>) -> vec2<f32> {
    return ndc * vec2(0.5, -0.5) + vec2(0.5);
}

/// Convert uv [0.0 .. 1.0] coordinate to ndc space xy [-1.0 .. 1.0]
fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
}

/// returns the (0.0, 0.0) .. (1.0, 1.0) position within the viewport for the current render target
/// [0 .. render target viewport size] eg. [(0.0, 0.0) .. (1280.0, 720.0)] to [(0.0, 0.0) .. (1.0, 1.0)]
fn frag_coord_to_uv(frag_coord: vec2<f32>, viewport: vec4<f32>) -> vec2<f32> {
    return (frag_coord - viewport.xy) / viewport.zw;
}

/// Convert frag coord to ndc
fn frag_coord_to_ndc(frag_coord: vec4<f32>, viewport: vec4<f32>) -> vec3<f32> {
    return vec3(uv_to_ndc(frag_coord_to_uv(frag_coord.xy, viewport)), frag_coord.z);
}

/// Convert ndc space xy coordinate [-1.0 .. 1.0] to [0 .. render target
/// viewport size]
fn ndc_to_frag_coord(ndc: vec2<f32>, viewport: vec4<f32>) -> vec2<f32> {
    return ndc_to_uv(ndc) * viewport.zw;
}
