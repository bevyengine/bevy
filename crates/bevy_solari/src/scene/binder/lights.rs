use super::allocator::SlotAllocator;
use crate::scene::extract::{
    ExtractedRaytracingDirectionalLight, ExtractedRaytracingPointLight,
    ExtractedRaytracingRectLight, ExtractedRaytracingSpotLight,
};
use bevy_ecs::{entity::Entity, query::Changed, system::Query};
use bevy_math::{ops::cos, Vec3};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_render::render_resource::{AtomicSparseBufferVec, BufferUsages};
use bevy_render::{impl_atomic_pod, render_resource::AtomicPod};
use bytemuck::{Pod, Zeroable};
use core::sync::atomic::{AtomicBool, Ordering};
use core::{f32::consts::TAU, hash::Hash};

const LIGHT_NOT_PRESENT_THIS_FRAME: u32 = u32::MAX;

const LIGHT_SOURCE_KIND_BITS: u32 = 3;

const LIGHT_SOURCE_KIND_EMISSIVE_MESH: u32 = 0;
const LIGHT_SOURCE_KIND_DIRECTIONAL: u32 = 1;
const LIGHT_SOURCE_KIND_POINT: u32 = 2;
const LIGHT_SOURCE_KIND_RECT: u32 = 3;

#[derive(Clone, Copy, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct GpuLightSource {
    kind: u32,
    id: u32,
}

/// Stable identity for one light in the light array.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum LightSourceId {
    EmissiveMesh(Entity),
    DirectionalLight(Entity),
    PointLight(Entity),
    SpotLight(Entity),
    RectLight(Entity),
}

#[derive(Default)]
pub struct LightIndex {
    indices: HashMap<LightSourceId, u32>,
    ids: Vec<LightSourceId>,
    changed: HashSet<LightSourceId>,
}

impl LightIndex {
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    fn get(&self, id: &LightSourceId) -> Option<u32> {
        self.indices.get(id).copied()
    }

    fn insert(&mut self, id: LightSourceId) -> u32 {
        if let Some(&index) = self.indices.get(&id) {
            return index;
        }

        let index = self.ids.len() as u32;
        self.ids.push(id);
        self.indices.insert(id, index);
        self.changed.insert(id);
        index
    }

    /// Removes `id` and reports both its old index and the old final index.
    ///
    /// Only the ids tracked here are swapped down. When the two indices differ, the caller has to
    /// mirror that swap in `sources`, copying the element at the old final index into the hole.
    fn remove(&mut self, id: LightSourceId) -> Option<(u32, u32)> {
        let index = self.indices.remove(&id)?;
        self.changed.insert(id);

        let last = self.ids.len() as u32 - 1;
        self.ids.swap_remove(index as usize);

        if index != last {
            let moved = self.ids[index as usize];
            self.indices.insert(moved, index);
            self.changed.insert(moved);
        }

        Some((index, last))
    }
}

impl GpuLightSource {
    pub fn new_emissive_mesh_light(instance_id: u32, triangle_count: u32) -> GpuLightSource {
        if triangle_count > u16::MAX as u32 {
            panic!("Too many triangles ({triangle_count}) in an emissive mesh, maximum is 65535.");
        }

        Self {
            kind: (triangle_count << LIGHT_SOURCE_KIND_BITS) | LIGHT_SOURCE_KIND_EMISSIVE_MESH,
            id: instance_id,
        }
    }

    fn new_directional_light(directional_light_id: u32) -> GpuLightSource {
        Self {
            kind: LIGHT_SOURCE_KIND_DIRECTIONAL,
            id: directional_light_id,
        }
    }

    fn new_point_light(point_light_id: u32) -> GpuLightSource {
        Self {
            kind: LIGHT_SOURCE_KIND_POINT,
            id: point_light_id,
        }
    }

    fn new_rect_light(rect_light_id: u32) -> GpuLightSource {
        Self {
            kind: LIGHT_SOURCE_KIND_RECT,
            id: rect_light_id,
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct GpuDirectionalLight {
    direction_to_light: Vec3,
    cos_theta_max: f32,
    luminance: Vec3,
    inverse_pdf: f32,
}

#[derive(Clone, Copy, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct GpuPointLight {
    position: Vec3,
    radius: f32,
    intensity: Vec3,
    cos_inner: f32,
    axis: Vec3,
    cos_outer: f32,
}

#[derive(Clone, Copy, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct GpuRectLight {
    center: Vec3,
    area: f32,
    edge_x: Vec3,
    _padding0: f32,
    edge_y: Vec3,
    _padding1: f32,
    radiance: Vec3,
    _padding2: f32,
}

impl_atomic_pod!(GpuLightSource, GpuLightSourceBlob);
impl_atomic_pod!(GpuDirectionalLight, GpuDirectionalLightBlob);
impl_atomic_pod!(GpuPointLight, GpuPointLightBlob);
impl_atomic_pod!(GpuRectLight, GpuRectLightBlob);

impl GpuDirectionalLight {
    fn new(directional_light: &ExtractedRaytracingDirectionalLight) -> Self {
        let cos_theta_max = cos(directional_light.sun_disk_angular_size / 2.0);
        let solid_angle = TAU * (1.0 - cos_theta_max);

        let (luminance, inverse_pdf) = if solid_angle > 0.0 {
            (directional_light.illuminance / solid_angle, solid_angle)
        } else {
            (directional_light.illuminance, 1.0)
        };

        Self {
            direction_to_light: directional_light.direction_to_light,
            cos_theta_max,
            luminance,
            inverse_pdf,
        }
    }
}

impl GpuPointLight {
    fn new_point(point_light: &ExtractedRaytracingPointLight) -> Self {
        // Point lights get a cone that never cuts anything off
        Self {
            position: point_light.position,
            radius: point_light.radius,
            intensity: point_light.intensity,
            cos_inner: -1.0,
            axis: Vec3::NEG_Z,
            cos_outer: -1.0,
        }
    }

    fn new_spot(spot_light: &ExtractedRaytracingSpotLight) -> Self {
        Self {
            position: spot_light.position,
            radius: spot_light.radius,
            intensity: spot_light.intensity,
            cos_inner: spot_light.cos_inner,
            axis: spot_light.axis,
            cos_outer: spot_light.cos_outer,
        }
    }
}

impl GpuRectLight {
    fn new(rect_light: &ExtractedRaytracingRectLight) -> Self {
        Self {
            center: rect_light.center,
            area: rect_light.area,
            edge_x: rect_light.edge_x,
            _padding0: 0.0,
            edge_y: rect_light.edge_y,
            _padding1: 0.0,
            radiance: rect_light.radiance,
            _padding2: 0.0,
        }
    }
}

/// Light slots and the incremental previous-frame id translation state.
pub struct LightState {
    /// Kept gap-free because shaders derive the light count with `arrayLength`.
    pub sources: AtomicSparseBufferVec<GpuLightSource>,
    pub directional_lights: AtomicSparseBufferVec<GpuDirectionalLight>,
    pub point_lights: AtomicSparseBufferVec<GpuPointLight>,
    pub rect_lights: AtomicSparseBufferVec<GpuRectLight>,
    pub previous_frame_id_translations: AtomicSparseBufferVec<u32>,
    pub index: LightIndex,
    /// Light ids as of the last frame whose translation table the lighting shader actually read.
    previous_index: HashMap<LightSourceId, u32>,
    nonidentity_translations: Vec<u32>,
    directional_light_slots: SlotAllocator<Entity>,
    /// Spot lights share the point light buffer, and so cannot be keyed by just Entity.
    point_light_slots: SlotAllocator<LightSourceId>,
    rect_light_slots: SlotAllocator<Entity>,
    /// Set by the lighting node once it has recorded work reading the translation table.
    translations_consumed: AtomicBool,
}

impl LightState {
    pub fn new() -> Self {
        Self {
            sources: AtomicSparseBufferVec::new(
                BufferUsages::STORAGE,
                "solari_light_sources".into(),
            ),
            directional_lights: AtomicSparseBufferVec::new(
                BufferUsages::STORAGE,
                "solari_directional_lights".into(),
            ),
            point_lights: AtomicSparseBufferVec::new(
                BufferUsages::STORAGE,
                "solari_point_lights".into(),
            ),
            rect_lights: AtomicSparseBufferVec::new(
                BufferUsages::STORAGE,
                "solari_rect_lights".into(),
            ),
            previous_frame_id_translations: AtomicSparseBufferVec::new(
                BufferUsages::STORAGE,
                "solari_previous_frame_light_id_translations".into(),
            ),
            index: LightIndex::default(),
            previous_index: HashMap::default(),
            nonidentity_translations: Vec::new(),
            directional_light_slots: SlotAllocator::new(),
            point_light_slots: SlotAllocator::new(),
            rect_light_slots: SlotAllocator::new(),
            translations_consumed: AtomicBool::new(false),
        }
    }

    /// Applies this frame's light changes.
    ///
    /// Lights are retained, so only the ones that changed are processed. Removals go first so that
    /// a light removed and re-added in the same frame ends up present.
    pub fn update_directional_lights(
        &mut self,
        changed: &Query<
            (Entity, &ExtractedRaytracingDirectionalLight),
            Changed<ExtractedRaytracingDirectionalLight>,
        >,
        removed: impl IntoIterator<Item = Entity>,
    ) {
        // Removals go first so that a light removed and re-added in the same frame ends up present
        for entity in removed {
            if self.directional_light_slots.remove(&entity).is_some() {
                self.remove_light(LightSourceId::DirectionalLight(entity));
            }
        }
        for (entity, directional_light) in changed {
            let slot = self.directional_light_slots.get_or_allocate(entity);
            self.directional_lights
                .grow_and_set(slot, GpuDirectionalLight::new(directional_light));
            self.add_light(
                LightSourceId::DirectionalLight(entity),
                GpuLightSource::new_directional_light(slot),
            );
        }
    }

    pub fn update_point_lights(
        &mut self,
        changed: &Query<
            (Entity, &ExtractedRaytracingPointLight),
            Changed<ExtractedRaytracingPointLight>,
        >,
        removed: impl IntoIterator<Item = Entity>,
    ) {
        for entity in removed {
            self.remove_point_or_spot_light(LightSourceId::PointLight(entity));
        }
        for (entity, point_light) in changed {
            self.set_point_or_spot_light(
                LightSourceId::PointLight(entity),
                GpuPointLight::new_point(point_light),
            );
        }
    }

    pub fn update_spot_lights(
        &mut self,
        changed: &Query<
            (Entity, &ExtractedRaytracingSpotLight),
            Changed<ExtractedRaytracingSpotLight>,
        >,
        removed: impl IntoIterator<Item = Entity>,
    ) {
        for entity in removed {
            self.remove_point_or_spot_light(LightSourceId::SpotLight(entity));
        }
        for (entity, spot_light) in changed {
            self.set_point_or_spot_light(
                LightSourceId::SpotLight(entity),
                GpuPointLight::new_spot(spot_light),
            );
        }
    }

    pub fn update_rect_lights(
        &mut self,
        changed: &Query<
            (Entity, &ExtractedRaytracingRectLight),
            Changed<ExtractedRaytracingRectLight>,
        >,
        removed: impl IntoIterator<Item = Entity>,
    ) {
        for entity in removed {
            if self.rect_light_slots.remove(&entity).is_some() {
                self.remove_light(LightSourceId::RectLight(entity));
            }
        }
        for (entity, rect_light) in changed {
            let slot = self.rect_light_slots.get_or_allocate(entity);
            self.rect_lights
                .grow_and_set(slot, GpuRectLight::new(rect_light));
            self.add_light(
                LightSourceId::RectLight(entity),
                GpuLightSource::new_rect_light(slot),
            );
        }
    }

    pub fn finish_update(&mut self) {
        self.write_light_id_translations();

        if self.index.len() > u16::MAX as usize {
            panic!("Too many light sources in the scene, maximum is 65535.");
        }
    }

    fn set_point_or_spot_light(&mut self, id: LightSourceId, light: GpuPointLight) {
        let slot = self.point_light_slots.get_or_allocate(id);
        self.point_lights.grow_and_set(slot, light);
        self.add_light(id, GpuLightSource::new_point_light(slot));
    }

    fn remove_point_or_spot_light(&mut self, id: LightSourceId) {
        if self.point_light_slots.remove(&id).is_some() {
            self.remove_light(id);
        }
    }

    pub fn add_light(&mut self, id: LightSourceId, source: GpuLightSource) {
        let index = self.index.insert(id);
        self.sources.grow_and_set(index, source);
    }

    /// Removes a light, moving the last one down into the hole so the array stays gap-free.
    pub fn remove_light(&mut self, id: LightSourceId) {
        let Some((index, last)) = self.index.remove(id) else {
            return;
        };

        if index != last {
            let source = self.sources.get(last);
            self.sources.grow_and_set(index, source);
        }
    }

    /// Rolls the translation table over for a new frame.
    ///
    /// `previous_index` and `changed` only advance once the shader has read the table. The
    /// lighting node bails out while its pipelines compile, and the reservoirs keep the older ids
    /// across such a gap, so the next table has to translate from those instead. `has_consumers`
    /// is false when no view runs Solari lighting, where deferring forever would grow both
    /// without bound.
    pub fn begin_frame(&mut self, has_consumers: bool) {
        for index in core::mem::take(&mut self.nonidentity_translations) {
            self.previous_frame_id_translations
                .grow_and_set(index, index);
        }

        if !has_consumers || self.translations_consumed.swap(false, Ordering::Relaxed) {
            for id in core::mem::take(&mut self.index.changed) {
                match self.index.get(&id) {
                    Some(index) => self.previous_index.insert(id, index),
                    None => self.previous_index.remove(&id),
                };
            }
        }
    }

    /// Records that the lighting shader read this frame's translation table.
    pub fn note_translations_consumed(&self) {
        self.translations_consumed.store(true, Ordering::Relaxed);
    }

    /// Records where each light that moved or disappeared this frame ended up, so that reservoirs
    /// still carrying last frame's light ids can be remapped.
    fn write_light_id_translations(&mut self) {
        for id in &self.index.changed {
            // Lights that first appeared since the last read table have no previous id
            let Some(&previous) = self.previous_index.get(id) else {
                continue;
            };
            let current = self.index.get(id).unwrap_or(LIGHT_NOT_PRESENT_THIS_FRAME);

            if current != previous {
                self.previous_frame_id_translations
                    .grow_and_set(previous, current);
                self.nonidentity_translations.push(previous);
            }
        }

        // Every index the shader might read has to be backed by a real element
        let light_count = self.index.len() as u32;
        let translations = &mut self.previous_frame_id_translations;
        if translations.len() < light_count {
            let start = translations.len();
            translations.grow(light_count);
            for index in start..light_count {
                translations.set(index, index);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LightIndex, LightSourceId};
    use bevy_ecs::entity::Entity;

    const ENTITY: Entity = Entity::PLACEHOLDER;

    #[test]
    fn light_index_keeps_sources_on_the_same_entity_independent() {
        let mut lights = LightIndex::default();

        assert_eq!(lights.insert(LightSourceId::EmissiveMesh(ENTITY)), 0);
        assert_eq!(lights.insert(LightSourceId::DirectionalLight(ENTITY)), 1);
        assert_eq!(lights.insert(LightSourceId::PointLight(ENTITY)), 2);
        assert_eq!(lights.insert(LightSourceId::SpotLight(ENTITY)), 3);
        assert_eq!(lights.insert(LightSourceId::RectLight(ENTITY)), 4);
        assert_eq!(lights.len(), 5);
    }

    #[test]
    fn light_index_insert_is_idempotent() {
        let point = LightSourceId::PointLight(ENTITY);
        let mut lights = LightIndex::default();

        assert_eq!(lights.insert(point), 0);
        assert_eq!(lights.insert(point), 0);
        assert_eq!(lights.len(), 1);
    }

    #[test]
    fn light_index_remove_moves_the_last_source_into_the_hole() {
        let emissive = LightSourceId::EmissiveMesh(ENTITY);
        let point = LightSourceId::PointLight(ENTITY);
        let rect = LightSourceId::RectLight(ENTITY);
        let mut lights = LightIndex::default();
        lights.insert(emissive);
        lights.insert(point);
        lights.insert(rect);

        assert_eq!(lights.remove(emissive), Some((0, 2)));
        assert_eq!(lights.get(&emissive), None);
        assert_eq!(lights.get(&rect), Some(0));
        assert_eq!(lights.get(&point), Some(1));
        assert_eq!(lights.len(), 2);
    }

    #[test]
    fn light_index_remove_of_the_last_source_moves_nothing() {
        let directional = LightSourceId::DirectionalLight(ENTITY);
        let rect = LightSourceId::RectLight(ENTITY);
        let mut lights = LightIndex::default();
        lights.insert(directional);
        lights.insert(rect);

        assert_eq!(lights.remove(rect), Some((1, 1)));
        assert_eq!(lights.get(&directional), Some(0));
        assert_eq!(lights.len(), 1);
    }

    #[test]
    fn light_index_remove_of_a_missing_source_returns_none() {
        let mut lights = LightIndex::default();
        lights.insert(LightSourceId::PointLight(ENTITY));

        assert_eq!(lights.remove(LightSourceId::RectLight(ENTITY)), None);
        assert_eq!(lights.len(), 1);
    }

    #[test]
    fn light_index_remove_of_every_source_leaves_it_empty() {
        let emissive = LightSourceId::EmissiveMesh(ENTITY);
        let directional = LightSourceId::DirectionalLight(ENTITY);
        let mut lights = LightIndex::default();
        lights.insert(emissive);
        lights.insert(directional);

        assert_eq!(lights.remove(emissive), Some((0, 1)));
        assert_eq!(lights.remove(directional), Some((0, 0)));
        assert!(lights.is_empty());
    }
}
