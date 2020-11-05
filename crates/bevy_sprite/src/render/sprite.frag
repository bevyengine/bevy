LAYOUT(location=0) in vec2 v_Uv;
LAYOUT(location=0) out vec4 o_Target;

BLOCK_LAYOUT(set = 1, binding = 0) uniform ColorMaterial_color {
    vec4 Color;
};

# ifdef COLORMATERIAL_TEXTURE
UNIFORM_TEXTURE(set=1, binding=1, ColorMaterial_texture)
UNIFORM_SAMPLER(set=1, binding=2, ColorMaterial_texture_sampler)
# endif

void main() {
    vec4 color = Color;
# ifdef COLORMATERIAL_TEXTURE
    color *= texture(
        sampler2D(ColorMaterial_texture, ColorMaterial_texture_sampler),
        v_Uv);
# endif
    o_Target = encodeColor(color);
}
