// Shared bindings and helpers for hair strand ribbons. Used by both the
// forward shader (`hair.wesl`) and the prepass shader (`hair_prepass.wesl`).

import bevy_pbr::render::mesh_view_bindings::view;

/// Mirrors `HairMaterialUniform` on the Rust side.
struct HairMaterial {
    base_color: vec4<f32>,
    specular_color: vec4<f32>,
    secondary_specular_color: vec4<f32>,
    root_width: f32,
    tip_width: f32,
    specular_shift: f32,
    specular_exponent: f32,
    secondary_shift: f32,
    secondary_exponent: f32,
    root_occlusion: f32,
    color_variation: f32,
    min_pixel_width: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: HairMaterial;

/// The smallest fraction of an entity's strands that level of detail keeps,
/// so the survivors are never widened by more than its reciprocal. Keep in
/// step with `MIN_KEEP_FRACTION` in `strands.rs`.
const MIN_KEEP_FRACTION: f32 = 0.125;

/// The raw vertex layout produced by `HairStrands::to_mesh`.
struct HairVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    /// Unit strand direction (`Snorm8x4`; w is unused).
    @location(1) tangent: vec4<f32>,
    /// `Unorm8x4`: x: ribbon side (0 / 1), y: root->tip parameter,
    /// z: per-strand seed, w: unused.
    @location(2) params: vec4<f32>,
}

/// The vertex's parameters as the shaders use them: x: ribbon side (-1 / +1),
/// y: root->tip parameter, z: per-strand seed in [0, 1].
fn decode_params(packed: vec4<f32>) -> vec3<f32> {
    return vec3(packed.x * 2.0 - 1.0, packed.y, packed.z);
}

/// A control point pushed out to one edge of its ribbon, in world space.
struct RibbonPoint {
    world_position: vec4<f32>,
    /// Unit strand direction.
    world_tangent: vec3<f32>,
    /// Unit vector facing the viewer, orthogonal to the tangent.
    world_normal: vec3<f32>,
}

/// Direction from `world_position` towards the viewer of the current `view`.
/// For orthographic views (including directional shadow cascades) this is the
/// constant view direction.
fn direction_to_view(world_position: vec3<f32>) -> vec3<f32> {
    let is_orthographic = view.clip_from_view[3][3] == 1.0;
    if is_orthographic {
        return normalize(vec3(
            view.clip_from_world[0].z,
            view.clip_from_world[1].z,
            view.clip_from_world[2].z,
        ));
    }
    return normalize(view.world_position - world_position);
}

/// Half of the ribbon width at parameter `t` along the strand.
fn half_width_at(t: f32) -> f32 {
    return 0.5 * mix(material.root_width, material.tip_width, t);
}

/// How many pixels of the current `view` one world unit covers at
/// `world_position`: constant for an orthographic view (a directional shadow
/// cascade included), falling off with distance for a perspective one.
fn pixels_per_unit(world_position: vec3<f32>) -> f32 {
    let is_orthographic = view.clip_from_view[3][3] == 1.0;
    // `clip_from_view[1][1]` is 1 / tan(fov / 2) for a perspective projection
    // and 2 / height for an orthographic one; either way half the viewport
    // height times it is the pixels per unit at unit distance.
    var pixels = 0.5 * view.viewport.w * view.clip_from_view[1][1];
    if !is_orthographic {
        pixels = pixels / max(length(view.world_position - world_position), 1e-6);
    }
    return pixels;
}

/// Level of detail for the strand with `seed` on the instance drawn by
/// `world_from_local`, as a multiplier on its width: `0.0` drops the strand
/// (its ribbon collapses to its centre line and rasterizes nothing), more
/// than `1.0` widens it to cover for the dropped ones.
///
/// Once a strand's root would be drawn thinner than
/// `material.min_pixel_width` pixels, only the fraction of the strands that
/// keeps the survivors that wide is drawn. The seeds are a low-discrepancy
/// sequence over the strand order, so the survivors are spread evenly. The
/// decision is per strand and per view, so the depth prepass and the main
/// pass agree, and each shadow map thins by its own resolution.
fn strand_width_scale(world_from_local: mat4x4<f32>, seed: f32) -> f32 {
    if material.min_pixel_width <= 0.0 {
        return 1.0;
    }
    let root_pixels = material.root_width * pixels_per_unit(world_from_local[3].xyz);
    let keep = root_pixels / material.min_pixel_width;
    if keep >= 1.0 {
        return 1.0;
    }
    let kept = max(keep, MIN_KEEP_FRACTION);
    return select(0.0, 1.0 / kept, seed < kept);
}

/// Expands a control point sideways so the ribbon faces `view`.
///
/// `world_from_local` is the model matrix of the instance being drawn; the
/// offset is applied in world space so non-uniform scale never squashes the
/// ribbon width. `width_scale` is the strand's level of detail from
/// `strand_width_scale`.
fn expand_ribbon(
    world_from_local: mat4x4<f32>,
    local_position: vec3<f32>,
    local_tangent: vec3<f32>,
    side: f32,
    t: f32,
    width_scale: f32,
) -> RibbonPoint {
    let centre = (world_from_local * vec4(local_position, 1.0)).xyz;
    var tangent = (world_from_local * vec4(local_tangent, 0.0)).xyz;
    let tangent_length = length(tangent);
    if tangent_length > 1e-6 {
        tangent = tangent / tangent_length;
    } else {
        tangent = vec3(0.0, 1.0, 0.0);
    }

    let to_view = direction_to_view(centre);

    // The ribbon's width axis is perpendicular to both the strand and the view
    // direction. When the strand points straight at the camera the cross
    // product vanishes, so fall back to any axis perpendicular to the tangent.
    var side_axis = cross(tangent, to_view);
    let side_length = length(side_axis);
    if side_length > 1e-4 {
        side_axis = side_axis / side_length;
    } else {
        let helper = select(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), abs(tangent.y) > 0.9);
        side_axis = normalize(cross(tangent, helper));
    }

    var out: RibbonPoint;
    out.world_position = vec4(centre + side_axis * (side * half_width_at(t) * width_scale), 1.0);
    out.world_tangent = tangent;
    // The shading normal is the view direction projected off the tangent so it
    // stays orthogonal to the strand; this is the normal the ribbon visually has.
    out.world_normal = normalize(to_view - tangent * dot(tangent, to_view));
    return out;
}
