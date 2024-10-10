//! This module is for 'retained' alternatives to the 'immediate mode' [`Gizmos`](crate::gizmos::Gizmos) system parameter.

use core::ops::{Deref, DerefMut};

use bevy_asset::Handle;
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;

#[cfg(feature = "bevy_render")]
use {
    crate::{config::GizmoLineJoint, LineGizmoUniform},
    bevy_ecs::{
        entity::Entity,
        system::{Commands, Local, Query},
    },
    bevy_render::{view::RenderLayers, Extract},
    bevy_transform::components::GlobalTransform,
};

use crate::{
    config::{ErasedGizmoConfigGroup, LineGizmoConfig},
    gizmos::GizmoBuffer,
    LineGizmoAsset,
};

impl Deref for LineGizmoAsset {
    type Target = GizmoBuffer<ErasedGizmoConfigGroup, ()>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for LineGizmoAsset {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

/// A component that draws the lines of a [`LineGizmoAsset`].
///
/// When drawing a greater number of lines that don't need to update as often
/// a [`LineGizmo`] can have far better performance than the [`Gizmos`] system parameter.
///
/// ## Example
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_gizmos::prelude::*;
/// # use bevy_asset::prelude::*;
/// # use bevy_color::palettes::css::*;
/// # use bevy_utils::default;
/// fn system(
///     mut commands: Commands,
///     mut linegizmos: ResMut<Assets<LineGizmoAsset>>,
/// ) {
///     let mut linegizmo = LineGizmoAsset::default();
///
///     linegizmo.sphere(default(), 1., RED);
///
///     commands.spawn(LineGizmo {
///         handle: linegizmos.add(linegizmo),
///         config: LineGizmoConfig {
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
#[require(Transform)]
pub struct LineGizmo {
    /// The handle to the line to draw.
    pub handle: Handle<LineGizmoAsset>,
    /// The configuration for this gizmo.
    pub config: LineGizmoConfig,
    /// How closer to the camera than real geometry the line should be.
    ///
    /// In 2D this setting has no effect and is effectively always -1.
    ///
    /// Value between -1 and 1 (inclusive).
    /// * 0 means that there is no change to the line position when rendering
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
    query: Extract<Query<(Entity, &LineGizmo, &GlobalTransform, Option<&RenderLayers>)>>,
) {
    use bevy_math::Affine3;
    use bevy_render::sync_world::{MainEntity, TemporaryRenderEntity};

    let mut values = Vec::with_capacity(*previous_len);
    for (entity, linegizmo, transform, render_layers) in &query {
        let joints_resolution = if let GizmoLineJoint::Round(resolution) = linegizmo.config.joints {
            resolution
        } else {
            0
        };

        values.push((
            LineGizmoUniform {
                world_from_local: Affine3::from(&transform.affine()).to_transpose(),
                line_width: linegizmo.config.width,
                depth_bias: linegizmo.depth_bias,
                joints_resolution,
                #[cfg(feature = "webgl")]
                _padding: Default::default(),
            },
            #[cfg(any(feature = "bevy_pbr", feature = "bevy_sprite"))]
            crate::config::GizmoMeshConfig {
                line_perspective: linegizmo.config.perspective,
                line_style: linegizmo.config.style,
                line_joints: linegizmo.config.joints,
                render_layers: render_layers.cloned().unwrap_or_default(),
                handle: linegizmo.handle.clone_weak(),
            },
            MainEntity::from(entity),
            TemporaryRenderEntity,
        ));
    }
    *previous_len = values.len();
    commands.spawn_batch(values);
}
