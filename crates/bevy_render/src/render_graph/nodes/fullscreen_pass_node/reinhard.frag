#version 450

layout(location=0) in vec2 v_Uv;

layout(set = 0, binding = 0) uniform texture2D color_texture;
layout(set = 0, binding = 1) uniform sampler color_texture_sampler;

layout(location=0) out vec4 o_Target;

// from https://64.github.io/tonemapping/
// reinhard on RGB oversaturates colors
vec3 reinhard(vec3 color) {
    return color / (1.0 + color);
}

vec3 reinhard_extended(vec3 color, float max_white) {
    vec3 numerator = color * (1.0f + (color / vec3(max_white * max_white)));
    return numerator / (1.0 + color);
}

// luminance coefficients from Rec. 709.
// https://en.wikipedia.org/wiki/Rec._709
float luminance(vec3 v) {
    return dot(v, vec3(0.2126, 0.7152, 0.0722));
}

vec3 change_luminance(vec3 c_in, float l_out) {
    float l_in = luminance(c_in);
    return c_in * (l_out / l_in);
}

vec3 reinhard_luminance(vec3 color) {
    float l_old = luminance(color);
    float l_new = l_old / (1.0f + l_old);
    return change_luminance(color, l_new);
}

vec3 reinhard_extended_luminance(vec3 color, float max_white_l) {
    float l_old = luminance(color);
    float numerator = l_old * (1.0f + (l_old / (max_white_l * max_white_l)));
    float l_new = numerator / (1.0f + l_old);
    return change_luminance(color, l_new);
}

void main() {
    vec4 output_color = texture(sampler2D(color_texture, color_texture_sampler), v_Uv);
    output_color.rgb = reinhard_luminance(output_color.rgb);

    o_Target = output_color;
}
