# ifdef WGPU

# define encodeColor
# define UNIFORM_TEXTURE(set_value, binding_value, name) layout(set_value, binding_value) uniform texture2D name;
# define UNIFORM_SAMPLER(set_value, binding_value, name) layout(set_value, binding_value) uniform sampler name;
# define TEXTURE_2D texture2D
# define LAYOUT(location) layout(location)
# define BLOCK_LAYOUT(set_value, binding_value) layout(set_value, binding_value)

# endif

# ifdef WEBGL2
precision highp float;
vec4 encodeColor(vec4 linearRGB_in)
{
    vec3 linearRGB = linearRGB_in.rgb;
    vec3 a = 12.92 * linearRGB;
    vec3 b = 1.055 * pow(linearRGB, vec3(1.0 / 2.4)) - 0.055;
    vec3 c = step(vec3(0.0031308), linearRGB);
    return vec4(mix(a, b, c), linearRGB_in.a);
}

# define UNIFORM_TEXTURE(set_value, binding_value, name) uniform sampler2D name;
# define UNIFORM_SAMPLER(set_value, binding_value, name)
# define LAYOUT(location)
# define BLOCK_LAYOUT(set_value, binding_value) layout(std140)
# define TEXTURE_2D sampler2D
# define sampler2D(a, b) (a)
# define gl_VertexIndex gl_VertexID
# endif
