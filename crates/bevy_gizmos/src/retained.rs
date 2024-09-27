use core::ops::{Deref, DerefMut};

use bevy_asset::Handle;
use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    system::{Commands, Local, Query},
};
use bevy_render::Extract;

use crate::{
    config::{self, GizmoConfig, GizmoLineJoint, NoGizmoConfigGroup},
    gizmos::GizmoBuffer,
    LineGizmo, LineGizmoUniform,
};

impl Deref for LineGizmo {
    type Target = GizmoBuffer<NoGizmoConfigGroup, ()>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for LineGizmo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

#[derive(Bundle, Default)]
pub struct LineGizmoBundle {
    pub linegizmo: Handle<LineGizmo>,
    pub config: GizmoConfig,
}

pub fn extract_linegizmos(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(Entity, &Handle<LineGizmo>, &GizmoConfig)>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, handle, config) in &query {
        let joints_resolution = if let GizmoLineJoint::Round(resolution) = config.line_joints {
            resolution
        } else {
            0
        };

        // TODO Add transform to LineGizmoUniform
        values.push((
            entity,
            (
                LineGizmoUniform {
                    line_width: config.line_width,
                    depth_bias: config.depth_bias,
                    joints_resolution,
                    #[cfg(feature = "webgl")]
                    _padding: Default::default(),
                },
                (*handle).clone_weak(),
                #[cfg(any(feature = "bevy_pbr", feature = "bevy_sprite"))]
                config::GizmoMeshConfig::from(config),
            ),
        ));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}
