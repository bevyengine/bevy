#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
# ifdef STANDARDMATERIAL_ALBEDO_TEXTURE
layout(location = 2) in vec2 Vertex_Uv;
# endif

layout(location = 0) out vec3 v_Position;
layout(location = 1) out vec3 v_Normal;
# ifdef STANDARDMATERIAL_ALBEDO_TEXTURE
layout(location = 2) out vec2 v_Uv;
# endif

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};

void main() {
    v_Normal = mat3(Model) * Vertex_Normal;
    v_Position = (Model * vec4(Vertex_Position, 1.0)).xyz;
# ifdef STANDARDMATERIAL_ALBEDO_TEXTURE
    v_Uv = Vertex_Uv;
# endif

    gl_Position = ViewProj * vec4(v_Position, 1.0);
}
