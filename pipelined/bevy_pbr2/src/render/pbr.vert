#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout(location = 0) out vec4 v_WorldPosition;
layout(location = 1) out vec3 v_WorldNormal;
layout(location = 2) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform View {
    mat4 ViewProj;
    vec3 ViewWorldPosition;
};

layout(set = 1, binding = 0) uniform MeshTransform {
    mat4 Model;
};

void main() {
    v_Uv = Vertex_Uv;
    vec4 world_position = Model * vec4(Vertex_Position, 1.0);
    v_WorldPosition = world_position;
    // FIXME: The inverse transpose of the model matrix should be used to correctly handle scaling
    // of normals
    v_WorldNormal = mat3(Model) * Vertex_Normal;
    gl_Position = ViewProj * world_position;
}
