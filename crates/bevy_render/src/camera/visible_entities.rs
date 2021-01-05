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

/// A mask describes which rendering group an entity belongs to.
///
/// Cameras with this component will only render entities with a matching
/// mask.
///
/// The [`Default`] instance of `RenderingMask` returns a mask that contains
/// no groups.
#[derive(Copy, Clone, Debug, Reflect, PartialEq, Eq, PartialOrd, Ord)]
#[reflect(Component)]
pub struct RenderingMask(pub u32);

impl Default for RenderingMask {
    fn default() -> Self {
        RenderingMask(0)
    }
}

impl RenderingMask {
    /// Create new `RenderingMask` with the given rendering group set.
    pub fn group(n: u8) -> Self {
        RenderingMask::default().set_group(n)
    }

    /// Set the given group on the mask.
    /// This may be called multiple times to allow an entity to belong
    /// to multiple rendering groups.
    pub fn set_group(mut self, group: u8) -> Self {
        self.0 |= 1 << group;
        self
    }

    /// Unset the given rendering group from the mask.
    pub fn unset_group(mut self, group: u8) -> Self {
        self.0 |= 0 << group;
        self
    }

    /// Determine if a `RenderingMask` matches another.
    /// `RenderingMask`s match if the first mask contains any of the groups
    /// in the other, or if both masks contain no groups.
    pub fn matches(&self, other: &RenderingMask) -> bool {
        ((self.0 & other.0) > 0) || (self.0 == 0 && other.0 == 0)
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
        assert!(RenderingMask::group(0)
            .set_group(3)
            .matches(&RenderingMask::group(3)));
        // default masks match each other
        assert!(RenderingMask::default().matches(&RenderingMask::default()));
        // masks with differing groups do not match
        assert_eq!(
            RenderingMask::group(0).matches(&RenderingMask::group(1)),
            false
        );
    }
}

pub fn visible_entities_system(
    mut camera_query: Query<(
        &Camera,
        &GlobalTransform,
        &mut VisibleEntities,
        Option<&RenderingMask>,
    )>,
    visible_query: Query<(Entity, &Visible, Option<&RenderingMask>)>,
    visible_transform_query: Query<&GlobalTransform, With<Visible>>,
) {
    for (camera, camera_global_transform, mut visible_entities, maybe_camera_mask) in
        camera_query.iter_mut()
    {
        visible_entities.value.clear();
        let camera_position = camera_global_transform.translation;

        let mut no_transform_order = 0.0;
        let mut transparent_entities = Vec::new();
        for (entity, visible, maybe_entity_mask) in visible_query.iter() {
            if !visible.is_visible {
                continue;
            }

            let camera_mask = maybe_camera_mask.copied().unwrap_or_default();
            let entity_mask = maybe_entity_mask.copied().unwrap_or_default();
            if !camera_mask.matches(&entity_mask) {
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
