use super::{Camera, DepthCalculation};
use crate::Draw;
use bevy_core::FloatOrd;
use bevy_ecs::{Entity, Query};
use bevy_property::Properties;
use bevy_transform::prelude::GlobalTransform;

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
    mut draw_query: Query<(Entity, &Draw)>,
    draw_transform_query: Query<(&Draw, &GlobalTransform)>,
) {
    for (camera, camera_global_transform, mut visible_entities) in &mut camera_query.iter() {
        visible_entities.value.clear();
        let camera_position = camera_global_transform.translation();

        let mut no_transform_order = 0.0;
        let mut transparent_entities = Vec::new();
        for (entity, draw) in &mut draw_query.iter() {
            if !draw.is_visible {
                continue;
            }

            let order =
                if let Ok(global_transform) = draw_transform_query.get::<GlobalTransform>(entity) {
                    let position = global_transform.translation();
                    // smaller distances are sorted to lower indices by using the distance from the camera
                    FloatOrd(match camera.depth_calculation {
                        DepthCalculation::ZDifference => camera_position.z() - position.z(),
                        DepthCalculation::Distance => (camera_position - position).length(),
                    })
                } else {
                    let order = FloatOrd(no_transform_order);
                    no_transform_order += 0.1;
                    order
                };

            if draw.is_transparent {
                transparent_entities.push(VisibleEntity { entity, order })
            } else {
                visible_entities.value.push(VisibleEntity { entity, order })
            }
        }

        // sort opaque entities front-to-back
        visible_entities.value.sort_by_key(|e| e.order);

        // sort transparent entities front-to-back
        transparent_entities.sort_by_key(|e| -e.order);
        visible_entities.value.extend(transparent_entities);

        // TODO: check for big changes in visible entities len() vs capacity() (ex: 2x) and resize to prevent holding unneeded memory
    }
}
