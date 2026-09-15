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
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: HairMaterial;

/// The raw vertex layout produced by `HairStrands::to_mesh`.
struct HairVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) tangent: vec3<f32>,
    /// x: ribbon side (-1 / +1), y: root->tip parameter, z: per-strand seed.
    @location(2) params: vec3<f32>,
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

/// Expands a control point sideways so the ribbon faces `view`.
///
/// `world_from_local` is the model matrix of the instance being drawn; the
/// offset is applied in world space so non-uniform scale never squashes the
/// ribbon width.
fn expand_ribbon(
    world_from_local: mat4x4<f32>,
    local_position: vec3<f32>,
    local_tangent: vec3<f32>,
    side: f32,
    t: f32,
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
    out.world_position = vec4(centre + side_axis * (side * half_width_at(t)), 1.0);
    out.world_tangent = tangent;
    // The shading normal is the view direction projected off the tangent so it
    // stays orthogonal to the strand; this is the normal the ribbon visually has.
    out.world_normal = normalize(to_view - tangent * dot(tangent, to_view));
    return out;
}
