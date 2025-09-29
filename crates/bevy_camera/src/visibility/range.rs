//! Specific distances from the camera in which entities are visible, also known
//! as *hierarchical levels of detail* or *HLOD*s.

use core::{
    hash::{Hash, Hasher},
    ops::Range,
};

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashMap},
    query::With,
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Local, Query, ResMut},
};
use bevy_math::FloatOrd;
use bevy_reflect::Reflect;
use bevy_transform::components::GlobalTransform;
use bevy_utils::Parallel;

use super::{check_visibility, VisibilitySystems};
use crate::{camera::Camera, primitives::Aabb};

/// A plugin that enables [`VisibilityRange`]s, which allow entities to be
/// hidden or shown based on distance to the camera.
pub struct VisibilityRangePlugin;

impl Plugin for VisibilityRangePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VisibleEntityRanges>().add_systems(
            PostUpdate,
            check_visibility_ranges
                .in_set(VisibilitySystems::CheckVisibility)
                .before(check_visibility),
        );
    }
}

/// Specifies the range of distances that this entity must be from the camera in
/// order to be rendered.
///
/// This is also known as *hierarchical level of detail* or *HLOD*.
///
/// Use this component when you want to render a high-polygon mesh when the
/// camera is close and a lower-polygon mesh when the camera is far away. This
/// is a common technique for improving performance, because fine details are
/// hard to see in a mesh at a distance. To avoid an artifact known as *popping*
/// between levels, each level has a *margin*, within which the object
/// transitions gradually from invisible to visible using a dithering effect.
///
/// You can also use this feature to replace multiple meshes with a single mesh
/// when the camera is distant. This is the reason for the term "*hierarchical*
/// level of detail". Reducing the number of meshes can be useful for reducing
/// drawcall count. Note that you must place the [`VisibilityRange`] component
/// on each entity you want to be part of a LOD group, as [`VisibilityRange`]
/// isn't automatically propagated down to children.
///
/// A typical use of this feature might look like this:
///
/// | Entity                  | `start_margin` | `end_margin` |
/// |-------------------------|----------------|--------------|
/// | Root                    | N/A            | N/A          |
/// | ├─ High-poly mesh       | [0, 0)         | [20, 25)     |
/// | ├─ Low-poly mesh        | [20, 25)       | [70, 75)     |
/// | └─ Billboard *imposter* | [70, 75)       | [150, 160)   |
///
/// With this setup, the user will see a high-poly mesh when the camera is
/// closer than 20 units. As the camera zooms out, between 20 units to 25 units,
/// the high-poly mesh will gradually fade to a low-poly mesh. When the camera
/// is 70 to 75 units away, the low-poly mesh will fade to a single textured
/// quad. And between 150 and 160 units, the object fades away entirely. Note
/// that the `end_margin` of a higher LOD is always identical to the
/// `start_margin` of the next lower LOD; this is important for the crossfade
/// effect to function properly.
#[derive(Component, Clone, PartialEq, Default, Reflect)]
#[reflect(Component, PartialEq, Hash, Clone)]
pub struct VisibilityRange {
    /// The range of distances, in world units, between which this entity will
    /// smoothly fade into view as the camera zooms out.
    ///
    /// If the start and end of this range are identical, the transition will be
    /// abrupt, with no crossfading.
    ///
    /// `start_margin.end` must be less than or equal to `end_margin.start`.
    pub start_margin: Range<f32>,

    /// The range of distances, in world units, between which this entity will
    /// smoothly fade out of view as the camera zooms out.
    ///
    /// If the start and end of this range are identical, the transition will be
    /// abrupt, with no crossfading.
    ///
    /// `end_margin.start` must be greater than or equal to `start_margin.end`.
    pub end_margin: Range<f32>,

    /// If set to true, Bevy will use the center of the axis-aligned bounding
    /// box ([`Aabb`]) as the position of the mesh for the purposes of
    /// visibility range computation.
    ///
    /// Otherwise, if this field is set to false, Bevy will use the origin of
    /// the mesh as the mesh's position.
    ///
    /// Usually you will want to leave this set to false, because different LODs
    /// may have different AABBs, and smooth crossfades between LOD levels
    /// require that all LODs of a mesh be at *precisely* the same position. If
    /// you aren't using crossfading, however, and your meshes aren't centered
    /// around their origins, then this flag may be useful.
    pub use_aabb: bool,
}

impl Eq for VisibilityRange {}

impl Hash for VisibilityRange {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        FloatOrd(self.start_margin.start).hash(state);
        FloatOrd(self.start_margin.end).hash(state);
        FloatOrd(self.end_margin.start).hash(state);
        FloatOrd(self.end_margin.end).hash(state);
    }
}

impl VisibilityRange {
    /// Creates a new *abrupt* visibility range, with no crossfade.
    ///
    /// There will be no crossfade; the object will immediately vanish if the
    /// camera is closer than `start` units or farther than `end` units from the
    /// model.
    ///
    /// The `start` value must be less than or equal to the `end` value.
    #[inline]
    pub fn abrupt(start: f32, end: f32) -> Self {
        Self {
            start_margin: start..start,
            end_margin: end..end,
            use_aabb: false,
        }
    }

    /// Returns true if both the start and end transitions for this range are
    /// abrupt: that is, there is no crossfading.
    #[inline]
    pub fn is_abrupt(&self) -> bool {
        self.start_margin.start == self.start_margin.end
            && self.end_margin.start == self.end_margin.end
    }

    /// Returns true if the object will be visible at all, given a camera
    /// `camera_distance` units away.
    ///
    /// Any amount of visibility, even with the heaviest dithering applied, is
    /// considered visible according to this check.
    #[inline]
    pub fn is_visible_at_all(&self, camera_distance: f32) -> bool {
        camera_distance >= self.start_margin.start && camera_distance < self.end_margin.end
    }

    /// Returns true if the object is completely invisible, given a camera
    /// `camera_distance` units away.
    ///
    /// This is equivalent to `!VisibilityRange::is_visible_at_all()`.
    #[inline]
    pub fn is_culled(&self, camera_distance: f32) -> bool {
        !self.is_visible_at_all(camera_distance)
    }
}

/// Stores which entities are in within the [`VisibilityRange`]s of views.
///
/// This doesn't store the results of frustum or occlusion culling; use
/// [`ViewVisibility`](`super::ViewVisibility`) for that. Thus entities in this list may not
/// actually be visible.
///
/// For efficiency, these tables only store entities that have
/// [`VisibilityRange`] components. Entities without such a component won't be
/// in these tables at all.
///
/// The table is indexed by entity and stores a 32-bit bitmask with one bit for
/// each camera, where a 0 bit corresponds to "out of range" and a 1 bit
/// corresponds to "in range". Hence it's limited to storing information for 32
/// views.
#[derive(Resource, Default)]
pub struct VisibleEntityRanges {
    /// Stores which bit index each view corresponds to.
    views: EntityHashMap<u8>,

    /// Stores a bitmask in which each view has a single bit.
    ///
    /// A 0 bit for a view corresponds to "out of range"; a 1 bit corresponds to
    /// "in range".
    entities: EntityHashMap<u32>,
}

impl VisibleEntityRanges {
    /// Clears out the [`VisibleEntityRanges`] in preparation for a new frame.
    fn clear(&mut self) {
        self.views.clear();
        self.entities.clear();
    }

    /// Returns true if the entity is in range of the given camera.
    ///
    /// This only checks [`VisibilityRange`]s and doesn't perform any frustum or
    /// occlusion culling. Thus the entity might not *actually* be visible.
    ///
    /// The entity is assumed to have a [`VisibilityRange`] component. If the
    /// entity doesn't have that component, this method will return false.
    #[inline]
    pub fn entity_is_in_range_of_view(&self, entity: Entity, view: Entity) -> bool {
        let Some(visibility_bitmask) = self.entities.get(&entity) else {
            return false;
        };
        let Some(view_index) = self.views.get(&view) else {
            return false;
        };
        (visibility_bitmask & (1 << view_index)) != 0
    }

    /// Returns true if the entity is in range of any view.
    ///
    /// This only checks [`VisibilityRange`]s and doesn't perform any frustum or
    /// occlusion culling. Thus the entity might not *actually* be visible.
    ///
    /// The entity is assumed to have a [`VisibilityRange`] component. If the
    /// entity doesn't have that component, this method will return false.
    #[inline]
    pub fn entity_is_in_range_of_any_view(&self, entity: Entity) -> bool {
        self.entities.contains_key(&entity)
    }
}

/// Checks all entities against all views in order to determine which entities
/// with [`VisibilityRange`]s are potentially visible.
///
/// This only checks distance from the camera and doesn't frustum or occlusion
/// cull.
pub fn check_visibility_ranges(
    mut visible_entity_ranges: ResMut<VisibleEntityRanges>,
    view_query: Query<(Entity, &GlobalTransform), With<Camera>>,
    mut par_local: Local<Parallel<Vec<(Entity, u32)>>>,
    entity_query: Query<(Entity, &GlobalTransform, Option<&Aabb>, &VisibilityRange)>,
) {
    visible_entity_ranges.clear();

    // Early out if the visibility range feature isn't in use.
    if entity_query.is_empty() {
        return;
    }

    // Assign an index to each view.
    let mut views = vec![];
    for (view, view_transform) in view_query.iter().take(32) {
        let view_index = views.len() as u8;
        visible_entity_ranges.views.insert(view, view_index);
        views.push((view, view_transform.translation_vec3a()));
    }

    // Check each entity/view pair. Only consider entities with
    // [`VisibilityRange`] components.
    entity_query.par_iter().for_each(
        |(entity, entity_transform, maybe_model_aabb, visibility_range)| {
            let mut visibility = 0;
            for (view_index, &(_, view_position)) in views.iter().enumerate() {
                // If instructed to use the AABB and the model has one, use its
                // center as the model position. Otherwise, use the model's
                // translation.
                let model_position = match (visibility_range.use_aabb, maybe_model_aabb) {
                    (true, Some(model_aabb)) => entity_transform
                        .affine()
                        .transform_point3a(model_aabb.center),
                    _ => entity_transform.translation_vec3a(),
                };

                if visibility_range.is_visible_at_all((view_position - model_position).length()) {
                    visibility |= 1 << view_index;
                }
            }

            // Invisible entities have no entry at all in the hash map. This speeds
            // up checks slightly in this common case.
            if visibility != 0 {
                par_local.borrow_local_mut().push((entity, visibility));
            }
        },
    );

    visible_entity_ranges.entities.extend(par_local.drain());
}
