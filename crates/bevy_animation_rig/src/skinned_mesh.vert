#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;
layout(location = 3) in vec4 Vertex_JointWeight;
layout(location = 4) in uvec4 Vertex_JointIndex;

#ifdef STANDARDMATERIAL_NORMAL_MAP
layout(location = 5) in vec4 Vertex_Tangent;
#endif

layout(location = 0) out vec3 v_WorldPosition;
layout(location = 1) out vec3 v_WorldNormal;
layout(location = 2) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

#ifdef STANDARDMATERIAL_NORMAL_MAP
layout(location = 3) out vec4 v_WorldTangent;
#endif

layout(set = 2, binding = 0) buffer JointTransforms {
    mat4[] Joints;
};

void main() {
    mat4 Model =
        Vertex_JointWeight.x * Joints[Vertex_JointIndex.x] +
        Vertex_JointWeight.y * Joints[Vertex_JointIndex.y] +
        Vertex_JointWeight.z * Joints[Vertex_JointIndex.z] +
        Vertex_JointWeight.w * Joints[Vertex_JointIndex.w];

    vec4 world_position = Model * vec4(Vertex_Position, 1.0);
    v_WorldPosition = world_position.xyz;
    v_WorldNormal = mat3(Model) * Vertex_Normal;
    v_Uv = Vertex_Uv;
#ifdef STANDARDMATERIAL_NORMAL_MAP
    v_WorldTangent = vec4(mat3(Model) * Vertex_Tangent.xyz, Vertex_Tangent.w);
#endif
    gl_Position = ViewProj * world_position;
}
