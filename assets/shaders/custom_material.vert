#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout(location = 0) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 clip_from_world;
    // Other attributes exist that can be described here.
    // See full definition in: crates/bevy_render/src/view/view.wgsl
    // Attributes added here must be in the same order as they are defined
    // in view.wgsl, and they must be contiguous starting from the top to
    // ensure they have the same layout.
    //
    // Needing to maintain this mapping yourself is one of the harder parts of using
    // GLSL with Bevy. WGSL provides a much better user experience! 
} camera_view;

struct Mesh {
    mat3x4 Model;
    mat4 InverseTransposeModel;
    uint flags;
};

#ifdef PER_OBJECT_BUFFER_BATCH_SIZE
layout(set = 2, binding = 0) uniform Mesh Meshes[#{PER_OBJECT_BUFFER_BATCH_SIZE}];
#else
layout(set = 2, binding = 0) readonly buffer _Meshes {
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
