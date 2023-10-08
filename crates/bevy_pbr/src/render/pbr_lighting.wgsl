#define_import_path bevy_pbr::lighting

#import bevy_pbr::prepass_utils as prepass_utils
#import bevy_pbr::utils PI
#import bevy_pbr::mesh_view_types as view_types
#import bevy_pbr::mesh_view_bindings as view_bindings

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
fn specular(
    f0: vec3<f32>,
    roughness: f32,
    h: vec3<f32>,
    NoV: f32,
    NoL: f32,
    NoH: f32,
    LoH: f32,
    specularIntensity: f32,
    f_ab: vec2<f32>
) -> vec3<f32> {
    let D = D_GGX(roughness, NoH, h);
    let V = V_SmithGGXCorrelated(roughness, NoV, NoL);
    let F = fresnel(f0, LoH);

    var Fr = (specularIntensity * D * V) * F;

    // Multiscattering approximation: https://google.github.io/filament/Filament.html#listing_energycompensationimpl
    Fr *= 1.0 + f0 * (1.0 / f_ab.x - 1.0);

    return Fr;
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

// Scale/bias approximation
// https://www.unrealengine.com/en-US/blog/physically-based-shading-on-mobile
// TODO: Use a LUT (more accurate)
fn F_AB(perceptual_roughness: f32, NoV: f32) -> vec2<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = perceptual_roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * NoV)) * r.x + r.y;
    return vec2<f32>(-1.04, 1.04) * a004 + r.zw;
}

fn EnvBRDFApprox(f0: vec3<f32>, f_ab: vec2<f32>) -> vec3<f32> {
    return f0 * f_ab.x + f_ab.y;
}

fn perceptualRoughnessToRoughness(perceptualRoughness: f32) -> f32 {
    // clamp perceptual roughness to prevent precision problems
    // According to Filament design 0.089 is recommended for mobile
    // Filament uses 0.045 for non-mobile
    let clampedPerceptualRoughness = clamp(perceptualRoughness, 0.089, 1.0);
    return clampedPerceptualRoughness * clampedPerceptualRoughness;
}

fn point_light(
    world_position: vec3<f32>,
    light_id: u32,
    roughness: f32,
    NdotV: f32,
    N: vec3<f32>,
    V: vec3<f32>,
    R: vec3<f32>,
    F0: vec3<f32>,
    f_ab: vec2<f32>,
    diffuseColor: vec3<f32>
) -> vec3<f32> {
    let light = &view_bindings::point_lights.data[light_id];
    let light_to_frag = (*light).position_radius.xyz - world_position.xyz;
    let distance_square = dot(light_to_frag, light_to_frag);
    let rangeAttenuation = getDistanceAttenuation(distance_square, (*light).color_inverse_square_range.w);

    // Specular.
    // Representative Point Area Lights.
    // see http://blog.selfshadow.com/publications/s2013-shading-course/karis/s2013_pbs_epic_notes_v2.pdf p14-16
    let a = roughness;
    let centerToRay = dot(light_to_frag, R) * R - light_to_frag;
    let closestPoint = light_to_frag + centerToRay * saturate((*light).position_radius.w * inverseSqrt(dot(centerToRay, centerToRay)));
    let LspecLengthInverse = inverseSqrt(dot(closestPoint, closestPoint));
    let normalizationFactor = a / saturate(a + ((*light).position_radius.w * 0.5 * LspecLengthInverse));
    let specularIntensity = normalizationFactor * normalizationFactor;

    var L: vec3<f32> = closestPoint * LspecLengthInverse; // normalize() equivalent?
    var H: vec3<f32> = normalize(L + V);
    var NoL: f32 = saturate(dot(N, L));
    var NoH: f32 = saturate(dot(N, H));
    var LoH: f32 = saturate(dot(L, H));

    let specular_light = specular(F0, roughness, H, NdotV, NoL, NoH, LoH, specularIntensity, f_ab);

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
    // our rangeAttenuation = 1 / d^2 multiplied with an attenuation factor for smoothing at the edge of the non-physical maximum light radius

    // For a point light, luminous intensity, I, in lumens per steradian is given by:
    // I = Φ / 4 π
    // The derivation of this can be seen here: https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminousPower

    // NOTE: (*light).color.rgb is premultiplied with (*light).intensity / 4 π (which would be the luminous intensity) on the CPU

    return ((diffuse + specular_light) * (*light).color_inverse_square_range.rgb) * (rangeAttenuation * NoL);
}

fn spot_light(
    world_position: vec3<f32>,
    light_id: u32,
    roughness: f32,
    NdotV: f32,
    N: vec3<f32>,
    V: vec3<f32>,
    R: vec3<f32>,
    F0: vec3<f32>,
    f_ab: vec2<f32>,
    diffuseColor: vec3<f32>
) -> vec3<f32> {
    // reuse the point light calculations
    let point_light = point_light(world_position, light_id, roughness, NdotV, N, V, R, F0, f_ab, diffuseColor);

    let light = &view_bindings::point_lights.data[light_id];

    // reconstruct spot dir from x/z and y-direction flag
    var spot_dir = vec3<f32>((*light).light_custom_data.x, 0.0, (*light).light_custom_data.y);
    spot_dir.y = sqrt(max(0.0, 1.0 - spot_dir.x * spot_dir.x - spot_dir.z * spot_dir.z));
    if ((*light).flags & view_types::POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u {
        spot_dir.y = -spot_dir.y;
    }
    let light_to_frag = (*light).position_radius.xyz - world_position.xyz;

    // calculate attenuation based on filament formula https://google.github.io/filament/Filament.html#listing_glslpunctuallight
    // spot_scale and spot_offset have been precomputed
    // note we normalize here to get "l" from the filament listing. spot_dir is already normalized
    let cd = dot(-spot_dir, normalize(light_to_frag));
    let attenuation = saturate(cd * (*light).light_custom_data.z + (*light).light_custom_data.w);
    let spot_attenuation = attenuation * attenuation;

    return point_light * spot_attenuation;
}

fn directional_light(light_id: u32, roughness: f32, NdotV: f32, normal: vec3<f32>, view: vec3<f32>, R: vec3<f32>, F0: vec3<f32>, f_ab: vec2<f32>, diffuseColor: vec3<f32>) -> vec3<f32> {
    let light = &view_bindings::lights.directional_lights[light_id];

    let incident_light = (*light).direction_to_light.xyz;

    let half_vector = normalize(incident_light + view);
    let NoL = saturate(dot(normal, incident_light));
    let NoH = saturate(dot(normal, half_vector));
    let LoH = saturate(dot(incident_light, half_vector));

    let diffuse = diffuseColor * Fd_Burley(roughness, NdotV, NoL, LoH);
    let specularIntensity = 1.0;
    let specular_light = specular(F0, roughness, half_vector, NdotV, NoL, NoH, LoH, specularIntensity, f_ab);

    return (specular_light + diffuse) * (*light).color.rgb * NoL;
}

fn specular_transmissive_light(world_position: vec4<f32>, frag_coord: vec3<f32>, view_z: f32, N: vec3<f32>, V: vec3<f32>, ior: f32, thickness: f32, perceptual_roughness: f32, specular_transmissive_color: vec3<f32>, transmitted_environment_light_specular: vec3<f32>) -> vec3<f32> {
    // Calculate the ratio between refaction indexes. Assume air/vacuum for the space outside the mesh
    let eta = 1.0 / ior;

    // Calculate incidence vector (opposite to view vector) and its dot product with the mesh normal
    let I = -V;
    let NdotI = dot(N, I);

    // Calculate refracted direction using Snell's law
    let k = 1.0 - eta * eta * (1.0 - NdotI * NdotI);
    let T = eta * I - (eta * NdotI + sqrt(k)) * N;

    // Calculate the exit position of the refracted ray, by propagating refacted direction through thickness
    let exit_position = world_position.xyz + T * thickness;

    // Transform exit_position into clip space
    let clip_exit_position = view_bindings::view.view_proj * vec4<f32>(exit_position, 1.0);

    // Scale / offset position so that coordinate is in right space for sampling transmissive background texture
    let offset_position = (clip_exit_position.xy / clip_exit_position.w) * vec2<f32>(0.5, -0.5) + 0.5;

    // Fetch background color
    var background_color: vec4<f32>;
    if perceptual_roughness == 0.0 {
        // If the material has zero roughness, we can use a faster approach without the blur
        background_color = fetch_transmissive_background_non_rough(offset_position);
    } else {
        background_color = fetch_transmissive_background(offset_position, frag_coord, view_z, perceptual_roughness);
    }

    // Calculate final color by applying specular transmissive color to a mix of background color and transmitted specular environment light
    return specular_transmissive_color * mix(transmitted_environment_light_specular, background_color.rgb, background_color.a);
}

// https://blog.demofox.org/2022/01/01/interleaved-gradient-noise-a-different-kind-of-low-discrepancy-sequence
fn interleaved_gradient_noise(pixel_coordinates: vec2<f32>) -> f32 {
#ifdef TEMPORAL_JITTER
    let frame = f32(view_bindings::globals.frame_count % 64u);
    let xy = pixel_coordinates + 5.588238 * frame;
#else
    // Don't vary noise per frame if TAA is not enabled
    let xy = pixel_coordinates;
#endif
    return fract(52.9829189 * fract(0.06711056 * xy.x + 0.00583715 * xy.y));
}

fn fetch_transmissive_background_non_rough(offset_position: vec2<f32>) -> vec4<f32> {
    return textureSample(
        view_bindings::view_transmission_texture,
        view_bindings::view_transmission_sampler,
        offset_position,
    );
}

fn fetch_transmissive_background(offset_position: vec2<f32>, frag_coord: vec3<f32>, view_z: f32, perceptual_roughness: f32) -> vec4<f32> {
    // Calculate view aspect ratio, used to scale offset so that it's proportionate
    let aspect = view_bindings::view.viewport.z / view_bindings::view.viewport.w;

    // Calculate how “blurry” the transmission should be.
    // Blur is more or less eyeballed to look approximately “right”, since the “correct”
    // approach would involve projecting many scattered rays and figuring out their individual
    // exit positions. IRL, light rays can be scattered when entering/exiting a material (due to
    // roughness) or inside the material (due to subsurface scattering). Here, we only consider
    // the first scenario.
    //
    // Blur intensity is:
    // - squarely proportional to `perceptual_roughness`
    // - inversely proportional to view z
    let blur_intensity = (perceptual_roughness * perceptual_roughness) / view_z;

#ifdef SCREEN_SPACE_TRANSMISSION_BLUR_TAPS
    let num_taps = #{SCREEN_SPACE_TRANSMISSION_BLUR_TAPS}; // Controlled by the `Camera3d::transmissive_quality` property
#else
    let num_taps = 8; // Fallback to 8 taps
#endif
    let num_spirals = i32(ceil(f32(num_taps) / 8.0));
    let random_angle = interleaved_gradient_noise(frag_coord.xy);

    // Pixel checkerboard pattern (helps make the interleaved gradient noise pattern less visible)
    let pixel_checkboard = (
#ifdef TEMPORAL_JITTER
        // 0 or 1 on even/odd pixels, alternates every frame
        (i32(frag_coord.x) + i32(frag_coord.y) + i32(view_bindings::globals.frame_count)) % 2
#else
        // 0 or 1 on even/odd pixels
        (i32(frag_coord.x) + i32(frag_coord.y)) % 2
#endif
    );

    var result = vec4<f32>(0.0);
    for (var i: i32 = 0; i < num_taps; i = i + 1) {
        let current_spiral = (i >> 3u);
        let angle = (random_angle + f32(current_spiral) / f32(num_spirals)) * 2.0 * PI;
        let m = vec2(sin(angle), cos(angle));
        let rotation_matrix = mat2x2(
            m.y, -m.x,
            m.x, m.y
        );

        // Get spiral offset
        var spiral_offset: vec2<f32>;
        switch i & 7 {
            // https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare (slides 120-135)
            // TODO: Figure out a more reasonable way of doing this, as WGSL
            // seems to only allow constant indexes into constant arrays
            case 0: { spiral_offset = vec2<f32>(-0.7071,  0.7071); }
            case 1: { spiral_offset = vec2<f32>(-0.0000, -0.8750); }
            case 2: { spiral_offset = vec2<f32>( 0.5303,  0.5303); }
            case 3: { spiral_offset = vec2<f32>(-0.6250, -0.0000); }
            case 4: { spiral_offset = vec2<f32>( 0.3536, -0.3536); }
            case 5: { spiral_offset = vec2<f32>(-0.0000,  0.3750); }
            case 6: { spiral_offset = vec2<f32>(-0.1768, -0.1768); }
            case 7: { spiral_offset = vec2<f32>( 0.1250,  0.0000); }
            default: {}
        }

        // Make each consecutive spiral slightly smaller than the previous one
        spiral_offset *= 1.0 - (0.5 * f32(current_spiral + 1) / f32(num_spirals));

        // Rotate and correct for aspect ratio
        let rotated_spiral_offset = (rotation_matrix * spiral_offset) * vec2(1.0, aspect);

        // Calculate final offset position, with blur and spiral offset
        let modified_offset_position = offset_position + rotated_spiral_offset * blur_intensity * (1.0 - f32(pixel_checkboard) * 0.1);

#ifdef PREPASS_DEPTH_SUPPORTED
        // Use depth prepass data to reject values that are in front of the current fragment
        if prepass_utils::prepass_depth(vec4<f32>(modified_offset_position * view_bindings::view.viewport.zw, 0.0, 0.0), 0u) > frag_coord.z {
            continue;
        }
#endif

        // Sample the view transmission texture at the offset position + noise offset, to get the background color
        let sample = textureSample(
            view_bindings::view_transmission_texture,
            view_bindings::view_transmission_sampler,
            modified_offset_position,
        );

        // As blur intensity grows higher, gradually limit *very bright* color RGB values towards a
        // maximum length of 1.0 to prevent stray “firefly” pixel artifacts. This can potentially make
        // very strong emissive meshes appear much dimmer, but the artifacts are noticeable enough to
        // warrant this treatment.
        let normalized_rgb = normalize(sample.rgb);
        result += vec4(min(sample.rgb, normalized_rgb / clamp(blur_intensity / 2.0, 0.0, 1.0)), sample.a);
    }

    result /= f32(num_taps);

    return result;
}
