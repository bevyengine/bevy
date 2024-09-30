//! This module is for 'retained' alternatives to the 'immediate mode' [`Gizmos`](crate::gizmos::Gizmos) system parameter.

use core::ops::{Deref, DerefMut};

use bevy_asset::Handle;
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;

#[cfg(feature = "bevy_render")]
use {
    crate::{config::GizmoLineJoint, LineGizmoUniform},
    bevy_ecs::system::{Commands, Local, Query},
    bevy_render::{
        world_sync::{RenderEntity, SyncToRenderWorld, TemporaryRenderEntity},
        Extract,
    },
};

use crate::{
    config::{ErasedGizmoConfigGroup, GizmoConfig},
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
///         config: GizmoConfig {
///             line_width: 3.,
///             ..default()
///         },
///     });
/// }
/// ```
///
/// [`Gizmos`]: crate::gizmos::Gizmos
#[derive(Component, Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "bevy_render", require(SyncToRenderWorld))]
pub struct LineGizmo {
    /// The handle to the line to draw.
    pub handle: Handle<LineGizmoAsset>,
    /// The configuration for this gizmo.
    pub config: GizmoConfig,
}

#[cfg(feature = "bevy_render")]
pub(crate) fn extract_linegizmos(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(&RenderEntity, &LineGizmo)>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (render_entity, linegizmo) in &query {
        if !linegizmo.config.enabled {
            continue;
        }
        let joints_resolution =
            if let GizmoLineJoint::Round(resolution) = linegizmo.config.line_joints {
                resolution
            } else {
                0
            };

        // TODO Add transform to LineGizmoUniform
        values.push((
            render_entity.id(),
            (
                LineGizmoUniform {
                    line_width: linegizmo.config.line_width,
                    depth_bias: linegizmo.config.depth_bias,
                    joints_resolution,
                    #[cfg(feature = "webgl")]
                    _padding: Default::default(),
                },
                linegizmo.handle.clone_weak(),
                #[cfg(any(feature = "bevy_pbr", feature = "bevy_sprite"))]
                crate::config::GizmoMeshConfig::from(&linegizmo.config),
                TemporaryRenderEntity,
            ),
        ));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}
