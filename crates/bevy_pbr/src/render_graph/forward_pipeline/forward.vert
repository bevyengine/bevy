LAYOUT(location = 0) in vec3 Vertex_Position;
LAYOUT(location = 1) in vec3 Vertex_Normal;
LAYOUT(location = 2) in vec2 Vertex_Uv;

LAYOUT(location = 0) out vec3 v_Position;
LAYOUT(location = 1) out vec3 v_Normal;
LAYOUT(location = 2) out vec2 v_Uv;

BLOCK_LAYOUT(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

BLOCK_LAYOUT(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};

void main() {
    v_Normal = (Model * vec4(Vertex_Normal, 1.0)).xyz;
    v_Normal = mat3(Model) * Vertex_Normal;
    v_Position = (Model * vec4(Vertex_Position, 1.0)).xyz;
    v_Uv = Vertex_Uv;
    gl_Position = ViewProj * vec4(v_Position, 1.0);
}
