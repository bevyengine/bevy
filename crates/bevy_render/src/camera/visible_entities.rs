use super::{Camera, DepthCalculation};
use crate::Draw;
use bevy_core::FloatOrd;
use bevy_ecs::{Entity, Query, With, Without};
use bevy_math::Vec3;
use bevy_property::Properties;
use bevy_transform::prelude::{Children, GlobalTransform, Parent};

#[derive(Debug)]
pub struct VisibleEntity {
    pub entity: Entity,
    pub order: FloatOrd,
}

#[derive(Default, Debug, Properties)]
pub struct VisibleEntities {
    #[property(ignore)]
    pub value: Vec<VisibleEntity>,
}

impl VisibleEntities {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &VisibleEntity> {
        self.value.iter()
    }
}

pub fn visible_entities_system(
    mut camera_query: Query<(&Camera, &GlobalTransform, &mut VisibleEntities)>,
    draw_orphan_query: Query<Entity, (Without<Parent>, With<Draw>)>,
    draw_query: Query<(Entity, &Draw, Option<&Children>, Option<&GlobalTransform>)>,
) {
    for (camera, camera_global_transform, mut visible_entities) in camera_query.iter_mut() {
        visible_entities.value.clear();
        let camera_position = camera_global_transform.translation;

        let mut no_transform_order = 0.0;
        let mut transparent_entities = Vec::new();
        for entity in draw_orphan_query.iter() {
            recursive_draw_check(
                &draw_query,
                entity,
                camera,
                camera_position,
                &mut no_transform_order,
                &mut transparent_entities,
                &mut visible_entities,
            );
        }

        // sort opaque entities front-to-back
        visible_entities.value.sort_by_key(|e| e.order);

        // sort transparent entities front-to-back
        transparent_entities.sort_by_key(|e| -e.order);
        visible_entities.value.extend(transparent_entities);

        // TODO: check for big changes in visible entities len() vs capacity() (ex: 2x) and resize to prevent holding unneeded memory
    }
}

/// Checks if an object is visible, and recursively checks the object's children.
fn recursive_draw_check(
    draw_query: &Query<(Entity, &Draw, Option<&Children>, Option<&GlobalTransform>)>,
    entity: Entity,
    camera: &Camera,
    camera_position: Vec3,
    no_transform_order: &mut f32,
    transparent_entities: &mut Vec<VisibleEntity>,
    visible_entities: &mut VisibleEntities,
) {
    let (entity, draw, children, global_transform) = if let Ok(result) = draw_query.get(entity) {
        result
    } else {
        return;
    };

    if !draw.is_visible {
        return;
    }

    let order = if let Some(global_transform) = global_transform {
        let position = global_transform.translation;
        // smaller distances are sorted to lower indices by using the distance from the camera
        FloatOrd(match camera.depth_calculation {
            DepthCalculation::ZDifference => camera_position.z() - position.z(),
            DepthCalculation::Distance => (camera_position - position).length(),
        })
    } else {
        let order = FloatOrd(*no_transform_order);
        *no_transform_order += 0.1;
        order
    };

    if draw.is_transparent {
        transparent_entities.push(VisibleEntity { entity, order })
    } else {
        visible_entities.value.push(VisibleEntity { entity, order })
    }

    if let Some(children) = children {
        for child in children.iter() {
            recursive_draw_check(
                draw_query,
                *child,
                camera,
                camera_position,
                no_transform_order,
                transparent_entities,
                visible_entities,
            )
        }
    }
}
