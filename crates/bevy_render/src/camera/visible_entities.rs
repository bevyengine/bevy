use crate::{draw::Draw, Camera};
use bevy_core::float_ord::FloatOrd;
use bevy_transform::prelude::Transform;
use bevy_ecs::{Query, Entity};

#[derive(Debug)]
pub struct VisibleEntity {
    pub entity: Entity,
    pub order: FloatOrd,
}

#[derive(Default, Debug)]
pub struct VisibleEntities {
    pub value: Vec<VisibleEntity>,
}

impl VisibleEntities {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item=&VisibleEntity> {
        self.value.iter()
    }
}

pub fn visible_entities_system(
    mut camera_query: Query<(Entity, &Camera, &mut VisibleEntities)>,
    mut entities_query: Query<(Entity, &Draw)>,
    transform_query: Query<&Transform>,
    _transform_entities_query: Query<(&Draw, &Transform)>, // ensures we can optionally access Transforms
) {
    for (camera_entity, _camera, visible_entities) in &mut camera_query.iter() {
        visible_entities.value.clear();
        let camera_transform = transform_query.get::<Transform>(camera_entity).unwrap();
        let camera_position = camera_transform.value.w_axis().truncate();

        let mut no_transform_order = 0.0;
        let mut transparent_entities = Vec::new();
        for (entity, draw) in &mut entities_query.iter() {
            if !draw.is_visible {
                continue;
            }

            let order = if let Ok(transform) = transform_query.get::<Transform>(entity) {
                let position = transform.value.w_axis().truncate();
                // smaller distances are sorted to lower indices by using the distance from the camera 
                FloatOrd((camera_position - position).length())
            } else {
                let order = FloatOrd(no_transform_order);
                no_transform_order += 0.1;
                order
            };

            if draw.is_transparent {
                transparent_entities.push(VisibleEntity {
                    entity,
                    order,
                })
            } else {
                visible_entities.value.push(VisibleEntity {
                    entity,
                    order,
                })
            }
        }


        // sort opaque entities front-to-back
        visible_entities.value.sort_by_key(|e| e.order);

        // sort transparent entities front-to-back
        transparent_entities.sort_by_key(|e|-e.order);
        visible_entities.value.extend(transparent_entities);

        // TODO: check for big changes in visible entities len() vs capacity() (ex: 2x) and resize to prevent holding unneeded memory
    }
}
