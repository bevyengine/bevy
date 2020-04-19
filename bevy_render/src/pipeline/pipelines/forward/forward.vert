#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

# ifdef INSTANCING
layout(location = 3) in vec4 I_Object_Model_0;
layout(location = 4) in vec4 I_Object_Model_1;
layout(location = 5) in vec4 I_Object_Model_2;
layout(location = 6) in vec4 I_Object_Model_3;
# endif

layout(location = 0) out vec3 v_Position;
layout(location = 1) out vec3 v_Normal;
layout(location = 2) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

# ifndef INSTANCING
layout(set = 1, binding = 0) uniform Object {
    mat4 Model;
};
# endif

void main() {
# ifdef INSTANCING
    mat4 Model = mat4(
        I_Object_Model_0,
        I_Object_Model_1,
        I_Object_Model_2,
        I_Object_Model_3
    );
# endif

    v_Normal = (Model * vec4(Vertex_Normal, 1.0)).xyz;
    v_Position = (Model * vec4(Vertex_Position, 1.0)).xyz;
    v_Uv = Vertex_Uv;
    gl_Position = ViewProj * vec4(v_Position, 1.0);
}
