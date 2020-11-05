LAYOUT(location = 0) in vec2 v_Uv;
LAYOUT(location = 1) in vec4 v_Color;

LAYOUT(location = 0) out vec4 o_Target;

UNIFORM_TEXTURE(set = 1, binding = 2, TextureAtlas_texture)
UNIFORM_SAMPLER(set = 1, binding = 3, TextureAtlas_texture_sampler)

void main() {
    o_Target = encodeColor(
        v_Color * texture(
            sampler2D(TextureAtlas_texture, TextureAtlas_texture_sampler),
            v_Uv
        )
    );
}
