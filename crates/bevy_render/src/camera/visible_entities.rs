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
/// There are 32 groups numbered `0` - `31`. A mask may belong to one or more
/// groups, or no group at all.
///
/// An entity with a mask belonging to no groups is invisible.
///
/// The [`Default`] instance of `RenderingMask` returns a mask belonging to
/// group `0`, the first group.
#[derive(Copy, Clone, Reflect, PartialEq, Eq, PartialOrd, Ord)]
#[reflect(Component)]
pub struct RenderingMask(u32);

impl std::fmt::Debug for RenderingMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RenderingMask")
            .field(&self.get_groups())
            .finish()
    }
}

impl std::iter::FromIterator<u8> for RenderingMask {
    fn from_iter<T: IntoIterator<Item = u8>>(i: T) -> Self {
        i.into_iter()
            .fold(RenderingMask(0), |mask, g| mask.with_group(g))
    }
}

/// Defaults to a mask belonging to group `0`, the first group.
impl Default for RenderingMask {
    fn default() -> Self {
        RenderingMask(1)
    }
}

impl RenderingMask {
    /// Create a new `RenderingMask` belonging to the given rendering group.
    pub fn group(n: u8) -> Self {
        RenderingMask(0).with_group(n)
    }

    /// Create a new `RenderingMask` that belongs to all rendering groups.
    pub fn all_groups() -> Self {
        RenderingMask(u32::MAX)
    }

    /// Create a new `RenderingMask` that belongs to no rendering groups.
    pub fn no_groups() -> Self {
        RenderingMask(0)
    }

    /// Create a `RenderingMask` from a list of groups.
    pub fn from_groups(groups: &[u8]) -> Self {
        groups
            .iter()
            .fold(RenderingMask(0), |mask, g| mask.with_group(*g))
    }

    /// Add the given group to the mask.
    ///
    /// This may be called multiple times to allow an entity to belong
    /// to multiple rendering groups. The maximum group is 31.
    ///
    /// # Panics
    /// Panics when called with a group greater than 31.
    pub fn with_group(mut self, group: u8) -> Self {
        assert!(group < 32, "RenderingMask only supports groups 0 to 31");
        self.0 |= 1 << group;
        self
    }

    /// Removes the given rendering group from the mask.
    ///
    /// # Panics
    /// Panics when called with a group greater than 31.
    pub fn without_group(mut self, group: u8) -> Self {
        assert!(group < 32, "RenderingMask only supports groups 0 to 31");
        self.0 |= 0 << group;
        self
    }

    /// Get a vector of this mask's groups.
    pub fn get_groups(&self) -> Vec<u8> {
        (0..32)
            .filter(|g| RenderingMask::group(*g).matches(self))
            .collect::<Vec<u8>>()
    }

    /// Determine if a `RenderingMask` matches another.
    ///
    /// `RenderingMask`s match if the first mask contains any of the groups
    /// in the other.
    ///
    /// A `RenderingMask` belonging to no groups will not match any other
    /// mask, even another belonging to no groups.
    pub fn matches(&self, other: &RenderingMask) -> bool {
        (self.0 & other.0) > 0
    }
}

#[cfg(test)]
mod rendering_mask_tests {
    use super::RenderingMask;

    #[test]
    fn rendering_mask_sanity() {
        assert_eq!(RenderingMask::group(0).0, 1, "group 0 is mask 1");
        assert_eq!(RenderingMask::group(1).0, 2, "group 1 is mask 2");
        assert_eq!(
            RenderingMask::group(0).with_group(1).0,
            3,
            "group 0 + 1 is mask 3"
        );
        assert!(
            RenderingMask::group(1).matches(&RenderingMask::group(1)),
            "groups match like groups"
        );
        assert!(
            RenderingMask::group(0).matches(&RenderingMask(1)),
            "a group of 0 means the mask is just 1 bit"
        );

        assert!(
            RenderingMask::group(0)
                .with_group(3)
                .matches(&RenderingMask::group(3)),
            "a mask will match another mask containing any similar groups"
        );

        assert!(
            RenderingMask::default().matches(&RenderingMask::default()),
            "default masks match each other"
        );

        assert_eq!(
            RenderingMask::group(0).matches(&RenderingMask::group(1)),
            false,
            "masks with differing groups do not match"
        );
        assert_eq!(
            RenderingMask(0).matches(&RenderingMask(0)),
            false,
            "empty masks don't match"
        );
        assert_eq!(
            RenderingMask::from_groups(&[0, 2, 16, 30]).get_groups(),
            vec![0, 2, 16, 30],
            "from_groups and get_groups should roundtrip"
        );
        assert_eq!(
            format!("{:?}", RenderingMask::from_groups(&[0, 1, 2, 3])).as_str(),
            "RenderingMask([0, 1, 2, 3])",
            "Debug instance shows groups"
        );
        assert_eq!(
            RenderingMask::from_groups(&[0, 1, 2]),
            <RenderingMask as std::iter::FromIterator<u8>>::from_iter(vec![0, 1, 2]),
            "from_groups and from_iter are equivalent"
        )
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
        let camera_mask = maybe_camera_mask.copied().unwrap_or_default();

        let mut no_transform_order = 0.0;
        let mut transparent_entities = Vec::new();
        for (entity, visible, maybe_entity_mask) in visible_query.iter() {
            if !visible.is_visible {
                continue;
            }

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
