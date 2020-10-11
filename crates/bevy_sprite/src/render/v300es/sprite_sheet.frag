#version 300 es
// fragment shader
precision highp float;

in vec2 v_Uv;
in vec4 v_Color;

out vec4 o_Target;

uniform sampler2D TextureAtlas_texture;
// uniform sampler TextureAtlas_texture_sampler;

vec4 encodeSRGB(vec4 linearRGB_in)
{
    vec3 linearRGB = linearRGB_in.rgb;
    vec3 a = 12.92 * linearRGB;
    vec3 b = 1.055 * pow(linearRGB, vec3(1.0 / 2.4)) - 0.055;
    vec3 c = step(vec3(0.0031308), linearRGB);
    return vec4(mix(a, b, c), linearRGB_in.a);
}

vec4 decodeSRGB(vec4 screenRGB_in)
{
    vec3 screenRGB = screenRGB_in.rgb;
    vec3 a = screenRGB / 12.92;
    vec3 b = pow((screenRGB.rgb + 0.055) / 1.055, vec3(2.4));
    vec3 c = step(vec3(0.04045), screenRGB.rgb);
    return vec4(mix(a, b, c), screenRGB_in.a);
}

void main() {
    vec4 color = texture(
        TextureAtlas_texture,
        v_Uv
    );
    o_Target = encodeSRGB(v_Color * color);
}
