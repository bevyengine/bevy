//! This module is for 'retained' alternatives to the 'immediate mode' [`Gizmos`](crate::gizmos::Gizmos) system parameter.

use core::ops::{Deref, DerefMut};

use bevy_asset::Handle;
#[cfg(feature = "bevy_render")]
use bevy_camera::visibility::RenderLayers;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::components::Transform;

#[cfg(feature = "bevy_render")]
use {
    crate::{config::GizmoLineJoint, LineGizmoUniform},
    bevy_ecs::{
        entity::Entity,
        system::{Commands, Local, Query},
    },
    bevy_render::Extract,
    bevy_transform::components::GlobalTransform,
};

use crate::{
    config::{ErasedGizmoConfigGroup, GizmoLineConfig},
    gizmos::GizmoBuffer,
    GizmoAsset,
};

impl Deref for GizmoAsset {
    type Target = GizmoBuffer<ErasedGizmoConfigGroup, ()>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for GizmoAsset {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

/// A component that draws the gizmos of a [`GizmoAsset`].
///
/// When drawing a greater number of static lines a [`Gizmo`] component can
/// have far better performance than the [`Gizmos`] system parameter,
/// but the system parameter will perform better for smaller lines that update often.
///
/// ## Example
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_gizmos::prelude::*;
/// # use bevy_asset::prelude::*;
/// # use bevy_color::palettes::css::*;
/// # use bevy_utils::default;
/// # use bevy_math::prelude::*;
/// fn system(
///     mut commands: Commands,
///     mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
/// ) {
///     let mut gizmo = GizmoAsset::default();
///
///     gizmo.sphere(Vec3::ZERO, 1., RED);
///
///     commands.spawn(Gizmo {
///         handle: gizmo_assets.add(gizmo),
///         line_config: GizmoLineConfig {
///             width: 4.,
///             ..default()
///         },
///         ..default()
///     });
/// }
/// ```
///
/// [`Gizmos`]: crate::gizmos::Gizmos
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Clone, Default)]
#[require(Transform)]
pub struct Gizmo {
    /// The handle to the gizmo to draw.
    pub handle: Handle<GizmoAsset>,
    /// The line specific configuration for this gizmo.
    pub line_config: GizmoLineConfig,
    /// How closer to the camera than real geometry the gizmo should be.
    ///
    /// In 2D this setting has no effect and is effectively always -1.
    ///
    /// Value between -1 and 1 (inclusive).
    /// * 0 means that there is no change to the gizmo position when rendering
    /// * 1 means it is furthest away from camera as possible
    /// * -1 means that it will always render in front of other things.
    ///
    /// This is typically useful if you are drawing wireframes on top of polygons
    /// and your wireframe is z-fighting (flickering on/off) with your main model.
    /// You would set this value to a negative number close to 0.
    pub depth_bias: f32,
}

#[cfg(feature = "bevy_render")]
pub(crate) fn extract_linegizmos(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(Entity, &Gizmo, &GlobalTransform, Option<&RenderLayers>)>>,
) {
    use bevy_math::Affine3;
    use bevy_render::sync_world::{MainEntity, TemporaryRenderEntity};
    use bevy_utils::once;
    use tracing::warn;

    use crate::config::GizmoLineStyle;

    let mut values = Vec::with_capacity(*previous_len);
    for (entity, gizmo, transform, render_layers) in &query {
        let joints_resolution = if let GizmoLineJoint::Round(resolution) = gizmo.line_config.joints
        {
            resolution
        } else {
            0
        };
        let (gap_scale, line_scale) = if let GizmoLineStyle::Dashed {
            gap_scale,
            line_scale,
        } = gizmo.line_config.style
        {
            if gap_scale <= 0.0 {
                once!(warn!("when using gizmos with the line style `GizmoLineStyle::Dashed{{..}}` the gap scale should be greater than zero"));
            }
            if line_scale <= 0.0 {
                once!(warn!("when using gizmos with the line style `GizmoLineStyle::Dashed{{..}}` the line scale should be greater than zero"));
            }
            (gap_scale, line_scale)
        } else {
            (1.0, 1.0)
        };

        values.push((
            LineGizmoUniform {
                world_from_local: Affine3::from(&transform.affine()).to_transpose(),
                line_width: gizmo.line_config.width,
                depth_bias: gizmo.depth_bias,
                joints_resolution,
                gap_scale,
                line_scale,
                #[cfg(feature = "webgl")]
                _padding: Default::default(),
            },
            #[cfg(any(feature = "bevy_pbr", feature = "bevy_sprite"))]
            crate::config::GizmoMeshConfig {
                line_perspective: gizmo.line_config.perspective,
                line_style: gizmo.line_config.style,
                line_joints: gizmo.line_config.joints,
                render_layers: render_layers.cloned().unwrap_or_default(),
                handle: gizmo.handle.clone(),
            },
            MainEntity::from(entity),
            TemporaryRenderEntity,
        ));
    }
    *previous_len = values.len();
    commands.spawn_batch(values);
}
