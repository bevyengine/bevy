//! A module adding debug visualization of [`Aabb`]s.

use bevy_app::{Plugin, PostUpdate};
use bevy_camera::{primitives::Aabb, visibility::ViewVisibility};
use bevy_color::Color;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Without,
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Query, Res},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::{components::GlobalTransform, TransformSystems};

use crate::{
    color_from_entity,
    config::{GizmoConfigGroup, GizmoConfigStore},
    gizmos::Gizmos,
    AppGizmoBuilder,
};

/// A [`Plugin`] that provides visualization of [`Aabb`]s for debugging.
pub struct AabbGizmoPlugin;

impl Plugin for AabbGizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_gizmo_group::<AabbGizmoConfigGroup>().add_systems(
            PostUpdate,
            (
                draw_aabbs,
                draw_all_aabbs.run_if(|config: Res<GizmoConfigStore>| {
                    config.config::<AabbGizmoConfigGroup>().1.draw_all
                }),
            )
                .after(bevy_camera::visibility::VisibilitySystems::MarkNewlyHiddenEntitiesInvisible)
                .after(TransformSystems::Propagate),
        );
    }
}
/// The [`GizmoConfigGroup`] used for debug visualizations of [`Aabb`] components on entities
#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
#[reflect(Clone, Default)]
pub struct AabbGizmoConfigGroup {
    /// Draws all bounding boxes in the scene when set to `true`.
    ///
    /// To draw a specific entity's bounding box, you can add the [`ShowAabbGizmo`] component.
    ///
    /// Defaults to `false`.
    pub draw_all: bool,
    /// The default color for bounding box gizmos.
    ///
    /// A random color is chosen per box if `None`.
    ///
    /// Defaults to `None`.
    pub default_color: Option<Color>,
}

/// Add this [`Component`] to an entity to draw its [`Aabb`] component.
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component, Default, Debug)]
pub struct ShowAabbGizmo {
    /// The color of the box.
    ///
    /// The default color from the [`AabbGizmoConfigGroup`] config is used if `None`,
    pub color: Option<Color>,
}

fn draw_aabbs(
    query: Query<(
        Entity,
        &Aabb,
        &GlobalTransform,
        Option<&ViewVisibility>,
        &ShowAabbGizmo,
    )>,
    mut gizmos: Gizmos<AabbGizmoConfigGroup>,
) {
    for (entity, &aabb, &transform, view_visibility, gizmo) in &query {
        if !is_visible(view_visibility) {
            continue;
        }

        let color = gizmo
            .color
            .or(gizmos.config_ext.default_color)
            .unwrap_or_else(|| color_from_entity(entity));
        gizmos.aabb_3d(aabb, transform, color);
    }
}

fn draw_all_aabbs(
    query: Query<
        (Entity, &Aabb, &GlobalTransform, Option<&ViewVisibility>),
        Without<ShowAabbGizmo>,
    >,
    mut gizmos: Gizmos<AabbGizmoConfigGroup>,
) {
    for (entity, &aabb, &transform, view_visibility) in &query {
        if !is_visible(view_visibility) {
            continue;
        }

        let color = gizmos
            .config_ext
            .default_color
            .unwrap_or_else(|| color_from_entity(entity));
        gizmos.aabb_3d(aabb, transform, color);
    }
}

fn is_visible(view_visibility: Option<&ViewVisibility>) -> bool {
    view_visibility.is_some_and(|v| v.get())
}
