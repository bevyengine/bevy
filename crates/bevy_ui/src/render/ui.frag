#version 450

struct Border {
    vec4 color;
    float radius;
    float thickness;
};

struct Bounds {
    vec2 offset;
    vec2 size;
    float radius;
    float thickness;
};


layout(location = 0) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 0) uniform ColorMaterial_color {
    vec4 Color;
};

layout(set = 1, binding = 1) uniform Node_size {
    vec2 NodeSize;
};

layout(set = 1, binding = 3) buffer Node_bounds {
    Bounds[] bounds;
};

layout(set = 1, binding = 2) uniform Style_border {
    Border _border;
};

# ifdef COLORMATERIAL_TEXTURE 
layout(set = 2, binding = 1) uniform texture2D ColorMaterial_texture;
layout(set = 2, binding = 2) uniform sampler ColorMaterial_texture_sampler;
# endif
#define saturate(x)        clamp(x, 0.0, 1.0)

float aastep(float threshold, float value, float factor) {
    float afwidth = length(vec2(dFdx(value), dFdy(value))) * 0.70710678118654757 * factor;
    return smoothstep(threshold-afwidth, threshold+afwidth, value);
}
vec2 aastep(vec2 threshold, vec2 value, float factor) {
    vec2 dfx = dFdx(value);
    vec2 dfy = dFdy(value);
    vec2 afwidth = vec2(length(vec2(dfx.x, dfy.x)), length(vec2(dfx.y, dfy.y))) * 0.70710678118654757 * factor;
    return smoothstep(threshold-afwidth, threshold+afwidth, value);
}
float aastep(float threshold, float value) {
    return aastep(threshold, value, 1.0);
}
vec2 aastep(vec2 threshold, vec2 value) {
    return aastep(threshold, value, 1.0);
}

void calcinner(in Bounds bounds, inout float overflow_mask) {
    float t = bounds.thickness;
    float r = bounds.radius;
    
    vec2 pos = abs((v_Uv - 0.5) * NodeSize + bounds.offset);
    vec2 half_size = bounds.size / 2.0;
    vec2 calc = pos + r - half_size;

    float m = max(r-t, 0.0);
    float R2 = 1.0 - aastep(m*m, dot(calc, calc));

    vec2 T = min(1.0.xx, pos + t - half_size);

    vec2 B2 = 1.0 - aastep(half_size, pos + r);

    overflow_mask *= T.x *  T.y * saturate(B2.x + B2.y + R2);
}

void main() {
    vec4 color = Color;
    # ifdef COLORMATERIAL_TEXTURE
        color *= texture(
            sampler2D(ColorMaterial_texture, ColorMaterial_texture_sampler),
            v_Uv);
    # endif
    float t = _border.thickness;
    float r = _border.radius;

    vec2 pos = abs(v_Uv - 0.5) * NodeSize;

    //https://www.desmos.com/calculator/1ttzhiy51j

    vec2 half_size = NodeSize / 2;
    vec2 calc = pos + r - half_size;
    float c_dist_sq = dot(calc, calc);

    float O = aastep(r*r, c_dist_sq * 0.995);
    float O2 = 1.0 - O;

    float m = max(r-t, 0.0);
    float R = aastep(m*m, c_dist_sq * 1.005);

    vec2 B = aastep(half_size, pos + r);
    vec2 B2 = 1.0.xx - B;

    float outside = B.x * B.y * O;
    
    float border_corners = B.x * B.y * R * O2;
    float border_sides = dot(step(half_size, pos + t), B2.yx);
    float border = min(t, min(1.0, border_corners + border_sides));

    float inner = 1.0 - saturate(border + outside);

    o_Target = inner * color + border * _border.color;

    float overflow_mask = 1.0;
    for (int i = 0; i < bounds.length(); i++) {
        calcinner(bounds[i], overflow_mask);
    }

    o_Target.a *= overflow_mask;
}