//! Module for the drawing of [`Frustum`]s.

use bevy_app::{Plugin, PostUpdate};
use bevy_camera::{primitives::Frustum, visibility::VisibilitySystems};
use bevy_color::Color;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Without,
    reflect::ReflectComponent,
    schedule::{IntoScheduleConfigs, SystemSet},
    system::{Query, Res},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect, ReflectFromReflect};

use crate::{
    color_from_entity,
    config::{GizmoConfigGroup, GizmoConfigStore},
    gizmos::Gizmos,
    AppGizmoBuilder,
};

/// A [`Plugin`] that provides visualization of [`Frustum`]s for debugging.
pub struct FrustumGizmoPlugin;

/// Frustum Gizmo system set. This exists in [`PostUpdate`].
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct FrustumGizmoSystems;

impl Plugin for FrustumGizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_gizmo_group::<FrustumGizmoConfigGroup>()
            .add_systems(
                PostUpdate,
                (
                    draw_frustum_gizmos,
                    draw_all_frustum_gizmos.run_if(|config: Res<GizmoConfigStore>| {
                        config.config::<FrustumGizmoConfigGroup>().1.draw_all
                    }),
                )
                    .in_set(FrustumGizmoSystems)
                    .after(VisibilitySystems::UpdateFrusta),
            );
    }
}

/// The [`GizmoConfigGroup`] used for debug visualizations of [`Frustum`] components on entities
#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
#[reflect(Clone, Default)]
pub struct FrustumGizmoConfigGroup {
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
pub struct ShowFrustumGizmo {
    /// The color of the frustum.
    ///
    /// The default color from the [`GizmoConfig`] resource is used if `None`,
    pub color: Option<Color>,
}

fn draw_frustum_gizmos(
    query: Query<(Entity, &Frustum, &ShowFrustumGizmo)>,
    mut gizmos: Gizmos<FrustumGizmoConfigGroup>,
) {
    for (entity, &frustum, gizmo) in &query {
        let color = gizmo
            .color
            .or(gizmos.config_ext.default_color)
            .unwrap_or_else(|| color_from_entity(entity));

        frustum_inner(&frustum, color, &mut gizmos);
    }
}

fn draw_all_frustum_gizmos(
    query: Query<(Entity, &Frustum), Without<ShowFrustumGizmo>>,
    mut gizmos: Gizmos<FrustumGizmoConfigGroup>,
) {
    for (entity, &frustum) in &query {
        let color = gizmos
            .config_ext
            .default_color
            .unwrap_or_else(|| color_from_entity(entity));

        frustum_inner(&frustum, color, &mut gizmos);
    }
}

fn frustum_inner(frustum: &Frustum, color: Color, gizmos: &mut Gizmos<FrustumGizmoConfigGroup>) {
    let Some([tln, trn, brn, bln, tlf, trf, brf, blf]) = frustum.corners() else {
        return;
    };

    gizmos.linestrip(
        [
            tln, trn, brn, bln, // Near
            tln, tlf, // Top Left Near to Far
            trf, brf, blf, tlf, // Far
        ],
        color,
    );
    gizmos.line(trn, trf, color); // Top Right Near to Far
    gizmos.line(brn, brf, color); // Bottom Right Near to Far
    gizmos.line(bln, blf, color); // Bottom Left Near to Far
}
