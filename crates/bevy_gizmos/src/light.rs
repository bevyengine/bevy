//! A module adding debug visualization of [`PointLight`]s, [`SpotLight`]s and [`DirectionalLight`]s.

use std::f32::consts::PI;

use crate::{self as bevy_gizmos, primitives::dim3::GizmoPrimitive3d};

use bevy_app::{Plugin, PostUpdate};
use bevy_color::LinearRgba;
use bevy_ecs::{
    component::Component,
    query::{With, Without},
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

fn draw_gizmos<'a, P, S, D>(
    point_lights: P,
    spot_lights: S,
    directional_lights: D,
    gizmos: &mut Gizmos<LightGizmoConfigGroup>,
) where
    P: 'a + IntoIterator<Item = (&'a PointLight, &'a GlobalTransform)>,
    S: 'a + IntoIterator<Item = (&'a SpotLight, &'a GlobalTransform)>,
    D: 'a + IntoIterator<Item = (&'a DirectionalLight, &'a GlobalTransform)>,
{
    // Standard sphere for the radius, axis sphere for the range.
    for (point_light, transform) in point_lights {
        let position = transform.translation();
        gizmos
            .primitive_3d(
                Sphere {
                    radius: point_light.radius,
                },
                position,
                Quat::IDENTITY,
                point_light.color,
            )
            .segments(16);
        gizmos
            .sphere(
                position,
                Quat::IDENTITY,
                point_light.range,
                point_light.color,
            )
            .circle_segments(32);
    }

    // A sphere for the radius, two cones for the inner and outer angles, plus two 3d arcs crossing the
    // farthest point of effect of the spot light along its direction.
    for (spot_light, transform) in spot_lights {
        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        gizmos
            .primitive_3d(
                Sphere {
                    radius: spot_light.radius,
                },
                translation,
                Quat::IDENTITY,
                spot_light.color,
            )
            .segments(16);

        // Offset the tip of the cone to the light position.
        gizmos.sphere(translation, Quat::IDENTITY, 0.01, LinearRgba::GREEN);
        for angle in [spot_light.inner_angle, spot_light.outer_angle] {
            let height = spot_light.range * angle.cos();
            let position = translation + rotation * Vec3::NEG_Z * height / 2.0;
            gizmos
                .primitive_3d(
                    Cone {
                        radius: spot_light.range * angle.sin(),
                        height,
                    },
                    position,
                    rotation * Quat::from_rotation_x(PI / 2.0),
                    spot_light.color,
                )
                .height_segments(4)
                .base_segments(32);
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
                    spot_light.color,
                )
                .segments(16);
        }
    }

    // An arrow alongside the directional light direction.
    for (directional_light, transform) in directional_lights {
        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        gizmos
            .arrow(
                translation,
                translation + rotation * Vec3::NEG_Z,
                directional_light.color,
            )
            .with_tip_length(0.3);
    }
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

/// The [`GizmoConfigGroup`] used to configure the visualization of lights.
#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
pub struct LightGizmoConfigGroup {
    /// Draw a gizmo for all lights if true.
    pub draw_all: bool,
}

/// Add this [`Component`] to an entity to draw any of its lights components
/// ([`PointLight`], [`SpotLight`] and [`DirectionalLight`]).
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component, Default)]
pub struct ShowLightGizmo;

fn draw_lights(
    point_query: Query<(&PointLight, &GlobalTransform), With<ShowLightGizmo>>,
    spot_query: Query<(&SpotLight, &GlobalTransform), With<ShowLightGizmo>>,
    directional_query: Query<(&DirectionalLight, &GlobalTransform), With<ShowLightGizmo>>,
    mut gizmos: Gizmos<LightGizmoConfigGroup>,
) {
    draw_gizmos(&point_query, &spot_query, &directional_query, &mut gizmos);
}

fn draw_all_lights(
    point_query: Query<(&PointLight, &GlobalTransform), Without<ShowLightGizmo>>,
    spot_query: Query<(&SpotLight, &GlobalTransform), Without<ShowLightGizmo>>,
    directional_query: Query<(&DirectionalLight, &GlobalTransform), Without<ShowLightGizmo>>,
    mut gizmos: Gizmos<LightGizmoConfigGroup>,
) {
    draw_gizmos(&point_query, &spot_query, &directional_query, &mut gizmos);
}
