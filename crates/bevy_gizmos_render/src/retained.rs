//! This module is for 'retained' alternatives to the 'immediate mode' [`Gizmos`](bevy_gizmos::gizmos::Gizmos) system parameter.

use crate::LineGizmoUniform;
use bevy_camera::visibility::RenderLayers;
use bevy_gizmos::retained::Gizmo;
use bevy_math::Affine3;
use bevy_render::sync_world::{MainEntity, TemporaryRenderEntity};
use bevy_utils::once;
use tracing::warn;
use {
    bevy_ecs::{
        entity::Entity,
        system::{Commands, Local, Query},
    },
    bevy_gizmos::config::GizmoLineJoint,
    bevy_render::Extract,
    bevy_transform::components::GlobalTransform,
};

use bevy_gizmos::config::GizmoLineStyle;

pub(crate) fn extract_linegizmos(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(Entity, &Gizmo, &GlobalTransform, Option<&RenderLayers>)>>,
) {
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
            #[cfg(any(feature = "bevy_pbr", feature = "bevy_sprite_render"))]
            bevy_gizmos::config::GizmoMeshConfig {
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
