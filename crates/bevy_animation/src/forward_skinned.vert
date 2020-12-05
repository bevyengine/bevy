#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;
layout(location = 3) in vec4 Vertex_Weight;
layout(location = 4) in uvec4 Vertex_Join;

layout(location = 0) out vec3 v_Position;
layout(location = 1) out vec3 v_Normal;
layout(location = 2) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};

layout(set = 2, binding = 1) buffer SkinInstance_joint_matrices {
    mat4[] Joints;
};

void main() {
    mat4 Mat = Vertex_Weight.x * Joints[Vertex_Join.x]
        + Vertex_Weight.y * Joints[Vertex_Join.y]
        + Vertex_Weight.z * Joints[Vertex_Join.z]
        + Vertex_Weight.w * Joints[Vertex_Join.w];

    Mat = Model * Mat;

    v_Normal = (Mat * vec4(Vertex_Normal, 1.0)).xyz;
    v_Normal = mat3(Mat) * Vertex_Normal;
    v_Position = (Mat * vec4(Vertex_Position, 1.0)).xyz;
    v_Uv = Vertex_Uv;
    gl_Position = ViewProj * vec4(v_Position, 1.0);
}
