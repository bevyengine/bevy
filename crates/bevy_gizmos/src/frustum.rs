//! Module for the drawing of [`Frustum`]s.

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Changed, Or, With, Without},
    reflect::ReflectComponent,
    removal_detection::RemovedComponents,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_math::Vec3;
use bevy_reflect::{std_traits::ReflectDefault, Reflect, ReflectFromReflect};
use bevy_render::{
    color::Color,
    primitives::{Frustum, HalfSpace},
    view::VisibilitySystems,
};

use crate::{color_from_entity, GizmoConfig, LineGizmo};

/// Plugin for the drawing of [`Frustum`]s.
pub struct FrustumGizmoPlugin;

impl Plugin for FrustumGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                frustum_gizmos,
                all_frustum_gizmos.run_if(|config: Res<GizmoConfig>| config.frustum.draw_all),
                remove_frustum_gizmos.run_if(|config: Res<GizmoConfig>| !config.frustum.draw_all),
            )
                .after(VisibilitySystems::UpdateOrthographicFrusta)
                .after(VisibilitySystems::UpdatePerspectiveFrusta)
                .after(VisibilitySystems::UpdateProjectionFrusta),
        );
    }
}

/// Configuration for drawing the [`Frustum`] component on entities.
#[derive(Clone, Default)]
pub struct FrustumGizmoConfig {
    /// Draws all frusta in the scene when set to `true`.
    ///
    /// To draw a specific entity's frustum, you can add the [`FrustumGizmo`] component.
    ///
    /// Defaults to `false`.
    pub draw_all: bool,
    /// The default color for frustum gizmos.
    ///
    /// A random color is chosen per frustum if `None`.
    ///
    /// Defaults to `None`.
    pub default_color: Option<Color>,
}

/// Add this [`Component`] to an entity to draw its [`Frustum`] component.
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component, FromReflect, Default)]
pub struct FrustumGizmo {
    /// The color of the frustum.
    ///
    /// The default color from the [`GizmoConfig`] resource is used if `None`,
    pub color: Option<Color>,
}

fn frustum_gizmos(
    query: Query<
        (Entity, &Frustum, &FrustumGizmo, Option<&Handle<LineGizmo>>),
        Or<(Changed<Frustum>, Changed<FrustumGizmo>)>,
    >,
    config: Res<GizmoConfig>,
    mut commands: Commands,
    mut lines: ResMut<Assets<LineGizmo>>,
    mut removed: RemovedComponents<FrustumGizmo>,
) {
    for entity in removed.read() {
        if !query.contains(entity) {
            commands.entity(entity).remove::<Handle<LineGizmo>>();
        }
    }

    for (entity, frustum, gizmo, line_handle) in &query {
        let color = gizmo
            .color
            .or(config.frustum.default_color)
            .unwrap_or_else(|| color_from_entity(entity));

        frustum_inner(
            &mut commands,
            &mut lines,
            entity,
            frustum,
            line_handle,
            color,
        );
    }
}

fn all_frustum_gizmos(
    query: Query<
        (Entity, &Frustum, Option<&Handle<LineGizmo>>),
        (Without<FrustumGizmo>, Changed<Frustum>),
    >,
    config: Res<GizmoConfig>,
    mut commands: Commands,
    mut lines: ResMut<Assets<LineGizmo>>,
) {
    for (entity, frustum, line_handle) in &query {
        let color = config
            .frustum
            .default_color
            .unwrap_or_else(|| color_from_entity(entity));

        frustum_inner(
            &mut commands,
            &mut lines,
            entity,
            frustum,
            line_handle,
            color,
        );
    }
}

fn frustum_inner(
    commands: &mut Commands,
    lines: &mut ResMut<Assets<LineGizmo>>,
    entity: Entity,
    frustum: &Frustum,
    line_handle: Option<&Handle<LineGizmo>>,
    color: Color,
) {
    let Some(positions) = calculate_frustum_positions(frustum) else {
        return;
    };

    let line = LineGizmo {
        colors: std::iter::repeat(color.as_linear_rgba_f32())
            .take(positions.len())
            .collect(),
        positions,
        strip: true,
    };

    if let Some(handle) = line_handle {
        lines.insert(handle, line);
    } else {
        commands.entity(entity).insert(lines.add(line));
    }
}

fn remove_frustum_gizmos(
    query: Query<Entity, (With<Handle<LineGizmo>>, Without<FrustumGizmo>)>,
    mut commands: Commands,
) {
    for entity in &query {
        commands.entity(entity).remove::<Handle<LineGizmo>>();
    }
}

fn calculate_frustum_positions(frustum: &Frustum) -> Option<Vec<[f32; 3]>> {
    let [left, right, top, bottom, near, far] = frustum.half_spaces;

    // Near
    let tln = intersect_halfspaces(top, left, near)?;
    let trn = intersect_halfspaces(top, right, near)?;
    let brn = intersect_halfspaces(bottom, right, near)?;
    let bln = intersect_halfspaces(bottom, left, near)?;
    // Far
    let tlf = intersect_halfspaces(top, left, far)?;
    let trf = intersect_halfspaces(top, right, far)?;
    let brf = intersect_halfspaces(bottom, right, far)?;
    let blf = intersect_halfspaces(bottom, left, far)?;

    #[rustfmt::skip]
    let positions = [
        tln, trn, brn, bln, tln, // Near
        tlf, trf, brf, blf, tlf, // Far
        Vec3::NAN, trn, trf, // Near to far
        Vec3::NAN, brn, brf,
        Vec3::NAN, bln, blf,
    ];

    Some(positions.into_iter().map(|v| v.to_array()).collect())
}

/// Returns the intersection position if the three halfspaces all intersect at a single point.
fn intersect_halfspaces(a: HalfSpace, b: HalfSpace, c: HalfSpace) -> Option<Vec3> {
    let an = a.normal();
    let bn = b.normal();
    let cn = c.normal();

    let x = Vec3::new(an.x, bn.x, cn.x);
    let y = Vec3::new(an.y, bn.y, cn.y);
    let z = Vec3::new(an.z, bn.z, cn.z);

    let d = -Vec3::new(a.d(), b.d(), c.d());

    let u = y.cross(z);
    let v = x.cross(d);

    let denom = x.dot(u);

    if denom.abs() < f32::EPSILON {
        return None;
    }

    Some(Vec3::new(d.dot(u), z.dot(v), -y.dot(v)) / denom)
}
