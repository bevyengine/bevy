//! A module adding debug visualization of [`Aabb`]s.

use crate as bevy_gizmos;

use bevy_app::{Plugin, PostUpdate};
use bevy_color::{Color, Hsla};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Without,
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{Query, Res},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::primitives::Aabb;
use bevy_transform::{
    components::{GlobalTransform, Transform},
    TransformSystem,
};

use crate::{
    config::{GizmoConfigGroup, GizmoConfigStore},
    gizmos::Gizmos,
    AppGizmoBuilder,
};

/// A [`Plugin`] that provides visualization of [`Aabb`]s for debugging.
pub struct AabbGizmoPlugin;

impl Plugin for AabbGizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_type::<AabbGizmoConfigGroup>()
            .init_gizmo_group::<AabbGizmoConfigGroup>()
            .add_systems(
                PostUpdate,
                (
                    draw_aabbs,
                    draw_all_aabbs.run_if(|config: Res<GizmoConfigStore>| {
                        config.config::<AabbGizmoConfigGroup>().1.draw_all
                    }),
                )
                    .after(TransformSystem::TransformPropagate),
            );
    }
}
/// The [`GizmoConfigGroup`] used for debug visualizations of [`Aabb`] components on entities
#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
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
#[reflect(Component, Default)]
pub struct ShowAabbGizmo {
    /// The color of the box.
    ///
    /// The default color from the [`AabbGizmoConfigGroup`] config is used if `None`,
    pub color: Option<Color>,
}

fn draw_aabbs(
    query: Query<(Entity, &Aabb, &GlobalTransform, &ShowAabbGizmo)>,
    mut gizmos: Gizmos<AabbGizmoConfigGroup>,
) {
    for (entity, &aabb, &transform, gizmo) in &query {
        let color = gizmo
            .color
            .or(gizmos.config_ext.default_color)
            .unwrap_or_else(|| color_from_entity(entity));
        gizmos.cuboid(aabb_transform(aabb, transform), color);
    }
}

fn draw_all_aabbs(
    query: Query<(Entity, &Aabb, &GlobalTransform), Without<ShowAabbGizmo>>,
    mut gizmos: Gizmos<AabbGizmoConfigGroup>,
) {
    for (entity, &aabb, &transform) in &query {
        let color = gizmos
            .config_ext
            .default_color
            .unwrap_or_else(|| color_from_entity(entity));
        gizmos.cuboid(aabb_transform(aabb, transform), color);
    }
}

fn color_from_entity(entity: Entity) -> Color {
    let index = entity.index();

    // from https://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences/
    //
    // See https://en.wikipedia.org/wiki/Low-discrepancy_sequence
    // Map a sequence of integers (eg: 154, 155, 156, 157, 158) into the [0.0..1.0] range,
    // so that the closer the numbers are, the larger the difference of their image.
    const FRAC_U32MAX_GOLDEN_RATIO: u32 = 2654435769; // (u32::MAX / Φ) rounded up
    const RATIO_360: f32 = 360.0 / u32::MAX as f32;
    let hue = index.wrapping_mul(FRAC_U32MAX_GOLDEN_RATIO) as f32 * RATIO_360;

    Hsla::hsl(hue, 1., 0.5).into()
}

fn aabb_transform(aabb: Aabb, transform: GlobalTransform) -> GlobalTransform {
    transform
        * GlobalTransform::from(
            Transform::from_translation(aabb.center.into())
                .with_scale((aabb.half_extents * 2.).into()),
        )
}
