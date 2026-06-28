//! Specific distances from the camera in which entities are visible, also known
//! as *hierarchical levels of detail* or *HLOD*s.

use super::VisibilityRange;
use bevy_app::{App, Plugin};
use bevy_ecs::{
    entity::Entity,
    lifecycle::RemovedComponents,
    query::Changed,
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Query, Res, ResMut},
};
use bevy_log::warn_once;
use bevy_math::{vec4, Vec4};
use bevy_platform::collections::HashMap;
use bevy_utils::prelude::default;
use nonmax::NonMaxU16;
use wgpu::{BufferBindingType, BufferUsages};

use crate::{
    render_resource::BufferVec,
    renderer::{RenderDevice, RenderQueue},
    sync_world::{MainEntity, MainEntityHashMap},
    Extract, ExtractSchedule, GpuResourceAppExt, Render, RenderApp, RenderSystems,
};

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

/// A plugin that enables [`RenderVisibilityRanges`]s, which allow entities to be
/// hidden or shown based on distance to the camera.
pub struct RenderVisibilityRangePlugin;

impl Plugin for RenderVisibilityRangePlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_gpu_resource::<RenderVisibilityRanges>()
            .add_systems(ExtractSchedule, extract_visibility_ranges)
            .add_systems(
                Render,
                write_render_visibility_ranges.in_set(RenderSystems::PrepareResourcesFlush),
            );
    }
}

/// Stores information related to [`VisibilityRange`]s in the render world.
#[derive(Resource)]
pub struct RenderVisibilityRanges {
    /// Information corresponding to each entity.
    entities: MainEntityHashMap<RenderVisibilityEntityInfo>,

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
    /// Clears the per-frame entity table in preparation for a new frame.
    ///
    /// `range_to_index` and `buffer` are deliberately *not* cleared: the
    /// GPU-driven path bakes a range's buffer index into the mesh's
    /// `MeshInputUniform` at extraction and only refreshes it on re-extraction,
    /// so a range must keep the same index for the lifetime of the app or
    /// still-visible meshes would end up pointing at the wrong slot.
    fn clear(&mut self) {
        self.entities.clear();
    }

    /// Inserts a new entity into the [`RenderVisibilityRanges`].
    fn insert(&mut self, entity: MainEntity, visibility_range: &VisibilityRange) {
        // Reuse this range's slot, or append a new one. Indices are never
        // reused, so any index already baked into a mesh stays valid.
        let buffer_index = match self.range_to_index.get(visibility_range) {
            Some(index) => *index,
            None => {
                // `try_from` errors instead of wrapping past `NonMaxU16`'s
                // range, so overflow warns rather than silently aliasing onto
                // slot 0.
                let index = u16::try_from(self.range_to_index.len())
                    .ok()
                    .and_then(|next| NonMaxU16::try_from(next).ok())
                    .unwrap_or_else(|| {
                        warn_once!(
                            "More than {} distinct `VisibilityRange`s are in use; \
                             additional ranges will share GPU slot 0 and may be \
                             culled or crossfaded incorrectly.",
                            u16::MAX
                        );
                        NonMaxU16::default()
                    });
                self.range_to_index.insert(visibility_range.clone(), index);
                self.buffer_dirty = true;
                index
            }
        };

        self.entities.insert(
            entity,
            RenderVisibilityEntityInfo {
                buffer_index,
                is_abrupt: visibility_range.is_abrupt(),
            },
        );
    }

    /// Rebuilds the GPU buffer from `range_to_index` in index order. Only runs
    /// when a new range was added this frame, and leaves `buffer_dirty` set for
    /// [`write_render_visibility_ranges`] to consume.
    fn rebuild_buffer_if_dirty(&mut self) {
        if !self.buffer_dirty {
            return;
        }
        let mut ordered = vec![Vec4::ZERO; self.range_to_index.len()];
        for (range, index) in &self.range_to_index {
            ordered[index.get() as usize] = vec4(
                range.start_margin.start,
                range.start_margin.end,
                range.end_margin.start,
                range.end_margin.end,
            );
        }
        self.buffer.clear();
        for value in ordered {
            self.buffer.push(value);
        }
    }

    /// Returns the index in the GPU buffer corresponding to the visible range
    /// for the given entity.
    ///
    /// If the entity has no visible range, returns `None`.
    #[inline]
    pub fn lod_index_for_entity(&self, entity: MainEntity) -> Option<NonMaxU16> {
        self.entities.get(&entity).map(|info| info.buffer_index)
    }

    /// Returns true if the entity has a visibility range and it isn't abrupt:
    /// i.e. if it has a crossfade.
    #[inline]
    pub fn entity_has_crossfading_visibility_ranges(&self, entity: MainEntity) -> bool {
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

/// Extracts all [`VisibilityRange`] components from the main world to the
/// render world and inserts them into [`RenderVisibilityRanges`].
pub fn extract_visibility_ranges(
    mut render_visibility_ranges: ResMut<RenderVisibilityRanges>,
    visibility_ranges_query: Extract<Query<(Entity, &VisibilityRange)>>,
    changed_ranges_query: Extract<Query<Entity, Changed<VisibilityRange>>>,
    mut removed_visibility_ranges: Extract<RemovedComponents<VisibilityRange>>,
) {
    if changed_ranges_query.is_empty() && removed_visibility_ranges.read().next().is_none() {
        return;
    }

    render_visibility_ranges.clear();
    for (entity, visibility_range) in visibility_ranges_query.iter() {
        render_visibility_ranges.insert(entity.into(), visibility_range);
    }
    render_visibility_ranges.rebuild_buffer_if_dirty();
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
