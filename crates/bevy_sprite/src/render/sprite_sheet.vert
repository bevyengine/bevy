#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout(location = 0) out vec2 v_Uv;
layout(location = 1) out vec4 v_Color;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

// TODO: merge dimensions into "sprites" buffer when that is supported in the Uniforms derive abstraction
layout(set = 1, binding = 0) uniform TextureAtlas_size {
    vec2 AtlasSize;
};

struct Rect {
    vec2 begin;
    vec2 end;
};

layout(set = 1, binding = 1) buffer TextureAtlas_textures {
    Rect[] Textures;
};


layout(set = 2, binding = 0) uniform Transform {
    mat4 SpriteTransform;
};

layout(set = 2, binding = 1) uniform TextureAtlasSprite {
    vec4 TextureAtlasSprite_color;
    uint TextureAtlasSprite_index;
};

void main() {
    Rect sprite_rect = Textures[TextureAtlasSprite_index];
    vec2 sprite_dimensions = sprite_rect.end - sprite_rect.begin;
    vec3 vertex_position = vec3(Vertex_Position.xy * sprite_dimensions, 0.0);
    vec2 atlas_positions[4] = vec2[](
        vec2(sprite_rect.begin.x, sprite_rect.end.y),
        sprite_rect.begin,
        vec2(sprite_rect.end.x, sprite_rect.begin.y), 
        sprite_rect.end
    );
    v_Uv = (atlas_positions[gl_VertexIndex] + vec2(0.05, 0.05)) / AtlasSize;
    v_Color = TextureAtlasSprite_color;
    gl_Position = ViewProj * SpriteTransform * vec4(ceil(vertex_position), 1.0);
}