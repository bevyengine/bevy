#define_import_path bevy_sprite::mesh2d_vertex_output

@location(0) world_position: vec4<f32>,
@location(1) world_normal: vec3<f32>,
@location(2) uv: vec2<f32>,
#ifdef VERTEX_TANGENTS
@location(3) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
@location(4) color: vec4<f32>,
#endif
