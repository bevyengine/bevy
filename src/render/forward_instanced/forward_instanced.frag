#version 450

const int MAX_LIGHTS = 10;

layout(location = 0) in vec3 v_Normal;
layout(location = 1) in vec4 v_Position;
layout(location = 2) in vec4 v_Color;

layout(location = 0) out vec4 o_Target;

struct Light {
    mat4 proj;
    vec4 pos;
    vec4 color;
};

layout(set = 0, binding = 0) uniform Globals {
    mat4 u_ViewProj;
    uvec4 u_NumLights;
};
layout(set = 0, binding = 1) uniform Lights {
    Light u_Lights[MAX_LIGHTS];
};

void main() {
    vec3 normal = normalize(v_Normal);
    vec3 ambient = vec3(0.05, 0.05, 0.05);
    // accumulate color
    vec3 color = ambient;
    for (int i=0; i<int(u_NumLights.x) && i<MAX_LIGHTS; ++i) {
        Light light = u_Lights[i];
        // compute Lambertian diffuse term
        vec3 light_dir = normalize(light.pos.xyz - v_Position.xyz);
        float diffuse = max(0.0, dot(normal, light_dir));
        // add light contribution
        color += diffuse * light.color.xyz;
    }
    // multiply the light by material color
    o_Target = vec4(color, 1.0) * v_Color;
}
