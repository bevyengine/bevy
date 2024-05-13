//! Specific distances from the camera in which entities are visible, also known
//! as *hierarchical levels of detail* or *HLOD*s.

use std::{
    hash::{Hash, Hasher},
    ops::Range,
};

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Changed, With},
    schedule::IntoSystemConfigs as _,
    system::{Query, Res, ResMut, Resource},
};
use bevy_math::{vec4, FloatOrd, Vec4};
use bevy_reflect::Reflect;
use bevy_transform::components::GlobalTransform;
use bevy_utils::{prelude::default, EntityHashMap, HashMap};
use nonmax::NonMaxU16;
use wgpu::{BufferBindingType, BufferUsages};

use crate::{
    camera::Camera,
    render_resource::BufferVec,
    renderer::{RenderDevice, RenderQueue},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};

use super::{check_visibility, VisibilitySystems, WithMesh};

/// We need at least 4 storage buffer bindings available to enable the
/// visibility range buffer.
///
/// Even though we only use one storage buffer, the first 3 available storage
/// buffers will go to various light-related buffers. We will grab the fourth
/// buffer slot.
pub const VISIBILITY_RANGES_STORAGE_BUFFER_COUNT: u32 = 4;

/// The size of the visibility ranges buffer in elements (not bytes) when fewer
/// than 6 storage buffers are available and we're forced to use a uniform
/// buffer instead (most notably, on WebGL 2).
const VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE: usize = 64;

/// A plugin that enables [`VisibilityRange`]s, which allow entities to be
/// hidden or shown based on distance to the camera.
pub struct VisibilityRangePlugin;

impl Plugin for VisibilityRangePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<VisibilityRange>()
            .init_resource::<VisibleEntityRanges>()
            .add_systems(
                PostUpdate,
                check_visibility_ranges
                    .in_set(VisibilitySystems::CheckVisibility)
                    .before(check_visibility::<WithMesh>),
            );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<RenderVisibilityRanges>()
            .add_systems(ExtractSchedule, extract_visibility_ranges)
            .add_systems(
                Render,
                write_render_visibility_ranges.in_set(RenderSet::PrepareResourcesFlush),
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
#[derive(Component, Clone, PartialEq, Reflect)]
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

/// Stores information related to [`VisibilityRange`]s in the render world.
#[derive(Resource)]
pub struct RenderVisibilityRanges {
    /// Information corresponding to each entity.
    entities: EntityHashMap<Entity, RenderVisibilityEntityInfo>,

    /// Maps a [`VisibilityRange`] to its index within the `buffer`.
    ///
    /// This map allows us to deduplicate identical visibility ranges, which
    /// saves GPU memory.
    range_to_index: HashMap<VisibilityRange, NonMaxU16>,

    /// The GPU buffer that stores [`VisibilityRange`]s.
    ///
    /// Each [`Vec4`] contains the start margin start, start margin end, end
    /// margin start, and end margin end distances, in that order.
    buffer: BufferVec<Vec4>,

    /// True if the buffer has been changed since the last frame and needs to be
    /// reuploaded to the GPU.
    buffer_dirty: bool,
}

/// Per-entity information related to [`VisibilityRange`]s.
struct RenderVisibilityEntityInfo {
    /// The index of the range within the GPU buffer.
    buffer_index: NonMaxU16,
    /// True if the range is abrupt: i.e. has no crossfade.
    is_abrupt: bool,
}

impl Default for RenderVisibilityRanges {
    fn default() -> Self {
        Self {
            entities: default(),
            range_to_index: default(),
            buffer: BufferVec::new(
                BufferUsages::STORAGE | BufferUsages::UNIFORM | BufferUsages::VERTEX,
            ),
            buffer_dirty: true,
        }
    }
}

impl RenderVisibilityRanges {
    /// Clears out the [`RenderVisibilityRanges`] in preparation for a new
    /// frame.
    fn clear(&mut self) {
        self.entities.clear();
        self.range_to_index.clear();
        self.buffer.clear();
        self.buffer_dirty = true;
    }

    /// Inserts a new entity into the [`RenderVisibilityRanges`].
    fn insert(&mut self, entity: Entity, visibility_range: &VisibilityRange) {
        // Grab a slot in the GPU buffer, or take the existing one if there
        // already is one.
        let buffer_index = *self
            .range_to_index
            .entry(visibility_range.clone())
            .or_insert_with(|| {
                NonMaxU16::try_from(self.buffer.push(vec4(
                    visibility_range.start_margin.start,
                    visibility_range.start_margin.end,
                    visibility_range.end_margin.start,
                    visibility_range.end_margin.end,
                )) as u16)
                .unwrap_or_default()
            });

        self.entities.insert(
            entity,
            RenderVisibilityEntityInfo {
                buffer_index,
                is_abrupt: visibility_range.is_abrupt(),
            },
        );
    }

    /// Returns the index in the GPU buffer corresponding to the visible range
    /// for the given entity.
    ///
    /// If the entity has no visible range, returns `None`.
    #[inline]
    pub fn lod_index_for_entity(&self, entity: Entity) -> Option<NonMaxU16> {
        self.entities.get(&entity).map(|info| info.buffer_index)
    }

    /// Returns true if the entity has a visibility range and it isn't abrupt:
    /// i.e. if it has a crossfade.
    #[inline]
    pub fn entity_has_crossfading_visibility_ranges(&self, entity: Entity) -> bool {
        self.entities
            .get(&entity)
            .is_some_and(|info| !info.is_abrupt)
    }

    /// Returns a reference to the GPU buffer that stores visibility ranges.
    #[inline]
    pub fn buffer(&self) -> &BufferVec<Vec4> {
        &self.buffer
    }
}

/// Stores which entities are in within the [`VisibilityRange`]s of views.
///
/// This doesn't store the results of frustum or occlusion culling; use
/// [`super::ViewVisibility`] for that. Thus entities in this list may not
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
    views: EntityHashMap<Entity, u8>,

    /// Stores a bitmask in which each view has a single bit.
    ///
    /// A 0 bit for a view corresponds to "out of range"; a 1 bit corresponds to
    /// "in range".
    entities: EntityHashMap<Entity, u32>,
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
    mut entity_query: Query<(Entity, &GlobalTransform, &VisibilityRange)>,
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
    for (entity, entity_transform, visibility_range) in entity_query.iter_mut() {
        let mut visibility = 0;
        for (view_index, &(_, view_position)) in views.iter().enumerate() {
            if visibility_range
                .is_visible_at_all((view_position - entity_transform.translation_vec3a()).length())
            {
                visibility |= 1 << view_index;
            }
        }

        // Invisible entities have no entry at all in the hash map. This speeds
        // up checks slightly in this common case.
        if visibility != 0 {
            visible_entity_ranges.entities.insert(entity, visibility);
        }
    }
}

/// Extracts all [`VisibilityRange`] components from the main world to the
/// render world and inserts them into [`RenderVisibilityRanges`].
pub fn extract_visibility_ranges(
    mut render_visibility_ranges: ResMut<RenderVisibilityRanges>,
    visibility_ranges_query: Extract<Query<(Entity, &VisibilityRange)>>,
    changed_ranges_query: Extract<Query<Entity, Changed<VisibilityRange>>>,
) {
    if changed_ranges_query.is_empty() {
        return;
    }

    render_visibility_ranges.clear();
    for (entity, visibility_range) in visibility_ranges_query.iter() {
        render_visibility_ranges.insert(entity, visibility_range);
    }
}

/// Writes the [`RenderVisibilityRanges`] table to the GPU.
pub fn write_render_visibility_ranges(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut render_visibility_ranges: ResMut<RenderVisibilityRanges>,
) {
    // If there haven't been any changes, early out.
    if !render_visibility_ranges.buffer_dirty {
        return;
    }

    // Mess with the length of the buffer to meet API requirements if necessary.
    match render_device.get_supported_read_only_binding_type(VISIBILITY_RANGES_STORAGE_BUFFER_COUNT)
    {
        // If we're using a uniform buffer, we must have *exactly*
        // `VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE` elements.
        BufferBindingType::Uniform
            if render_visibility_ranges.buffer.len() > VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE =>
        {
            render_visibility_ranges
                .buffer
                .truncate(VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE);
        }
        BufferBindingType::Uniform
            if render_visibility_ranges.buffer.len() < VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE =>
        {
            while render_visibility_ranges.buffer.len() < VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE {
                render_visibility_ranges.buffer.push(default());
            }
        }

        // Otherwise, if we're using a storage buffer, just ensure there's
        // something in the buffer, or else it won't get allocated.
        BufferBindingType::Storage { .. } if render_visibility_ranges.buffer.is_empty() => {
            render_visibility_ranges.buffer.push(default());
        }

        _ => {}
    }

    // Schedule the write.
    render_visibility_ranges
        .buffer
        .write_buffer(&render_device, &render_queue);
    render_visibility_ranges.buffer_dirty = false;
}
