#version 450

layout(location = 0) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 0) uniform ColorMaterial_color {
    vec4 Color;
};

layout(set = 1, binding = 1) uniform Node_size {
    vec2 NodeSize;
};

struct Border {
    vec4 color;
    float radius;
    float thickness;
};

layout(set = 1, binding = 2) uniform Style_border {
    Border _border;
};

# ifdef COLORMATERIAL_TEXTURE 
layout(set = 2, binding = 1) uniform texture2D ColorMaterial_texture;
layout(set = 2, binding = 2) uniform sampler ColorMaterial_texture_sampler;
# endif

void main() {
    vec4 color = Color;
# ifdef COLORMATERIAL_TEXTURE
    color *= texture(
        sampler2D(ColorMaterial_texture, ColorMaterial_texture_sampler),
        v_Uv);
# endif
    float t = _border.thickness;
    float r = _border.radius;

    float aa = clamp(1.0, 20.0, 4 * t);

    vec2 pos = abs(v_Uv - 0.5) * NodeSize;

    //https://www.desmos.com/calculator/1ttzhiy51j

    vec2 half_size = NodeSize / 2;
    vec2 calc = pos + r - half_size;
    float c_dist_sq = dot(calc, calc);

    float O = smoothstep(-aa, aa, c_dist_sq - r*r);
    float O2 = 1.0 - O;

    float m = max(r-t, 0);
    float R = smoothstep(-aa, aa, c_dist_sq - m*m);

    vec2 B = step(0.0, pos + r - half_size);
    vec2 B2 = 1.0.xx - B;

    float outside = B.x * B.y * O;
    
    float border_corners = B.x * B.y * R * O2;
    float border_sides = dot(step(0.0, pos + t - half_size), B2.yx);
    float border = min(1.0, border_corners + border_sides);

    float inner = 1 - clamp(0.0, 1.0, border + outside);

    o_Target = inner * color + border * _border.color;
}
