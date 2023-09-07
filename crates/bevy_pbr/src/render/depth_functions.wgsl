#define_import_path bevy_pbr::depth_functions

// NDC depth using our projections is 1 at the near plane and 0 at the far plane.
// If UV is 0,0 top left and 1,1 bottom right, then say xy = uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0) I think.
// and then you make p_ndc = vec4(xy, depth_ndc, 1.0); p_view_homogeneous = view.inverse_projection * p_ndc; p_view = p_view_homogeneous.xyz / p_view_homogeneous.w; p_world = view.view * vec4(p_view, 1.0);

fn depth_to_world_position(uv: vec2<f32>, depth: f32, inverse_projection: mat4x4<f32>, view: mat4x4<f32>) -> vec3<f32>{
    let clip_xy = uv_to_clip(uv);
    let p_ndc = vec4(clip_xy, depth, 1.0);
    let p_view_homogeneous = inverse_projection * p_ndc;
    let p_view  = p_view_homogeneous.xyz / p_view_homogeneous.w;
    let p_world = view * vec4(p_view, 1.0);
    return p_world.xyz;
}

fn depth_to_world_position_two(uv: vec2<f32>, depth: f32, inverse_projection: mat4x4<f32>, view_world_pos: vec3<f32>) -> vec3<f32>{
    let view_pos = depth_to_view_space_position(uv, depth, inverse_projection);
    let world_pos = view_pos - view_world_pos;
    return world_pos;
}

fn depth_to_view_space_position(uv: vec2<f32>, depth: f32, inverse_projection: mat4x4<f32>) -> vec3<f32> {
    let clip_xy = uv_to_clip(uv);
    let t = inverse_projection * vec4(clip_xy, depth, 1.0);
    let view_xyz = t.xyz / t.w;
    return view_xyz;
}

fn uv_to_clip(uv: vec2<f32>) -> vec2<f32>{
    return uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
}