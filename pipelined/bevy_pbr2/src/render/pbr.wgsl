// NOTE: Keep in sync with depth.wgsl
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
};


[[block]]
struct Mesh {
    model: mat4x4<f32>;
    inverse_transpose_model: mat4x4<f32>;
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32;
};

let MESH_FLAGS_SHADOW_RECEIVER_BIT: u32 = 1u;

[[group(0), binding(0)]]
var<uniform> view: View;
[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] tangent: vec4<f32>;
#endif
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] world_tangent: vec4<f32>;
#endif
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.uv = vertex.uv;
    out.world_position = world_position;
    out.clip_position = view.view_proj * world_position;
    out.world_normal = mat3x3<f32>(
        mesh.inverse_transpose_model.x.xyz,
        mesh.inverse_transpose_model.y.xyz,
        mesh.inverse_transpose_model.z.xyz
    ) * vertex.normal;
#ifdef VERTEX_TANGENTS
    out.world_tangent = vec4<f32>(
        mat3x3<f32>(
            mesh.model.x.xyz,
            mesh.model.y.xyz,
            mesh.model.z.xyz
        ) * vertex.tangent.xyz,
        vertex.tangent.w
    );
#endif
    return out;
}

// From the Filament design doc
// https://google.github.io/filament/Filament.html#table_symbols
// Symbol Definition
// v    View unit vector
// l    Incident light unit vector
// n    Surface normal unit vector
// h    Half unit vector between l and v
// f    BRDF
// f_d    Diffuse component of a BRDF
// f_r    Specular component of a BRDF
// α    Roughness, remapped from using input perceptualRoughness
// σ    Diffuse reflectance
// Ω    Spherical domain
// f0    Reflectance at normal incidence
// f90    Reflectance at grazing angle
// χ+(a)    Heaviside function (1 if a>0 and 0 otherwise)
// nior    Index of refraction (IOR) of an interface
// ⟨n⋅l⟩    Dot product clamped to [0..1]
// ⟨a⟩    Saturated value (clamped to [0..1])

// The Bidirectional Reflectance Distribution Function (BRDF) describes the surface response of a standard material
// and consists of two components, the diffuse component (f_d) and the specular component (f_r):
// f(v,l) = f_d(v,l) + f_r(v,l)
//
// The form of the microfacet model is the same for diffuse and specular
// f_r(v,l) = f_d(v,l) = 1 / { |n⋅v||n⋅l| } ∫_Ω D(m,α) G(v,l,m) f_m(v,l,m) (v⋅m) (l⋅m) dm
//
// In which:
// D, also called the Normal Distribution Function (NDF) models the distribution of the microfacets
// G models the visibility (or occlusion or shadow-masking) of the microfacets
// f_m is the microfacet BRDF and differs between specular and diffuse components
//
// The above integration needs to be approximated.

[[block]]
struct StandardMaterial {
    base_color: vec4<f32>;
    emissive: vec4<f32>;
    perceptual_roughness: f32;
    metallic: f32;
    reflectance: f32;
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32;
};

let STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT: u32         = 1u;
let STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT: u32           = 2u;
let STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT: u32 = 4u;
let STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT: u32          = 8u;
let STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT: u32               = 16u;
let STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32                      = 32u;

struct PointLight {
    projection: mat4x4<f32>;
    color: vec4<f32>;
    position: vec3<f32>;
    inverse_square_range: f32;
    radius: f32;
    near: f32;
    far: f32;
    shadow_depth_bias: f32;
    shadow_normal_bias: f32;
};

struct DirectionalLight {
    view_projection: mat4x4<f32>;
    color: vec4<f32>;
    direction_to_light: vec3<f32>;
    shadow_depth_bias: f32;
    shadow_normal_bias: f32;
};

[[block]]
struct Lights {
    // NOTE: this array size must be kept in sync with the constants defined bevy_pbr2/src/render/light.rs
    // TODO: this can be removed if we move to storage buffers for light arrays
    point_lights: array<PointLight, 10>;
    directional_lights: array<DirectionalLight, 1>;
    ambient_color: vec4<f32>;
    n_point_lights: u32;
    n_directional_lights: u32;
};


[[group(0), binding(1)]]
var<uniform> lights: Lights;
[[group(0), binding(2)]]
var point_shadow_textures: texture_depth_cube_array;
[[group(0), binding(3)]]
var point_shadow_textures_sampler: sampler_comparison;
[[group(0), binding(4)]]
var directional_shadow_textures: texture_depth_2d_array;
[[group(0), binding(5)]]
var directional_shadow_textures_sampler: sampler_comparison;

[[group(1), binding(0)]]
var<uniform> material: StandardMaterial;
[[group(1), binding(1)]]
var base_color_texture: texture_2d<f32>;
[[group(1), binding(2)]]
var base_color_sampler: sampler;
[[group(1), binding(3)]]
var emissive_texture: texture_2d<f32>;
[[group(1), binding(4)]]
var emissive_sampler: sampler;
[[group(1), binding(5)]]
var metallic_roughness_texture: texture_2d<f32>;
[[group(1), binding(6)]]
var metallic_roughness_sampler: sampler;
[[group(1), binding(7)]]
var occlusion_texture: texture_2d<f32>;
[[group(1), binding(8)]]
var occlusion_sampler: sampler;
[[group(1), binding(9)]]
var normal_map_texture: texture_2d<f32>;
[[group(1), binding(10)]]
var normal_map_sampler: sampler;

let PI: f32 = 3.141592653589793;

fn saturate(value: f32) -> f32 {
    return clamp(value, 0.0, 1.0);
}

// distanceAttenuation is simply the square falloff of light intensity
// combined with a smooth attenuation at the edge of the light radius
//
// light radius is a non-physical construct for efficiency purposes,
// because otherwise every light affects every fragment in the scene
fn getDistanceAttenuation(distanceSquare: f32, inverseRangeSquared: f32) -> f32 {
    let factor = distanceSquare * inverseRangeSquared;
    let smoothFactor = saturate(1.0 - factor * factor);
    let attenuation = smoothFactor * smoothFactor;
    return attenuation * 1.0 / max(distanceSquare, 0.0001);
}

// Normal distribution function (specular D)
// Based on https://google.github.io/filament/Filament.html#citation-walter07

// D_GGX(h,α) = α^2 / { π ((n⋅h)^2 (α2−1) + 1)^2 }

// Simple implementation, has precision problems when using fp16 instead of fp32
// see https://google.github.io/filament/Filament.html#listing_speculardfp16
fn D_GGX(roughness: f32, NoH: f32, h: vec3<f32>) -> f32 {
    let oneMinusNoHSquared = 1.0 - NoH * NoH;
    let a = NoH * roughness;
    let k = roughness / (oneMinusNoHSquared + a * a);
    let d = k * k * (1.0 / PI);
    return d;
}

// Visibility function (Specular G)
// V(v,l,a) = G(v,l,α) / { 4 (n⋅v) (n⋅l) }
// such that f_r becomes
// f_r(v,l) = D(h,α) V(v,l,α) F(v,h,f0)
// where
// V(v,l,α) = 0.5 / { n⋅l sqrt((n⋅v)^2 (1−α2) + α2) + n⋅v sqrt((n⋅l)^2 (1−α2) + α2) }
// Note the two sqrt's, that may be slow on mobile, see https://google.github.io/filament/Filament.html#listing_approximatedspecularv
fn V_SmithGGXCorrelated(roughness: f32, NoV: f32, NoL: f32) -> f32 {
    let a2 = roughness * roughness;
    let lambdaV = NoL * sqrt((NoV - a2 * NoV) * NoV + a2);
    let lambdaL = NoV * sqrt((NoL - a2 * NoL) * NoL + a2);
    let v = 0.5 / (lambdaV + lambdaL);
    return v;
}

// Fresnel function
// see https://google.github.io/filament/Filament.html#citation-schlick94
// F_Schlick(v,h,f_0,f_90) = f_0 + (f_90 − f_0) (1 − v⋅h)^5
fn F_Schlick_vec(f0: vec3<f32>, f90: f32, VoH: f32) -> vec3<f32> {
    // not using mix to keep the vec3 and float versions identical
    return f0 + (f90 - f0) * pow(1.0 - VoH, 5.0);
}

fn F_Schlick(f0: f32, f90: f32, VoH: f32) -> f32 {
    // not using mix to keep the vec3 and float versions identical
    return f0 + (f90 - f0) * pow(1.0 - VoH, 5.0);
}

fn fresnel(f0: vec3<f32>, LoH: f32) -> vec3<f32> {
    // f_90 suitable for ambient occlusion
    // see https://google.github.io/filament/Filament.html#lighting/occlusion
    let f90 = saturate(dot(f0, vec3<f32>(50.0 * 0.33)));
    return F_Schlick_vec(f0, f90, LoH);
}

// Specular BRDF
// https://google.github.io/filament/Filament.html#materialsystem/specularbrdf

// Cook-Torrance approximation of the microfacet model integration using Fresnel law F to model f_m
// f_r(v,l) = { D(h,α) G(v,l,α) F(v,h,f0) } / { 4 (n⋅v) (n⋅l) }
fn specular(f0: vec3<f32>, roughness: f32, h: vec3<f32>, NoV: f32, NoL: f32,
              NoH: f32, LoH: f32, specularIntensity: f32) -> vec3<f32> {
    let D = D_GGX(roughness, NoH, h);
    let V = V_SmithGGXCorrelated(roughness, NoV, NoL);
    let F = fresnel(f0, LoH);

    return (specularIntensity * D * V) * F;
}

// Diffuse BRDF
// https://google.github.io/filament/Filament.html#materialsystem/diffusebrdf
// fd(v,l) = σ/π * 1 / { |n⋅v||n⋅l| } ∫Ω D(m,α) G(v,l,m) (v⋅m) (l⋅m) dm
//
// simplest approximation
// float Fd_Lambert() {
//     return 1.0 / PI;
// }
//
// vec3 Fd = diffuseColor * Fd_Lambert();
//
// Disney approximation
// See https://google.github.io/filament/Filament.html#citation-burley12
// minimal quality difference
fn Fd_Burley(roughness: f32, NoV: f32, NoL: f32, LoH: f32) -> f32 {
    let f90 = 0.5 + 2.0 * roughness * LoH * LoH;
    let lightScatter = F_Schlick(1.0, f90, NoL);
    let viewScatter = F_Schlick(1.0, f90, NoV);
    return lightScatter * viewScatter * (1.0 / PI);
}

// From https://www.unrealengine.com/en-US/blog/physically-based-shading-on-mobile
fn EnvBRDFApprox(f0: vec3<f32>, perceptual_roughness: f32, NoV: f32) -> vec3<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = perceptual_roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * NoV)) * r.x + r.y;
    let AB = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    return f0 * AB.x + AB.y;
}

fn perceptualRoughnessToRoughness(perceptualRoughness: f32) -> f32 {
    // clamp perceptual roughness to prevent precision problems
    // According to Filament design 0.089 is recommended for mobile
    // Filament uses 0.045 for non-mobile
    let clampedPerceptualRoughness = clamp(perceptualRoughness, 0.089, 1.0);
    return clampedPerceptualRoughness * clampedPerceptualRoughness;
}

// from https://64.github.io/tonemapping/
// reinhard on RGB oversaturates colors
fn reinhard(color: vec3<f32>) -> vec3<f32> {
    return color / (1.0 + color);
}

fn reinhard_extended(color: vec3<f32>, max_white: f32) -> vec3<f32> {
    let numerator = color * (1.0 + (color / vec3<f32>(max_white * max_white)));
    return numerator / (1.0 + color);
}

// luminance coefficients from Rec. 709.
// https://en.wikipedia.org/wiki/Rec._709
fn luminance(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn change_luminance(c_in: vec3<f32>, l_out: f32) -> vec3<f32> {
    let l_in = luminance(c_in);
    return c_in * (l_out / l_in);
}

fn reinhard_luminance(color: vec3<f32>) -> vec3<f32> {
    let l_old = luminance(color);
    let l_new = l_old / (1.0 + l_old);
    return change_luminance(color, l_new);
}

fn reinhard_extended_luminance(color: vec3<f32>, max_white_l: f32) -> vec3<f32> {
    let l_old = luminance(color);
    let numerator = l_old * (1.0 + (l_old / (max_white_l * max_white_l)));
    let l_new = numerator / (1.0 + l_old);
    return change_luminance(color, l_new);
}

fn point_light(
    world_position: vec3<f32>, light: PointLight, roughness: f32, NdotV: f32, N: vec3<f32>, V: vec3<f32>,
    R: vec3<f32>, F0: vec3<f32>, diffuseColor: vec3<f32>
) -> vec3<f32> {
    let light_to_frag = light.position.xyz - world_position.xyz;
    let distance_square = dot(light_to_frag, light_to_frag);
    let rangeAttenuation =
        getDistanceAttenuation(distance_square, light.inverse_square_range);

    // Specular.
    // Representative Point Area Lights.
    // see http://blog.selfshadow.com/publications/s2013-shading-course/karis/s2013_pbs_epic_notes_v2.pdf p14-16
    let a = roughness;
    let centerToRay = dot(light_to_frag, R) * R - light_to_frag;
    let closestPoint = light_to_frag + centerToRay * saturate(light.radius * inverseSqrt(dot(centerToRay, centerToRay)));
    let LspecLengthInverse = inverseSqrt(dot(closestPoint, closestPoint));
    let normalizationFactor = a / saturate(a + (light.radius * 0.5 * LspecLengthInverse));
    let specularIntensity = normalizationFactor * normalizationFactor;

    var L: vec3<f32> = closestPoint * LspecLengthInverse; // normalize() equivalent?
    var H: vec3<f32> = normalize(L + V);
    var NoL: f32 = saturate(dot(N, L));
    var NoH: f32 = saturate(dot(N, H));
    var LoH: f32 = saturate(dot(L, H));

    let specular_light = specular(F0, roughness, H, NdotV, NoL, NoH, LoH, specularIntensity);

    // Diffuse.
    // Comes after specular since its NoL is used in the lighting equation.
    L = normalize(light_to_frag);
    H = normalize(L + V);
    NoL = saturate(dot(N, L));
    NoH = saturate(dot(N, H));
    LoH = saturate(dot(L, H));

    let diffuse = diffuseColor * Fd_Burley(roughness, NdotV, NoL, LoH);

    // See https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminanceEquation
    // Lout = f(v,l) Φ / { 4 π d^2 }⟨n⋅l⟩
    // where
    // f(v,l) = (f_d(v,l) + f_r(v,l)) * light_color
    // Φ is luminous power in lumens
    // our rangeAttentuation = 1 / d^2 multiplied with an attenuation factor for smoothing at the edge of the non-physical maximum light radius

    // For a point light, luminous intensity, I, in lumens per steradian is given by:
    // I = Φ / 4 π
    // The derivation of this can be seen here: https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminousPower

    // NOTE: light.color.rgb is premultiplied with light.intensity / 4 π (which would be the luminous intensity) on the CPU

    // TODO compensate for energy loss https://google.github.io/filament/Filament.html#materialsystem/improvingthebrdfs/energylossinspecularreflectance

    return ((diffuse + specular_light) * light.color.rgb) * (rangeAttenuation * NoL);
}

fn directional_light(light: DirectionalLight, roughness: f32, NdotV: f32, normal: vec3<f32>, view: vec3<f32>, R: vec3<f32>, F0: vec3<f32>, diffuseColor: vec3<f32>) -> vec3<f32> {
    let incident_light = light.direction_to_light.xyz;

    let half_vector = normalize(incident_light + view);
    let NoL = saturate(dot(normal, incident_light));
    let NoH = saturate(dot(normal, half_vector));
    let LoH = saturate(dot(incident_light, half_vector));

    let diffuse = diffuseColor * Fd_Burley(roughness, NdotV, NoL, LoH);
    let specularIntensity = 1.0;
    let specular_light = specular(F0, roughness, half_vector, NdotV, NoL, NoH, LoH, specularIntensity);

    return (specular_light + diffuse) * light.color.rgb * NoL;
}

fn fetch_point_shadow(light_id: i32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light = lights.point_lights[light_id];

    // because the shadow maps align with the axes and the frustum planes are at 45 degrees
    // we can get the worldspace depth by taking the largest absolute axis
    let surface_to_light = light.position.xyz - frag_position.xyz;
    let surface_to_light_abs = abs(surface_to_light);
    let distance_to_light = max(surface_to_light_abs.x, max(surface_to_light_abs.y, surface_to_light_abs.z));

    // The normal bias here is already scaled by the texel size at 1 world unit from the light.
    // The texel size increases proportionally with distance from the light so multiplying by
    // distance to light scales the normal bias to the texel size at the fragment distance.
    let normal_offset = light.shadow_normal_bias * distance_to_light * surface_normal.xyz;
    let depth_offset = light.shadow_depth_bias * normalize(surface_to_light.xyz);
    let offset_position = frag_position.xyz + normal_offset + depth_offset;

    // similar largest-absolute-axis trick as above, but now with the offset fragment position
    let frag_ls = light.position.xyz - offset_position.xyz;
    let abs_position_ls = abs(frag_ls);
    let major_axis_magnitude = max(abs_position_ls.x, max(abs_position_ls.y, abs_position_ls.z));

    // NOTE: These simplifications come from multiplying:
    //       projection * vec4(0, 0, -major_axis_magnitude, 1.0)
    //       and keeping only the terms that have any impact on the depth.
    // Projection-agnostic approach:
    let z = -major_axis_magnitude * light.projection[2][2] + light.projection[3][2];
    let w = -major_axis_magnitude * light.projection[2][3] + light.projection[3][3];

    // For perspective_rh:
    // let proj_r = light.far / (light.near - light.far);
    // let z = -major_axis_magnitude * proj_r + light.near * proj_r;
    // let w = major_axis_magnitude;

    // For perspective_infinite_reverse_rh:
    // let z = light.near;
    // let w = major_axis_magnitude;

    let depth = z / w;

    // do the lookup, using HW PCF and comparison
    // NOTE: Due to the non-uniform control flow above, we must use the Level variant of
    //       textureSampleCompare to avoid undefined behaviour due to some of the fragments in
    //       a quad (2x2 fragments) being processed not being sampled, and this messing with
    //       mip-mapping functionality. The shadow maps have no mipmaps so Level just samples
    //       from LOD 0.
    return textureSampleCompareLevel(point_shadow_textures, point_shadow_textures_sampler, frag_ls, i32(light_id), depth);
}

fn fetch_directional_shadow(light_id: i32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light = lights.directional_lights[light_id];

    // The normal bias is scaled to the texel size.
    let normal_offset = light.shadow_normal_bias * surface_normal.xyz;
    let depth_offset = light.shadow_depth_bias * light.direction_to_light.xyz;
    let offset_position = vec4<f32>(frag_position.xyz + normal_offset + depth_offset, frag_position.w);

    let offset_position_clip = light.view_projection * offset_position;
    if (offset_position_clip.w <= 0.0) {
        return 1.0;
    }
    let offset_position_ndc = offset_position_clip.xyz / offset_position_clip.w;
    // No shadow outside the orthographic projection volume
    if (any(offset_position_ndc.xy < vec2<f32>(-1.0)) || offset_position_ndc.z < 0.0
            || any(offset_position_ndc > vec3<f32>(1.0))) {
        return 1.0;
    }

    // compute texture coordinates for shadow lookup, compensating for the Y-flip difference
    // between the NDC and texture coordinates
    let flip_correction = vec2<f32>(0.5, -0.5);
    let light_local = offset_position_ndc.xy * flip_correction + vec2<f32>(0.5, 0.5);

    let depth = offset_position_ndc.z;
    // do the lookup, using HW PCF and comparison
    // NOTE: Due to non-uniform control flow above, we must use the level variant of the texture
    //       sampler to avoid use of implicit derivatives causing possible undefined behavior.
    return textureSampleCompareLevel(directional_shadow_textures, directional_shadow_textures_sampler, light_local, i32(light_id), depth);
}

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] world_tangent: vec4<f32>;
#endif
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    var output_color: vec4<f32> = material.base_color;
    if ((material.flags & STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        output_color = output_color * textureSample(base_color_texture, base_color_sampler, in.uv);
    }

    // // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        // TODO use .a for exposure compensation in HDR
        var emissive: vec4<f32> = material.emissive;
        if ((material.flags & STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb * textureSample(emissive_texture, emissive_sampler, in.uv).rgb, 1.0);
        }

        // calculate non-linear roughness from linear perceptualRoughness
        var metallic: f32 = material.metallic;
        var perceptual_roughness: f32 = material.perceptual_roughness;
        if ((material.flags & STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv);
            // Sampling from GLTF standard channels for now
            metallic = metallic * metallic_roughness.b;
            perceptual_roughness = perceptual_roughness * metallic_roughness.g;
        }
        let roughness = perceptualRoughnessToRoughness(perceptual_roughness);

        var occlusion: f32 = 1.0;
        if ((material.flags & STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            occlusion = textureSample(occlusion_texture, occlusion_sampler, in.uv).r;
        }

        var N: vec3<f32> = normalize(in.world_normal);

#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
        var T: vec3<f32> = normalize(in.world_tangent.xyz - N * dot(in.world_tangent.xyz, N));
        var B: vec3<f32> = cross(N, T) * in.world_tangent.w;
#endif
#endif

        if ((material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u) {
            if (!in.is_front) {
                N = -N;
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
                T = -T;
                B = -B;
#endif
#endif
            }
        }

#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
        let TBN = mat3x3<f32>(T, B, N);
        N = TBN * normalize(textureSample(normal_map_texture, normal_map_sampler, in.uv).rgb * 2.0 - 1.0);
#endif
#endif

        var V: vec3<f32>;
        if (view.projection.w.w != 1.0) { // If the projection is not orthographic
            // Only valid for a perpective projection
            V = normalize(view.world_position.xyz - in.world_position.xyz);
        } else {
            // Ortho view vec
            V = normalize(vec3<f32>(view.view_proj.x.z, view.view_proj.y.z, view.view_proj.z.z));
        }

        // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
        let NdotV = max(dot(N, V), 0.0001);

        // Remapping [0,1] reflectance to F0
        // See https://google.github.io/filament/Filament.html#materialsystem/parameterization/remapping
        let reflectance = material.reflectance;
        let F0 = 0.16 * reflectance * reflectance * (1.0 - metallic) + output_color.rgb * metallic;

        // Diffuse strength inversely related to metallicity
        let diffuse_color = output_color.rgb * (1.0 - metallic);

        let R = reflect(-V, N);

        // accumulate color
        var light_accum: vec3<f32> = vec3<f32>(0.0);
        let n_point_lights = i32(lights.n_point_lights);
        let n_directional_lights = i32(lights.n_directional_lights);
        for (var i: i32 = 0; i < n_point_lights; i = i + 1) {
            let light = lights.point_lights[i];
            var shadow: f32;
            if ((mesh.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u) {
                shadow = fetch_point_shadow(i, in.world_position, in.world_normal);
            } else {
                shadow = 1.0;
            }
            let light_contrib = point_light(in.world_position.xyz, light, roughness, NdotV, N, V, R, F0, diffuse_color);
            light_accum = light_accum + light_contrib * shadow;
        }
        for (var i: i32 = 0; i < n_directional_lights; i = i + 1) {
            let light = lights.directional_lights[i];
            var shadow: f32;
            if ((mesh.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u) {
                shadow = fetch_directional_shadow(i, in.world_position, in.world_normal);
            } else {
                shadow = 1.0;
            }
            let light_contrib = directional_light(light, roughness, NdotV, N, V, R, F0, diffuse_color);
            light_accum = light_accum + light_contrib * shadow;
        }

        let diffuse_ambient = EnvBRDFApprox(diffuse_color, 1.0, NdotV);
        let specular_ambient = EnvBRDFApprox(F0, perceptual_roughness, NdotV);

        output_color = vec4<f32>(
            light_accum +
                (diffuse_ambient + specular_ambient) * lights.ambient_color.rgb * occlusion +
                emissive.rgb * output_color.a,
            output_color.a);

        // tone_mapping
        output_color = vec4<f32>(reinhard_luminance(output_color.rgb), output_color.a);
        // Gamma correction.
        // Not needed with sRGB buffer
        // output_color.rgb = pow(output_color.rgb, vec3(1.0 / 2.2));
    }

    return output_color;
}
