use super::{Camera, DepthCalculation};
use crate::Draw;
use bevy_core::FloatOrd;
use bevy_ecs::{Entity, Query, QuerySet, With, Without};
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
    draw_queries: QuerySet<(
        Query<(Entity, Option<&Children>, &Draw), Without<Parent>>,
        Query<(Entity, Option<&Children>, &Draw)>,
        Query<&GlobalTransform, With<Draw>>,
    )>,
) {
    for (camera, camera_global_transform, mut visible_entities) in camera_query.iter_mut() {
        visible_entities.value.clear();
        let camera_position = camera_global_transform.translation;

        let mut no_transform_order = 0.0;
        let mut transparent_entities = Vec::new();
        for (entity, children, draw) in draw_queries.q0().iter() {
            if !draw.is_visible {
                continue;
            }

            if let Some(children) = children {
                recursive_draw_check(
                    &draw_queries,
                    children,
                    camera,
                    camera_position,
                    &mut no_transform_order,
                    &mut transparent_entities,
                    &mut visible_entities,
                )
            }

            process_visible(
                entity,
                camera,
                camera_position,
                &draw_queries,
                &mut no_transform_order,
                &mut transparent_entities,
                &mut visible_entities,
                draw,
            )
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
    draw_queries: &QuerySet<(
        Query<(Entity, Option<&Children>, &Draw), Without<Parent>>,
        Query<(Entity, Option<&Children>, &Draw)>,
        Query<&GlobalTransform, With<Draw>>,
    )>,
    children: &Children,
    camera: &Camera,
    camera_position: Vec3,
    no_transform_order: &mut f32,
    transparent_entities: &mut Vec<VisibleEntity>,
    visible_entities: &mut VisibleEntities,
) {
    for child in children.0.iter() {
        draw_queries.q1().get(*child).unwrap();

        if let Ok((entity, children, draw)) = draw_queries.q1().get(*child) {
            if !draw.is_visible {
                continue;
            }

            if let Some(children) = children {
                recursive_draw_check(
                    draw_queries,
                    children,
                    camera,
                    camera_position,
                    no_transform_order,
                    transparent_entities,
                    visible_entities,
                )
            }

            process_visible(
                entity,
                camera,
                camera_position,
                draw_queries,
                no_transform_order,
                transparent_entities,
                visible_entities,
                draw,
            )
        }
    }
}

/// Processes a visible entity
#[allow(clippy::too_many_arguments)]
fn process_visible(
    entity: Entity,
    camera: &Camera,
    camera_position: Vec3,
    draw_queries: &QuerySet<(
        Query<(Entity, Option<&Children>, &Draw), Without<Parent>>,
        Query<(Entity, Option<&Children>, &Draw)>,
        Query<&GlobalTransform, With<Draw>>,
    )>,
    no_transform_order: &mut f32,
    transparent_entities: &mut Vec<VisibleEntity>,
    visible_entities: &mut VisibleEntities,
    draw: &Draw,
) {
    let order = if let Ok(global_transform) = draw_queries.q2().get(entity) {
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
}
