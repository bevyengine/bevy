#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout(location = 0) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 clip_from_world;
    mat4 unjittered_clip_from_world;
    mat4 world_from_clip;
    mat4 world_from_view;
    mat4 view_from_world;
    mat4 clip_from_view;
    mat4 view_from_clip;
    vec3 world_position; // Camera's world position
    float exposure;
    vec4 viewport; // viewport(x_origin, y_origin, width, height)
    vec4 frustum[6];
    // See full definition in: crates/bevy_render/src/view/view.wgsl
} camera_view;

struct Mesh {
    mat3x4 Model;
    mat4 InverseTransposeModel;
    uint flags;
};

#ifdef PER_OBJECT_BUFFER_BATCH_SIZE
layout(set = 1, binding = 0) uniform Mesh Meshes[#{PER_OBJECT_BUFFER_BATCH_SIZE}];
#else
layout(set = 1, binding = 0) readonly buffer _Meshes {
    Mesh Meshes[];
};
#endif // PER_OBJECT_BUFFER_BATCH_SIZE

mat4 affine_to_square(mat3x4 affine) {
    return transpose(mat4(
        affine[0],
        affine[1],
        affine[2],
        vec4(0.0, 0.0, 0.0, 1.0)
    ));
}

void main() {
    v_Uv = Vertex_Uv;
    gl_Position = camera_view.clip_from_world
        * affine_to_square(Meshes[gl_InstanceIndex].Model)
        * vec4(Vertex_Position, 1.0);
}
