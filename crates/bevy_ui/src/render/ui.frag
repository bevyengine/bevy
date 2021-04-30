#version 450

layout(location = 0) in vec2 v_Uv;
layout(location = 1) in vec4 v_Bounds;
layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 0) uniform ColorMaterial_color {
    vec4 Color;
};

# ifdef COLORMATERIAL_TEXTURE
layout(set = 2, binding = 1) uniform texture2D ColorMaterial_texture;
layout(set = 2, binding = 2) uniform sampler ColorMaterial_texture_sampler;
# endif

void main() {
    if(gl_FragCoord.x <= v_Bounds.x || gl_FragCoord.x >= v_Bounds.z ||
        gl_FragCoord.y <= v_Bounds.y || gl_FragCoord.y >= v_Bounds.w)
    {
        discard;
    }
vec4 color = Color;
# ifdef COLORMATERIAL_TEXTURE
    color *= texture(
        sampler2D(ColorMaterial_texture, ColorMaterial_texture_sampler),
        v_Uv);
# endif
    o_Target = color;
}
