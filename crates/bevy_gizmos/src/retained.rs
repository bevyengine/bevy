use std::ops::{Deref, DerefMut};

use bevy_asset::{Asset, Handle};
use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    system::{Commands, Local, Query},
};
use bevy_math::Vec3;
use bevy_reflect::Reflect;
use bevy_render::{
    view::{InheritedVisibility, ViewVisibility, Visibility},
    Extract,
};
use bevy_transform::prelude::{GlobalTransform, Transform};

use crate::{
    config::{self, DefaultGizmoConfigGroup, GizmoConfig, GizmoLineJoint},
    gizmos::GizmoBuffer,
    LineGizmoUniform,
};

#[derive(Debug, Default, Asset, Clone, Reflect)]
pub struct Polyline {
    buffer: GizmoBuffer<DefaultGizmoConfigGroup, ()>,
}

impl Deref for Polyline {
    type Target = GizmoBuffer<DefaultGizmoConfigGroup, ()>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for Polyline {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

#[derive(Bundle, Default)]
pub struct PolylineBundle {
    pub polyline: Handle<Polyline>,
    pub config: GizmoConfig,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

pub fn extract_polylines(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<
        Query<(
            Entity,
            &InheritedVisibility,
            &ViewVisibility,
            &GlobalTransform,
            &Handle<Polyline>,
            &GizmoConfig,
        )>,
    >,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, inherited_visibility, view_visibility, transform, handle, config) in &query {
        if !inherited_visibility.get() || !view_visibility.get() {
            continue;
        }

        let joints_resolution = if let GizmoLineJoint::Round(resolution) = config.line_joints {
            resolution
        } else {
            0
        };

        // TODO Add transform to LineGizmoUniform
        // let transform = transform.compute_matrix();
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
