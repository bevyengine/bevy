#version 450

layout(location = 0) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 1, binding = 1) uniform Node_size {
    vec2 NodeSize;
};

layout(set = 2, binding = 0) uniform ColorMaterial_color {
    vec4 Color;
};
layout(set = 2, binding = 1) uniform ColorMaterial_border {
    float border_radius;
};

# ifdef COLORMATERIAL_TEXTURE 
layout(set = 2, binding = 2) uniform texture2D ColorMaterial_texture;
layout(set = 2, binding = 3) uniform sampler ColorMaterial_texture_sampler;
# endif

// Calculate the distance from the fragment to the border of the rounded rectangle,
// return negative value when the fragment is inside the rounded rectangle.
float distance_round_border(vec2 point, vec2 size, float round) {
    vec2 dr = abs(point) - (size - round);
    float d = length(max(dr, vec2(0.0))) - round;
    vec2 t = min(dr, vec2(0.0));
    float d_extra = max(t.x, t.y);

    return d + d_extra;
}

void main() {
    vec4 color = Color;
# ifdef COLORMATERIAL_TEXTURE
    color *= texture(
        sampler2D(ColorMaterial_texture, ColorMaterial_texture_sampler),
        v_Uv);
# endif

    if (border_radius > 0.0) {
        float d = distance_round_border((v_Uv - vec2(0.5)) * NodeSize, NodeSize * 0.5, border_radius);
        float softness = 0.33;
        color.a *= 1.0 - smoothstep(-softness, softness, d);
    }

    o_Target = color;
}
