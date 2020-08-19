#version 450

const int MAX_LIGHTS = 10;

struct Light {
    mat4 proj;
    vec3 pos;
    float attenuation;
    vec4 color;
};

layout(location = 0) in vec3 v_Position;
layout(location = 1) in vec3 v_Normal;
layout(location = 2) in vec2 v_Uv;
layout(location = 3) in vec3 w_Position;


layout(location = 0) out vec4 o_Target;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
    vec4 CameraPos;
};

layout(set = 1, binding = 0) uniform Lights {
    uvec4 NumLights;
    Light SceneLights[MAX_LIGHTS];
};

layout(set = 3, binding = 0) uniform StandardMaterial_albedo {
    vec4 Albedo;
};

layout(set = 3, binding = 3) uniform StandardMaterial_pbr {
    vec2 pbr;
};

# ifdef STANDARDMATERIAL_ALBEDO_TEXTURE
layout(set = 3, binding = 1) uniform texture2D StandardMaterial_albedo_texture;
layout(set = 3, binding = 2) uniform sampler StandardMaterial_albedo_texture_sampler;
# endif

# ifdef STANDARDMATERIAL_SHADED

#define saturate(x)        clamp(x, 0.0, 1.0)
const float PI = 3.141592653589793;

float pow5(float x) {
    float x2 = x * x;
    return x2 * x2 * x;
}

float getSquareFalloffAttenuation(float distanceSquare, float falloff) {
    float factor = distanceSquare * falloff;
    float smoothFactor = saturate(1.0 - factor * factor);
    return smoothFactor * smoothFactor;
}

float getDistanceAttenuation(const highp vec3 posToLight, float falloff) {
    float distanceSquare = dot(posToLight, posToLight);
    float attenuation = getSquareFalloffAttenuation(distanceSquare, falloff);
    return attenuation * 1.0 / max(distanceSquare, 1e-4);
}

float D_GGX(float roughness, float NoH, const vec3 h) {
    float oneMinusNoHSquared = 1.0 - NoH * NoH;
    float a = NoH * roughness;
    float k = roughness / (oneMinusNoHSquared + a * a);
    float d = k * k * (1.0 / PI);
    return d;
}

float V_SmithGGXCorrelated(float roughness, float NoV, float NoL) {
    float a2 = roughness * roughness;
    float lambdaV = NoL * sqrt((NoV - a2 * NoV) * NoV + a2);
    float lambdaL = NoV * sqrt((NoL - a2 * NoL) * NoL + a2);
    float v = 0.5 / (lambdaV + lambdaL);
    return v;
}

vec3 F_Schlick(const vec3 f0, float f90, float VoH) {
    return f0 + (f90 - f0) * pow5(1.0 - VoH);
}

float F_Schlick(float f0, float f90, float VoH) {
    return f0 + (f90 - f0) * pow5(1.0 - VoH);
}

vec3 fresnel(vec3 f0, float LoH) {
    float f90 = saturate(dot(f0, vec3(50.0 * 0.33)));
    return F_Schlick(f0, f90, LoH);
}

vec3 isotropicLobe(vec3 f0, float roughness, const vec3 h, float NoV, float NoL, float NoH, float LoH) {

    float D = D_GGX(roughness, NoH, h);
    float V = V_SmithGGXCorrelated(roughness, NoV, NoL);
    vec3  F = fresnel(f0, LoH);

    return (D * V) * F;
}

float Fd_Burley(float roughness, float NoV, float NoL, float LoH) {
    float f90 = 0.5 + 2.0 * roughness * LoH * LoH;
    float lightScatter = F_Schlick(1.0, f90, NoL);
    float viewScatter  = F_Schlick(1.0, f90, NoV);
    return lightScatter * viewScatter * (1.0 / PI);
}

vec3 computeDiffuseColor(const vec3 baseColor, float metallic) {
    return baseColor * (1.0 - metallic);
}

#define MIN_N_DOT_V 1e-4

float clampNoV(float NoV) {
    // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
    return max(NoV, MIN_N_DOT_V);
}

# endif

void main() {
    vec4 output_color = vec4(1.0, 0.0, 0.0, 1.0); //Albedo;
# ifdef STANDARDMATERIAL_ALBEDO_TEXTURE
    output_color *= texture(
        sampler2D(StandardMaterial_albedo_texture, StandardMaterial_albedo_texture_sampler),
        v_Uv);
# endif

# ifdef STANDARDMATERIAL_SHADED
    float roughness = pbr.x;
    float metallic = pbr.y;
    vec3 N = normalize(v_Normal);
    vec3 V = normalize(CameraPos.xyz - v_Position.xyz);

    vec3 F0 = vec3(0.04); 
    F0 = mix(F0, output_color.rgb, metallic);

    vec3 diffuseColor = computeDiffuseColor(output_color.rgb, metallic);

    // accumulate color
    vec3 light_accum = vec3(0.0);
    for (int i=0; i<int(NumLights.x) && i<MAX_LIGHTS; ++i) {
        Light light = SceneLights[i];

        vec3 lightDir = light.pos.xyz - v_Position.xyz;
        vec3 L = normalize(lightDir);

        float rangeAttenuation = getDistanceAttenuation(lightDir, light.attenuation);
        
        vec3 H = normalize(L + V);

        float NdotL = clampNoV(dot(N, L));
        float NdotV = clampNoV(dot(N, V));
        float NoL = saturate(NdotL);
        float NoH = saturate(dot(N, H));
        float LoH = saturate(dot(L, H));

        vec3 specular = isotropicLobe(F0, roughness, H, NdotV, NoL, NoH, LoH);
        vec3 diffuse = diffuseColor * Fd_Burley(roughness, NdotV, NoL, LoH);

        light_accum += ((diffuse + specular) * light.color.xyz) * (light.color.w * NdotL);
    }

    output_color.xyz = light_accum;

    // Gamma correction.
    output_color.xyz = output_color.xyz / (output_color.xyz + vec3(1.0));
    output_color.xyz = pow(output_color.xyz, vec3(1.0/2.2)); 
# endif

    // multiply the light by material color
    o_Target = output_color;
}
