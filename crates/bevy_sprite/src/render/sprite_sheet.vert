#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout(location = 0) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform Camera2d {
    mat4 ViewProj;
};

// TODO: merge dimensions into "sprites" buffer when that is supported in the Uniforms derive abstraction
layout(set = 1, binding = 0) uniform TextureAtlas_dimensions {
    vec2 Dimensions;
};

struct Rect {
    vec2 begin;
    vec2 end;
};

layout(set = 1, binding = 1) buffer TextureAtlas_textures {
    Rect[] Textures;
};


layout(set = 2, binding = 0) uniform TextureAtlasSprite {
    vec3 TextureAtlasSprite_position;
    float TextureAtlasSprite_scale;
    uint TextureAtlasSprite_index;
};

void main() {
    Rect sprite_rect = Textures[TextureAtlasSprite_index];
    vec2 sprite_dimensions = sprite_rect.end - sprite_rect.begin;
    vec3 vertex_position = vec3(Vertex_Position.xy * sprite_dimensions * TextureAtlasSprite_scale, 0.0) + TextureAtlasSprite_position;
    vec2 uvs[4] = vec2[](
        vec2(sprite_rect.begin.x, sprite_rect.end.y),
        sprite_rect.begin,
        vec2(sprite_rect.end.x, sprite_rect.begin.y), 
        sprite_rect.end
    );
    v_Uv = uvs[gl_VertexIndex] / Dimensions;
    gl_Position = ViewProj * vec4(vertex_position, 1.0);
}