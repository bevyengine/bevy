use super::{Camera, DepthCalculation};
use crate::prelude::Visible;
use bevy_core::FloatOrd;
use bevy_ecs::{Entity, Query, With};
use bevy_reflect::{Reflect, ReflectComponent};
use bevy_transform::prelude::GlobalTransform;

#[derive(Debug)]
pub struct VisibleEntity {
    pub entity: Entity,
    pub order: FloatOrd,
}

#[derive(Default, Debug, Reflect)]
#[reflect(Component)]
pub struct VisibleEntities {
    #[reflect(ignore)]
    pub value: Vec<VisibleEntity>,
}

impl VisibleEntities {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &VisibleEntity> {
        self.value.iter()
    }
}

/// A mask that describes which rendering group an entity belongs to.
/// Cameras with this component will only render entities with a matching
/// mask. In other words, only an entity with a matching camera will be
/// rendered.
#[derive(Debug, Reflect, PartialEq, Eq, PartialOrd, Ord)]
#[reflect(Component)]
pub struct RenderingMask(pub u8);

impl Default for RenderingMask {
    fn default() -> Self {
        RenderingMask( 0 )
    }
}

impl RenderingMask {
    pub fn group(n: u8) -> Self {
        RenderingMask::default().with_group(n)
    }

    pub fn with_group(mut self, group: u8) -> Self {
        self.0 = self.0 | (1 << group);
        self
    }

    pub fn without_group(mut self, group: u8) -> Self {
        self.0 = self.0 | (0 << group);
        self
    }

    pub fn matches(&self, other: &RenderingMask) -> bool {
        (self.0 & other.0) > 0
    }
}


#[cfg(test)]
mod rendering_mask_tests {
    use super::RenderingMask;

    #[test]
    fn rendering_mask_sanity() {
        // groups match groups
        assert!(RenderingMask::group(1).matches(&RenderingMask::group(1)));
        // a group of 0 means the mask is just 1 bit
        assert!(RenderingMask::group(0).matches(&RenderingMask(1)));
        // a mask will match another mask containing any similar groups
        assert!(RenderingMask::group(0).with_group(3).matches(&RenderingMask::group(3)));
    }
}

pub fn visible_entities_system(
    mut camera_query: Query<(&Camera, &GlobalTransform, &mut VisibleEntities, Option<&RenderingMask>)>,
    visible_query: Query<(Entity, &Visible, Option<&RenderingMask>)>,
    visible_transform_query: Query<&GlobalTransform, With<Visible>>,
) {
    for (camera, camera_global_transform, mut visible_entities, maybe_camera_mask) in camera_query.iter_mut() {
        visible_entities.value.clear();
        let camera_position = camera_global_transform.translation;

        let mut no_transform_order = 0.0;
        let mut transparent_entities = Vec::new();
        for (entity, visible, maybe_ent_mask) in visible_query.iter() {
            if !visible.is_visible {
                continue;
            }

            if let Some(camera_mask) = maybe_camera_mask {
                if let Some(entity_mask) = maybe_ent_mask {
                    if !camera_mask.matches(entity_mask) {
                        continue;
                    }
                } else {
                    continue;
                }
            } else if maybe_ent_mask.is_some() {
                continue;
            }

            let order = if let Ok(global_transform) = visible_transform_query.get(entity) {
                let position = global_transform.translation;
                // smaller distances are sorted to lower indices by using the distance from the camera
                FloatOrd(match camera.depth_calculation {
                    DepthCalculation::ZDifference => camera_position.z - position.z,
                    DepthCalculation::Distance => (camera_position - position).length(),
                })
            } else {
                let order = FloatOrd(no_transform_order);
                no_transform_order += 0.1;
                order
            };

            if visible.is_transparent {
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
