#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout(location = 0) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};
layout(set = 2, binding = 1) uniform Sprite {
    vec2 Sprite_size;
};

layout(set = 1, binding = 1) uniform ColorMaterial_flip_horz {
    float flip_x;
};

layout(set = 1, binding = 2) uniform ColorMaterial_flip_vert {
    float flip_y;
};

void main() {
    v_Uv = vec2(flip_x == -1 ? 1.0 - Vertex_Uv.x : Vertex_Uv.x, flip_y == -1 ? 1.0 - Vertex_Uv.y : Vertex_Uv.y);
    vec3 position = Vertex_Position * vec3(Sprite_size, 1.0);
    gl_Position = ViewProj * Model * vec4(position, 1.0);
}