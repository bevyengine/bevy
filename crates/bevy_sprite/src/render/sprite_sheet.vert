#version 450

// sprite
layout(location = 0) in vec3 Sprite_Position;
// this is a vec2 instead of an int due to WebGPU limitations
layout(location = 1) in ivec2 Sprite_Index; 

layout(location = 0) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform Camera2d {
    mat4 ViewProj;
};

struct Rect {
    vec2 begin;
    vec2 end;
};

layout(set = 1, binding = 0) buffer SpriteSheet {
    Rect[] SpriteSheet_sprites;
};

const vec2 positions[4] = vec2[](
    vec2(0.5, -0.5),
    vec2(-0.5, -0.5),
    vec2(0.5, 0.5),
    vec2(-0.5, 0.5)
);

void main() {
    Rect sprite_rect = SpriteSheet_sprites[Sprite_Index.x]; 
    vec2 dimensions = sprite_rect.end - sprite_rect.begin;
    vec2 vertex_position = positions[gl_VertexIndex] * dimensions;
    vec2 uvs[4] = vec2[](
        vec2(sprite_rect.end.x, sprite_rect.begin.y), 
        sprite_rect.begin,
        sprite_rect.end,
        vec2(sprite_rect.begin.x, sprite_rect.end.y)
    );
    v_Uv = uvs[gl_VertexIndex];
    gl_Position = ViewProj * vec4(vec3(vertex_position, 0.0) + Sprite_Position, 1.0);
}