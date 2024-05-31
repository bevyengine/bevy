//! A module adding debug visualization of [`PointLight`]s, [`SpotLight`]s and [`DirectionalLight`]s.

use std::f32::consts::PI;

use crate::{self as bevy_gizmos, primitives::dim3::GizmoPrimitive3d};

use bevy_app::{Plugin, PostUpdate};
use bevy_color::{
    palettes::basic::{BLUE, GREEN, RED},
    Color, Oklcha,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Without,
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{Query, Res},
};
use bevy_math::{
    primitives::{Cone, Sphere},
    Quat, Vec3,
};
use bevy_pbr::{DirectionalLight, PointLight, SpotLight};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::{components::GlobalTransform, TransformSystem};

use crate::{
    config::{GizmoConfigGroup, GizmoConfigStore},
    gizmos::Gizmos,
    AppGizmoBuilder,
};

/// Draws a standard sphere for the radius and an axis sphere for the range.
fn point_light_gizmo(
    transform: &GlobalTransform,
    point_light: &PointLight,
    color: Color,
    gizmos: &mut Gizmos<LightGizmoConfigGroup>,
) {
    let position = transform.translation();
    gizmos
        .primitive_3d(
            &Sphere {
                radius: point_light.radius,
            },
            position,
            Quat::IDENTITY,
            color,
        )
        .resolution(16);
    gizmos
        .sphere(position, Quat::IDENTITY, point_light.range, color)
        .resolution(32);
}

/// Draws a sphere for the radius, two cones for the inner and outer angles, plus two 3d arcs crossing the
/// farthest point of effect of the spot light along its direction.
fn spot_light_gizmo(
    transform: &GlobalTransform,
    spot_light: &SpotLight,
    color: Color,
    gizmos: &mut Gizmos<LightGizmoConfigGroup>,
) {
    let (_, rotation, translation) = transform.to_scale_rotation_translation();
    gizmos
        .primitive_3d(
            &Sphere {
                radius: spot_light.radius,
            },
            translation,
            Quat::IDENTITY,
            color,
        )
        .resolution(16);

    // Offset the tip of the cone to the light position.
    for angle in [spot_light.inner_angle, spot_light.outer_angle] {
        let height = spot_light.range * angle.cos();
        let position = translation + rotation * Vec3::NEG_Z * height / 2.0;
        gizmos
            .primitive_3d(
                &Cone {
                    radius: spot_light.range * angle.sin(),
                    height,
                },
                position,
                rotation * Quat::from_rotation_x(PI / 2.0),
                color,
            )
            .height_resolution(4)
            .base_resolution(32);
    }

    for arc_rotation in [
        Quat::from_rotation_y(PI / 2.0 - spot_light.outer_angle),
        Quat::from_euler(
            bevy_math::EulerRot::XZY,
            0.0,
            PI / 2.0,
            PI / 2.0 - spot_light.outer_angle,
        ),
    ] {
        gizmos
            .arc_3d(
                2.0 * spot_light.outer_angle,
                spot_light.range,
                translation,
                rotation * arc_rotation,
                color,
            )
            .resolution(16);
    }
}

/// Draws an arrow alongside the directional light direction.
fn directional_light_gizmo(
    transform: &GlobalTransform,
    color: Color,
    gizmos: &mut Gizmos<LightGizmoConfigGroup>,
) {
    let (_, rotation, translation) = transform.to_scale_rotation_translation();
    gizmos
        .arrow(translation, translation + rotation * Vec3::NEG_Z, color)
        .with_tip_length(0.3);
}

/// A [`Plugin`] that provides visualization of [`PointLight`]s, [`SpotLight`]s
/// and [`DirectionalLight`]s for debugging.
pub struct LightGizmoPlugin;

impl Plugin for LightGizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_type::<LightGizmoConfigGroup>()
            .init_gizmo_group::<LightGizmoConfigGroup>()
            .add_systems(
                PostUpdate,
                (
                    draw_lights,
                    draw_all_lights.run_if(|config: Res<GizmoConfigStore>| {
                        config.config::<LightGizmoConfigGroup>().1.draw_all
                    }),
                )
                    .after(TransformSystem::TransformPropagate),
            );
    }
}

/// Configures how a color is attributed to a light gizmo.
#[derive(Debug, Clone, Copy, Default, Reflect)]
pub enum LightGizmoColor {
    /// User-specified color.
    Manual(Color),
    /// Random color derived from the light's [`Entity`].
    Varied,
    /// Take the color of the represented light.
    #[default]
    MatchLightColor,
    /// Take the color provided by [`LightGizmoConfigGroup`] depending on the light kind.
    ByLightType,
}

/// The [`GizmoConfigGroup`] used to configure the visualization of lights.
#[derive(Clone, Reflect, GizmoConfigGroup)]
pub struct LightGizmoConfigGroup {
    /// Draw a gizmo for all lights if true.
    ///
    /// Defaults to `false`.
    pub draw_all: bool,
    /// Default color strategy for all light gizmos.
    ///
    /// Defaults to [`LightGizmoColor::MatchLightColor`].
    pub color: LightGizmoColor,
    /// [`Color`] to use for drawing a [`PointLight`] gizmo when [`LightGizmoColor::ByLightType`] is used.
    ///
    /// Defaults to [`RED`].
    pub point_light_color: Color,
    /// [`Color`] to use for drawing a [`SpotLight`] gizmo when [`LightGizmoColor::ByLightType`] is used.
    ///
    /// Defaults to [`GREEN`].
    pub spot_light_color: Color,
    /// [`Color`] to use for drawing a [`DirectionalLight`] gizmo when [`LightGizmoColor::ByLightType`] is used.
    ///
    /// Defaults to [`BLUE`].
    pub directional_light_color: Color,
}

impl Default for LightGizmoConfigGroup {
    fn default() -> Self {
        Self {
            draw_all: false,
            color: LightGizmoColor::MatchLightColor,
            point_light_color: RED.into(),
            spot_light_color: GREEN.into(),
            directional_light_color: BLUE.into(),
        }
    }
}

/// Add this [`Component`] to an entity to draw any of its lights components
/// ([`PointLight`], [`SpotLight`] and [`DirectionalLight`]).
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component, Default)]
pub struct ShowLightGizmo {
    /// Default color strategy for this light gizmo. if [`None`], use the one provided by [`LightGizmoConfigGroup`].
    ///
    /// Defaults to [`None`].
    pub color: Option<LightGizmoColor>,
}

fn draw_lights(
    point_query: Query<(Entity, &PointLight, &GlobalTransform, &ShowLightGizmo)>,
    spot_query: Query<(Entity, &SpotLight, &GlobalTransform, &ShowLightGizmo)>,
    directional_query: Query<(Entity, &DirectionalLight, &GlobalTransform, &ShowLightGizmo)>,
    mut gizmos: Gizmos<LightGizmoConfigGroup>,
) {
    let color = |entity: Entity, gizmo_color: Option<LightGizmoColor>, light_color, type_color| {
        match gizmo_color.unwrap_or(gizmos.config_ext.color) {
            LightGizmoColor::Manual(color) => color,
            LightGizmoColor::Varied => Oklcha::sequential_dispersed(entity.index()).into(),
            LightGizmoColor::MatchLightColor => light_color,
            LightGizmoColor::ByLightType => type_color,
        }
    };
    for (entity, light, transform, light_gizmo) in &point_query {
        let color = color(
            entity,
            light_gizmo.color,
            light.color,
            gizmos.config_ext.point_light_color,
        );
        point_light_gizmo(transform, light, color, &mut gizmos);
    }
    for (entity, light, transform, light_gizmo) in &spot_query {
        let color = color(
            entity,
            light_gizmo.color,
            light.color,
            gizmos.config_ext.spot_light_color,
        );
        spot_light_gizmo(transform, light, color, &mut gizmos);
    }
    for (entity, light, transform, light_gizmo) in &directional_query {
        let color = color(
            entity,
            light_gizmo.color,
            light.color,
            gizmos.config_ext.directional_light_color,
        );
        directional_light_gizmo(transform, color, &mut gizmos);
    }
}

fn draw_all_lights(
    point_query: Query<(Entity, &PointLight, &GlobalTransform), Without<ShowLightGizmo>>,
    spot_query: Query<(Entity, &SpotLight, &GlobalTransform), Without<ShowLightGizmo>>,
    directional_query: Query<
        (Entity, &DirectionalLight, &GlobalTransform),
        Without<ShowLightGizmo>,
    >,
    mut gizmos: Gizmos<LightGizmoConfigGroup>,
) {
    match gizmos.config_ext.color {
        LightGizmoColor::Manual(color) => {
            for (_, light, transform) in &point_query {
                point_light_gizmo(transform, light, color, &mut gizmos);
            }
            for (_, light, transform) in &spot_query {
                spot_light_gizmo(transform, light, color, &mut gizmos);
            }
            for (_, _, transform) in &directional_query {
                directional_light_gizmo(transform, color, &mut gizmos);
            }
        }
        LightGizmoColor::Varied => {
            let color = |entity: Entity| Oklcha::sequential_dispersed(entity.index()).into();
            for (entity, light, transform) in &point_query {
                point_light_gizmo(transform, light, color(entity), &mut gizmos);
            }
            for (entity, light, transform) in &spot_query {
                spot_light_gizmo(transform, light, color(entity), &mut gizmos);
            }
            for (entity, _, transform) in &directional_query {
                directional_light_gizmo(transform, color(entity), &mut gizmos);
            }
        }
        LightGizmoColor::MatchLightColor => {
            for (_, light, transform) in &point_query {
                point_light_gizmo(transform, light, light.color, &mut gizmos);
            }
            for (_, light, transform) in &spot_query {
                spot_light_gizmo(transform, light, light.color, &mut gizmos);
            }
            for (_, light, transform) in &directional_query {
                directional_light_gizmo(transform, light.color, &mut gizmos);
            }
        }
        LightGizmoColor::ByLightType => {
            for (_, light, transform) in &point_query {
                point_light_gizmo(
                    transform,
                    light,
                    gizmos.config_ext.point_light_color,
                    &mut gizmos,
                );
            }
            for (_, light, transform) in &spot_query {
                spot_light_gizmo(
                    transform,
                    light,
                    gizmos.config_ext.spot_light_color,
                    &mut gizmos,
                );
            }
            for (_, _, transform) in &directional_query {
                directional_light_gizmo(
                    transform,
                    gizmos.config_ext.directional_light_color,
                    &mut gizmos,
                );
            }
        }
    }
}
