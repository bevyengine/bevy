#version 450

layout(location = 0) in vec4 v_WorldPosition;
layout(location = 1) in vec3 v_WorldNormal;
layout(location = 2) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

struct PointLight {
    vec4 color;
    float range;
    float radius;
    vec3 position;
    mat4 projection;
};

// NOTE: this must be kept in sync with lights::MAX_LIGHTS
// TODO: this can be removed if we move to storage buffers for light arrays
const int MAX_POINT_LIGHTS = 10;

layout(set = 0, binding = 0) uniform View {
    mat4 ViewProj;
    vec3 ViewWorldPosition;
};
layout(std140, set = 0, binding = 1) uniform Lights {
    uint NumLights;
    PointLight PointLights[MAX_POINT_LIGHTS];
};
layout(set = 0, binding = 2) uniform texture2DArray t_Shadow;
layout(set = 0, binding = 3) uniform samplerShadow s_Shadow;

#    define saturate(x) clamp(x, 0.0, 1.0)
const float PI = 3.141592653589793;

float pow5(float x) {
    float x2 = x * x;
    return x2 * x2 * x;
}

// distanceAttenuation is simply the square falloff of light intensity
// combined with a smooth attenuation at the edge of the light radius
//
// light radius is a non-physical construct for efficiency purposes,
// because otherwise every light affects every fragment in the scene
float getDistanceAttenuation(float distanceSquare, float inverseRangeSquared) {
    float factor = distanceSquare * inverseRangeSquared;
    float smoothFactor = saturate(1.0 - factor * factor);
    float attenuation = smoothFactor * smoothFactor;
    return attenuation * 1.0 / max(distanceSquare, 1e-4);
}

// Normal distribution function (specular D)
// Based on https://google.github.io/filament/Filament.html#citation-walter07

// D_GGX(h,α) = α^2 / { π ((n⋅h)^2 (α2−1) + 1)^2 }

// Simple implementation, has precision problems when using fp16 instead of fp32
// see https://google.github.io/filament/Filament.html#listing_speculardfp16
float D_GGX(float roughness, float NoH, const vec3 h) {
    float oneMinusNoHSquared = 1.0 - NoH * NoH;
    float a = NoH * roughness;
    float k = roughness / (oneMinusNoHSquared + a * a);
    float d = k * k * (1.0 / PI);
    return d;
}

// Visibility function (Specular G)
// V(v,l,a) = G(v,l,α) / { 4 (n⋅v) (n⋅l) }
// such that f_r becomes
// f_r(v,l) = D(h,α) V(v,l,α) F(v,h,f0)
// where
// V(v,l,α) = 0.5 / { n⋅l sqrt((n⋅v)^2 (1−α2) + α2) + n⋅v sqrt((n⋅l)^2 (1−α2) + α2) }
// Note the two sqrt's, that may be slow on mobile, see https://google.github.io/filament/Filament.html#listing_approximatedspecularv
float V_SmithGGXCorrelated(float roughness, float NoV, float NoL) {
    float a2 = roughness * roughness;
    float lambdaV = NoL * sqrt((NoV - a2 * NoV) * NoV + a2);
    float lambdaL = NoV * sqrt((NoL - a2 * NoL) * NoL + a2);
    float v = 0.5 / (lambdaV + lambdaL);
    return v;
}

// Fresnel function
// see https://google.github.io/filament/Filament.html#citation-schlick94
// F_Schlick(v,h,f_0,f_90) = f_0 + (f_90 − f_0) (1 − v⋅h)^5
vec3 F_Schlick(const vec3 f0, float f90, float VoH) {
    // not using mix to keep the vec3 and float versions identical
    return f0 + (f90 - f0) * pow5(1.0 - VoH);
}

float F_Schlick(float f0, float f90, float VoH) {
    // not using mix to keep the vec3 and float versions identical
    return f0 + (f90 - f0) * pow5(1.0 - VoH);
}

vec3 fresnel(vec3 f0, float LoH) {
    // f_90 suitable for ambient occlusion
    // see https://google.github.io/filament/Filament.html#lighting/occlusion
    float f90 = saturate(dot(f0, vec3(50.0 * 0.33)));
    return F_Schlick(f0, f90, LoH);
}

// Specular BRDF
// https://google.github.io/filament/Filament.html#materialsystem/specularbrdf

// Cook-Torrance approximation of the microfacet model integration using Fresnel law F to model f_m
// f_r(v,l) = { D(h,α) G(v,l,α) F(v,h,f0) } / { 4 (n⋅v) (n⋅l) }
vec3 specular(vec3 f0, float roughness, const vec3 h, float NoV, float NoL,
              float NoH, float LoH, float specularIntensity) {
    float D = D_GGX(roughness, NoH, h);
    float V = V_SmithGGXCorrelated(roughness, NoV, NoL);
    vec3 F = fresnel(f0, LoH);

    return (specularIntensity * D * V) * F;
}

// Diffuse BRDF
// https://google.github.io/filament/Filament.html#materialsystem/diffusebrdf
// fd(v,l) = σ/π * 1 / { |n⋅v||n⋅l| } ∫Ω D(m,α) G(v,l,m) (v⋅m) (l⋅m) dm

// simplest approximation
// float Fd_Lambert() {
//     return 1.0 / PI;
// }
//
// vec3 Fd = diffuseColor * Fd_Lambert();

// Disney approximation
// See https://google.github.io/filament/Filament.html#citation-burley12
// minimal quality difference
float Fd_Burley(float roughness, float NoV, float NoL, float LoH) {
    float f90 = 0.5 + 2.0 * roughness * LoH * LoH;
    float lightScatter = F_Schlick(1.0, f90, NoL);
    float viewScatter = F_Schlick(1.0, f90, NoV);
    return lightScatter * viewScatter * (1.0 / PI);
}

// From https://www.unrealengine.com/en-US/blog/physically-based-shading-on-mobile
vec3 EnvBRDFApprox(vec3 f0, float perceptual_roughness, float NoV) {
    const vec4 c0 = { -1, -0.0275, -0.572, 0.022 };
    const vec4 c1 = { 1, 0.0425, 1.04, -0.04 };
    vec4 r = perceptual_roughness * c0 + c1;
    float a004 = min(r.x * r.x, exp2(-9.28 * NoV)) * r.x + r.y;
    vec2 AB = vec2(-1.04, 1.04) * a004 + r.zw;
    return f0 * AB.x + AB.y;
}

float perceptualRoughnessToRoughness(float perceptualRoughness) {
    // clamp perceptual roughness to prevent precision problems
    // According to Filament design 0.089 is recommended for mobile
    // Filament uses 0.045 for non-mobile
    float clampedPerceptualRoughness = clamp(perceptualRoughness, 0.089, 1.0);
    return clampedPerceptualRoughness * clampedPerceptualRoughness;
}

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

vec3 point_light(PointLight light, float roughness, float NdotV, vec3 N, vec3 V, vec3 R, vec3 F0, vec3 diffuseColor) {
    vec3 light_to_frag = light.position.xyz - v_WorldPosition.xyz;
    float distance_square = dot(light_to_frag, light_to_frag);
    float rangeAttenuation =
        getDistanceAttenuation(distance_square, light.range);

    // Specular.
    // Representative Point Area Lights.
    // see http://blog.selfshadow.com/publications/s2013-shading-course/karis/s2013_pbs_epic_notes_v2.pdf p14-16
    float a = roughness;
    vec3 centerToRay = dot(light_to_frag, R) * R - light_to_frag;
    vec3 closestPoint = light_to_frag + centerToRay * saturate(light.radius * inversesqrt(dot(centerToRay, centerToRay)));
    float LspecLengthInverse = inversesqrt(dot(closestPoint, closestPoint));
    float normalizationFactor = a / saturate(a + (light.radius * 0.5 * LspecLengthInverse));
    float specularIntensity = normalizationFactor * normalizationFactor;

    vec3 L = closestPoint * LspecLengthInverse; // normalize() equivalent?
    vec3 H = normalize(L + V);
    float NoL = saturate(dot(N, L));
    float NoH = saturate(dot(N, H));
    float LoH = saturate(dot(L, H));

    vec3 specular = specular(F0, roughness, H, NdotV, NoL, NoH, LoH, specularIntensity);

    // Diffuse.
    // Comes after specular since its NoL is used in the lighting equation.
    L = normalize(light_to_frag);
    H = normalize(L + V);
    NoL = saturate(dot(N, L));
    NoH = saturate(dot(N, H));
    LoH = saturate(dot(L, H));

    vec3 diffuse = diffuseColor * Fd_Burley(roughness, NdotV, NoL, LoH);

    // Lout = f(v,l) Φ / { 4 π d^2 }⟨n⋅l⟩
    // where
    // f(v,l) = (f_d(v,l) + f_r(v,l)) * light_color
    // Φ is light intensity

    // our rangeAttentuation = 1 / d^2 multiplied with an attenuation factor for smoothing at the edge of the non-physical maximum light radius
    // It's not 100% clear where the 1/4π goes in the derivation, but we follow the filament shader and leave it out

    // See https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminanceEquation
    // TODO compensate for energy loss https://google.github.io/filament/Filament.html#materialsystem/improvingthebrdfs/energylossinspecularreflectance
    // light.color.rgb is premultiplied with light.intensity on the CPU
    return ((diffuse + specular) * light.color.rgb) * (rangeAttenuation * NoL);
}

float fetch_shadow(int light_id, vec4 homogeneous_coords) {
    if (homogeneous_coords.w <= 0.0) {
        return 1.0;
    }
    // compensate for the Y-flip difference between the NDC and texture coordinates
    const vec2 flip_correction = vec2(0.5, -0.5);
    // compute texture coordinates for shadow lookup
    vec4 light_local = vec4(
        homogeneous_coords.xy * flip_correction/homogeneous_coords.w + 0.5,
        light_id,
        homogeneous_coords.z / homogeneous_coords.w
    );
    // do the lookup, using HW PCF and comparison
    return texture(sampler2DArrayShadow(t_Shadow, s_Shadow), light_local);
}

void main() {
    vec4 color = vec4(0.6, 0.6, 0.6, 1.0); 
    float metallic = 0.01;
    float reflectance = 0.5;
    float perceptual_roughness = 0.089;
    vec3 emissive = vec3(0.0, 0.0, 0.0);
    vec3 ambient_color = vec3(0.1, 0.1, 0.1);
    float occlusion = 1.0;

    float roughness = perceptualRoughnessToRoughness(perceptual_roughness);    
    vec3 N = normalize(v_WorldNormal);
    vec3 V = normalize(ViewWorldPosition.xyz - v_WorldPosition.xyz);
    vec3 R = reflect(-V, N);
    // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
    float NdotV = max(dot(N, V), 1e-4);

    // Remapping [0,1] reflectance to F0
    // See https://google.github.io/filament/Filament.html#materialsystem/parameterization/remapping
    vec3 F0 = 0.16 * reflectance * reflectance * (1.0 - metallic) + color.rgb * metallic;

    // Diffuse strength inversely related to metallicity
    vec3 diffuse_color = color.rgb * (1.0 - metallic);

    vec3 output_color = vec3(0.0);
    for (int i = 0; i < int(NumLights); ++i) {
        PointLight light = PointLights[i];
        vec3 light_contrib = point_light(light, roughness, NdotV, N, V, R, F0, diffuse_color);
        float shadow = fetch_shadow(i, light.projection * v_WorldPosition);
        output_color += light_contrib * shadow;
    }

    vec3 diffuse_ambient = EnvBRDFApprox(diffuse_color, 1.0, NdotV);
    vec3 specular_ambient = EnvBRDFApprox(F0, perceptual_roughness, NdotV);

    output_color += (diffuse_ambient + specular_ambient) * ambient_color * occlusion;
    output_color += emissive * color.a;

    // tone_mapping
    output_color = reinhard_luminance(output_color);
    // Gamma correction.
    // Not needed with sRGB buffer
    // output_color = pow(output_color, vec3(1.0 / 2.2));

    o_Target = vec4(output_color, 1.0);
}
