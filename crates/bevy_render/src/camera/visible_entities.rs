use crate::{draw::Draw, Camera};
use bevy_core::float_ord::FloatOrd;
use bevy_transform::prelude::Transform;
use legion::{
    entity::Entity,
    prelude::{Read, Write},
    systems::{Query, SubWorld},
};

pub struct VisibleEntity {
    pub entity: Entity,
    pub order: FloatOrd,
}

#[derive(Default)]
pub struct VisibleEntities {
    pub value: Vec<VisibleEntity>,
}

impl VisibleEntities {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item=&VisibleEntity> {
        self.value.iter()
    }
}

pub fn visible_entities_system(
    world: &mut SubWorld,
    camera_query: &mut Query<(Read<Camera>, Read<Transform>, Write<VisibleEntities>)>,
    entities_query: &mut Query<Read<Draw>>,
    _transform_entities_query: &mut Query<(Read<Draw>, Read<Transform>)>, // ensures we can optionally access Transforms
) {
    for (_camera, camera_transform, mut visible_entities) in camera_query.iter_mut(world) {
        visible_entities.value.clear();
        let camera_position = camera_transform.value.w_axis().truncate();

        let mut no_transform_order = 0.0;
        for (entity, draw) in entities_query.iter_entities(world) {
            if !draw.is_visible {
                continue;
            }

            let order = if let Some(transform) = world.get_component::<Transform>(entity) {
                let position = transform.value.w_axis().truncate();
                // smaller distances are sorted to lower indices by using the negative distance from the camera 
                FloatOrd(-(camera_position - position).length())
            } else {
                let order = FloatOrd(no_transform_order);
                no_transform_order += 0.1;
                order
            };
            visible_entities.value.push(VisibleEntity {
                entity,
                order,
            })
        }

        visible_entities.value.sort_by_key(|e| e.order)

        // TODO: check for big changes in visible entities len() vs capacity() (ex: 2x) and resize to prevent holding unneeded memory
    }
}
