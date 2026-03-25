use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{prelude::AssetChanged, AssetId, Assets};
use bevy_ecs::{
    entity::{Entity, EntityHashMap},
    prelude::*,
};
use bevy_material::AlphaMode;
use bevy_math::{bounding::Aabb3d, IVec3, Quat, Vec3};
use bevy_mesh::{morph::MeshMorphWeights, skinning::SkinnedMesh, Mesh, Mesh3d};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_transform::{components::GlobalTransform, TransformSystems};

use crate::{MeshMaterial3d, StandardMaterial};

/// Enables the experimental chunk index used by the mass-instance rendering
/// path.
///
/// This plugin only builds and maintains the main-world chunk index. It does
/// not replace the legacy extraction or queuing paths by itself.
#[derive(Default)]
pub struct MassInstanceRenderingPlugin;

impl Plugin for MassInstanceRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MassInstanceRenderingSettings>()
            .init_resource::<MassInstanceChunkIndex>()
            .add_systems(
                PostUpdate,
                sync_mass_instance_chunk_index.after(TransformSystems::Propagate),
            );
    }
}

/// Runtime settings for the experimental mass-instance chunk index.
#[derive(Resource, Clone, Debug)]
pub struct MassInstanceRenderingSettings {
    /// Whether the chunk index should be maintained this frame.
    pub enabled: bool,
    /// Edge length, in world units, for each spatial chunk.
    pub chunk_world_size: f32,
}

impl Default for MassInstanceRenderingSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            chunk_world_size: 64.0,
        }
    }
}

/// Dense integer coordinates identifying a chunk in world space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChunkId(pub IVec3);

/// Phase-1 pass flags supported by the chunked path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct MassInstancePassFlags(pub u8);

impl MassInstancePassFlags {
    /// Opaque-only main-pass rendering.
    pub const MAIN_OPAQUE: Self = Self(1);
}

/// Groups compatible instances inside a chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MassInstanceChunkBatchKey {
    pub mesh_asset_id: AssetId<Mesh>,
    pub material_asset_id: AssetId<StandardMaterial>,
    pub pass_flags: MassInstancePassFlags,
}

#[derive(Clone, Copy, Debug)]
struct MassInstanceEntityLocation {
    chunk_id: ChunkId,
    slot: usize,
    batch_key: MassInstanceChunkBatchKey,
    batch_member_index: usize,
}

/// Dense SoA data for a single chunk.
#[derive(Clone, Debug)]
pub struct MassInstanceChunk {
    pub id: ChunkId,
    pub bounds: Aabb3d,
    pub dirty_epoch: u64,
    pub entities: Vec<Entity>,
    pub translations: Vec<Vec3>,
    pub rotations: Vec<Quat>,
    pub scales: Vec<Vec3>,
    pub mesh_asset_ids: Vec<AssetId<Mesh>>,
    pub material_asset_ids: Vec<AssetId<StandardMaterial>>,
    pub batch_keys: Vec<MassInstanceChunkBatchKey>,
    pub batches: HashMap<MassInstanceChunkBatchKey, Vec<usize>>,
}

impl MassInstanceChunk {
    fn new(id: ChunkId, chunk_world_size: f32, dirty_epoch: u64) -> Self {
        Self {
            id,
            bounds: chunk_bounds(id, chunk_world_size),
            dirty_epoch,
            entities: Vec::new(),
            translations: Vec::new(),
            rotations: Vec::new(),
            scales: Vec::new(),
            mesh_asset_ids: Vec::new(),
            material_asset_ids: Vec::new(),
            batch_keys: Vec::new(),
            batches: HashMap::default(),
        }
    }
}

/// Main-world chunk index for the experimental mass-instance path.
#[derive(Resource, Default)]
pub struct MassInstanceChunkIndex {
    chunk_world_size: f32,
    next_dirty_epoch: u64,
    pub chunks: HashMap<ChunkId, MassInstanceChunk>,
    entity_locations: EntityHashMap<MassInstanceEntityLocation>,
    dirty_chunks: HashSet<ChunkId>,
}

impl MassInstanceChunkIndex {
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn entity_count(&self) -> usize {
        self.entity_locations.len()
    }

    pub fn dirty_chunk_count(&self) -> usize {
        self.dirty_chunks.len()
    }

    pub fn chunk_world_size(&self) -> f32 {
        self.chunk_world_size
    }

    pub fn is_empty(&self) -> bool {
        self.entity_locations.is_empty()
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
        self.entity_locations.clear();
        self.dirty_chunks.clear();
        self.next_dirty_epoch = 0;
    }

    fn clear_and_reconfigure(&mut self, chunk_world_size: f32) {
        self.clear();
        self.chunk_world_size = chunk_world_size;
    }

    fn mark_chunk_dirty(&mut self, chunk_id: ChunkId) {
        self.next_dirty_epoch = self.next_dirty_epoch.saturating_add(1);
        if let Some(chunk) = self.chunks.get_mut(&chunk_id) {
            chunk.dirty_epoch = self.next_dirty_epoch;
            self.dirty_chunks.insert(chunk_id);
        }
    }

    fn upsert_entity(
        &mut self,
        entity: Entity,
        transform: &GlobalTransform,
        mesh_asset_id: AssetId<Mesh>,
        material_asset_id: AssetId<StandardMaterial>,
    ) {
        let (scale, rotation, translation) = transform.to_scale_rotation_translation();
        let chunk_id = chunk_id_for_translation(translation, self.chunk_world_size);
        let batch_key = MassInstanceChunkBatchKey {
            mesh_asset_id,
            material_asset_id,
            pass_flags: MassInstancePassFlags::MAIN_OPAQUE,
        };

        let Some(current_location) = self.entity_locations.get(&entity).copied() else {
            self.insert_new_entity(entity, chunk_id, batch_key, translation, rotation, scale);
            return;
        };

        if current_location.chunk_id != chunk_id {
            self.remove_entity(entity);
            self.insert_new_entity(entity, chunk_id, batch_key, translation, rotation, scale);
            return;
        }

        let mut changed = false;
        if current_location.batch_key != batch_key {
            self.move_between_batches(entity, current_location, batch_key);
            changed = true;
        }

        if let Some(chunk) = self.chunks.get_mut(&chunk_id) {
            chunk.translations[current_location.slot] = translation;
            chunk.rotations[current_location.slot] = rotation;
            chunk.scales[current_location.slot] = scale;
            chunk.mesh_asset_ids[current_location.slot] = mesh_asset_id;
            chunk.material_asset_ids[current_location.slot] = material_asset_id;
            changed = true;
        }

        if changed {
            self.mark_chunk_dirty(chunk_id);
        }
    }

    fn insert_new_entity(
        &mut self,
        entity: Entity,
        chunk_id: ChunkId,
        batch_key: MassInstanceChunkBatchKey,
        translation: Vec3,
        rotation: Quat,
        scale: Vec3,
    ) {
        let dirty_epoch = self.next_dirty_epoch.saturating_add(1);
        let chunk = self.chunks.entry(chunk_id).or_insert_with(|| {
            MassInstanceChunk::new(chunk_id, self.chunk_world_size, dirty_epoch)
        });

        let slot = chunk.entities.len();
        chunk.entities.push(entity);
        chunk.translations.push(translation);
        chunk.rotations.push(rotation);
        chunk.scales.push(scale);
        chunk.mesh_asset_ids.push(batch_key.mesh_asset_id);
        chunk.material_asset_ids.push(batch_key.material_asset_id);
        chunk.batch_keys.push(batch_key);

        let batch_members = chunk.batches.entry(batch_key).or_default();
        let batch_member_index = batch_members.len();
        batch_members.push(slot);

        self.entity_locations.insert(
            entity,
            MassInstanceEntityLocation {
                chunk_id,
                slot,
                batch_key,
                batch_member_index,
            },
        );
        self.mark_chunk_dirty(chunk_id);
    }

    fn move_between_batches(
        &mut self,
        entity: Entity,
        location: MassInstanceEntityLocation,
        new_batch_key: MassInstanceChunkBatchKey,
    ) {
        let Some(chunk) = self.chunks.get_mut(&location.chunk_id) else {
            return;
        };

        if let Some(old_members) = chunk.batches.get_mut(&location.batch_key) {
            old_members.swap_remove(location.batch_member_index);
            if location.batch_member_index < old_members.len() {
                let moved_slot = old_members[location.batch_member_index];
                if let Some(moved_entity) = chunk.entities.get(moved_slot).copied() {
                    if let Some(moved_location) = self.entity_locations.get_mut(&moved_entity) {
                        moved_location.batch_member_index = location.batch_member_index;
                    }
                }
            }
            if old_members.is_empty() {
                chunk.batches.remove(&location.batch_key);
            }
        }

        let new_members = chunk.batches.entry(new_batch_key).or_default();
        let new_batch_member_index = new_members.len();
        new_members.push(location.slot);
        chunk.batch_keys[location.slot] = new_batch_key;

        if let Some(entity_location) = self.entity_locations.get_mut(&entity) {
            entity_location.batch_key = new_batch_key;
            entity_location.batch_member_index = new_batch_member_index;
        }
    }

    fn remove_entity(&mut self, entity: Entity) {
        let Some(location) = self.entity_locations.remove(&entity) else {
            return;
        };

        let mut should_remove_chunk = false;
        let mut dirty_chunk = false;

        if let Some(chunk) = self.chunks.get_mut(&location.chunk_id) {
            if let Some(batch_members) = chunk.batches.get_mut(&location.batch_key) {
                batch_members.swap_remove(location.batch_member_index);
                if location.batch_member_index < batch_members.len() {
                    let moved_slot = batch_members[location.batch_member_index];
                    if let Some(moved_entity) = chunk.entities.get(moved_slot).copied() {
                        if let Some(moved_location) = self.entity_locations.get_mut(&moved_entity) {
                            moved_location.batch_member_index = location.batch_member_index;
                        }
                    }
                }
                if batch_members.is_empty() {
                    chunk.batches.remove(&location.batch_key);
                }
            }

            let last_slot = chunk.entities.len().saturating_sub(1);
            chunk.entities.swap_remove(location.slot);
            chunk.translations.swap_remove(location.slot);
            chunk.rotations.swap_remove(location.slot);
            chunk.scales.swap_remove(location.slot);
            chunk.mesh_asset_ids.swap_remove(location.slot);
            chunk.material_asset_ids.swap_remove(location.slot);
            chunk.batch_keys.swap_remove(location.slot);

            if location.slot < last_slot
                && let Some(swapped_entity) = chunk.entities.get(location.slot).copied()
                && let Some(swapped_location) = self.entity_locations.get_mut(&swapped_entity)
            {
                swapped_location.slot = location.slot;
                if let Some(swapped_batch_members) =
                    chunk.batches.get_mut(&swapped_location.batch_key)
                {
                    swapped_batch_members[swapped_location.batch_member_index] = location.slot;
                }
            }

            should_remove_chunk = chunk.entities.is_empty();
            dirty_chunk = true;
        }

        if should_remove_chunk {
            self.chunks.remove(&location.chunk_id);
            self.dirty_chunks.remove(&location.chunk_id);
        } else if dirty_chunk {
            self.mark_chunk_dirty(location.chunk_id);
        }
    }
}

type AllMassInstanceCandidates<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        &'static Mesh3d,
        &'static MeshMaterial3d<StandardMaterial>,
    ),
    (Without<SkinnedMesh>, Without<MeshMorphWeights>),
>;

type ChangedMassInstanceCandidates<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        &'static Mesh3d,
        &'static MeshMaterial3d<StandardMaterial>,
    ),
    (
        Without<SkinnedMesh>,
        Without<MeshMorphWeights>,
        Or<(
            Added<GlobalTransform>,
            Changed<GlobalTransform>,
            Added<Mesh3d>,
            Changed<Mesh3d>,
            AssetChanged<Mesh3d>,
            Added<MeshMaterial3d<StandardMaterial>>,
            Changed<MeshMaterial3d<StandardMaterial>>,
            AssetChanged<MeshMaterial3d<StandardMaterial>>,
        )>,
    ),
>;

fn sync_mass_instance_chunk_index(
    settings: Res<MassInstanceRenderingSettings>,
    materials: Res<Assets<StandardMaterial>>,
    mut chunk_index: ResMut<MassInstanceChunkIndex>,
    all_candidates: AllMassInstanceCandidates,
    changed_candidates: ChangedMassInstanceCandidates,
    newly_unsupported: Query<Entity, Or<(Added<SkinnedMesh>, Added<MeshMorphWeights>)>>,
    mut removed_meshes: RemovedComponents<Mesh3d>,
    mut removed_materials: RemovedComponents<MeshMaterial3d<StandardMaterial>>,
    mut removed_global_transforms: RemovedComponents<GlobalTransform>,
    mut removed_skinned_meshes: RemovedComponents<SkinnedMesh>,
    mut removed_morph_weights: RemovedComponents<MeshMorphWeights>,
) {
    let chunk_world_size = settings.chunk_world_size.max(0.01);

    if !settings.enabled {
        if !chunk_index.is_empty() {
            chunk_index.clear_and_reconfigure(chunk_world_size);
        }
        return;
    }

    if chunk_index.chunk_world_size() != chunk_world_size {
        rebuild_mass_instance_chunk_index(
            &mut chunk_index,
            chunk_world_size,
            &materials,
            all_candidates.iter(),
        );
        return;
    }

    for entity in removed_meshes
        .read()
        .chain(removed_materials.read())
        .chain(removed_global_transforms.read())
    {
        chunk_index.remove_entity(entity);
    }

    for entity in &newly_unsupported {
        chunk_index.remove_entity(entity);
    }

    let mut potentially_reeligible = Vec::new();
    potentially_reeligible.extend(removed_skinned_meshes.read());
    potentially_reeligible.extend(removed_morph_weights.read());

    for (entity, transform, mesh, material) in &changed_candidates {
        if let Some(material_asset_id) = eligible_material_asset_id(&materials, material) {
            chunk_index.upsert_entity(entity, transform, mesh.id(), material_asset_id);
        } else {
            chunk_index.remove_entity(entity);
        }
    }

    for entity in potentially_reeligible {
        if let Ok((entity, transform, mesh, material)) = all_candidates.get(entity) {
            if let Some(material_asset_id) = eligible_material_asset_id(&materials, material) {
                chunk_index.upsert_entity(entity, transform, mesh.id(), material_asset_id);
            } else {
                chunk_index.remove_entity(entity);
            }
        }
    }
}

fn rebuild_mass_instance_chunk_index<'a>(
    chunk_index: &mut MassInstanceChunkIndex,
    chunk_world_size: f32,
    materials: &Assets<StandardMaterial>,
    candidates: impl Iterator<
        Item = (
            Entity,
            &'a GlobalTransform,
            &'a Mesh3d,
            &'a MeshMaterial3d<StandardMaterial>,
        ),
    >,
) {
    chunk_index.clear_and_reconfigure(chunk_world_size);

    for (entity, transform, mesh, material) in candidates {
        if let Some(material_asset_id) = eligible_material_asset_id(materials, material) {
            chunk_index.upsert_entity(entity, transform, mesh.id(), material_asset_id);
        }
    }
}

fn eligible_material_asset_id(
    materials: &Assets<StandardMaterial>,
    material: &MeshMaterial3d<StandardMaterial>,
) -> Option<AssetId<StandardMaterial>> {
    let material_asset_id = material.id();
    let material_asset = materials.get(material_asset_id)?;

    match material_asset.alpha_mode {
        AlphaMode::Opaque => Some(material_asset_id),
        _ => None,
    }
}

fn chunk_id_for_translation(translation: Vec3, chunk_world_size: f32) -> ChunkId {
    ChunkId(IVec3::new(
        (translation.x / chunk_world_size).floor() as i32,
        (translation.y / chunk_world_size).floor() as i32,
        (translation.z / chunk_world_size).floor() as i32,
    ))
}

fn chunk_bounds(chunk_id: ChunkId, chunk_world_size: f32) -> Aabb3d {
    let half_size = Vec3::splat(chunk_world_size * 0.5);
    let min = chunk_id.0.as_vec3() * chunk_world_size;
    Aabb3d::new(min + half_size, half_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_asset::uuid::Uuid;
    use bevy_transform::prelude::Transform;

    fn mesh_asset_id(value: u128) -> AssetId<Mesh> {
        AssetId::Uuid {
            uuid: Uuid::from_u128(value),
        }
    }

    fn material_asset_id(value: u128) -> AssetId<StandardMaterial> {
        AssetId::Uuid {
            uuid: Uuid::from_u128(value),
        }
    }

    #[test]
    fn inserts_and_batches_entities() {
        let entity = Entity::from_bits(1);
        let mut index = MassInstanceChunkIndex::default();
        index.clear_and_reconfigure(32.0);

        index.upsert_entity(
            entity,
            &GlobalTransform::from(Transform::from_xyz(4.0, 0.0, 4.0)),
            mesh_asset_id(1),
            material_asset_id(2),
        );

        assert_eq!(index.chunk_count(), 1);
        assert_eq!(index.entity_count(), 1);
        assert_eq!(index.dirty_chunk_count(), 1);

        let chunk = index.chunks.values().next().unwrap();
        assert_eq!(chunk.entities, vec![entity]);
        assert_eq!(chunk.mesh_asset_ids, vec![mesh_asset_id(1)]);
        assert_eq!(chunk.material_asset_ids, vec![material_asset_id(2)]);
        assert_eq!(chunk.batches.len(), 1);
    }

    #[test]
    fn moving_entity_between_chunks_updates_membership() {
        let entity = Entity::from_bits(1);
        let mut index = MassInstanceChunkIndex::default();
        index.clear_and_reconfigure(16.0);

        index.upsert_entity(
            entity,
            &GlobalTransform::from(Transform::from_xyz(1.0, 0.0, 1.0)),
            mesh_asset_id(1),
            material_asset_id(2),
        );
        index.dirty_chunks.clear();

        index.upsert_entity(
            entity,
            &GlobalTransform::from(Transform::from_xyz(64.0, 0.0, 1.0)),
            mesh_asset_id(1),
            material_asset_id(2),
        );

        assert_eq!(index.chunk_count(), 1);
        assert_eq!(index.entity_count(), 1);
        assert_eq!(index.dirty_chunk_count(), 1);
        let chunk = index.chunks.values().next().unwrap();
        assert_eq!(chunk.entities, vec![entity]);
    }

    #[test]
    fn changing_batch_key_moves_entity_between_batches() {
        let entity = Entity::from_bits(1);
        let mut index = MassInstanceChunkIndex::default();
        index.clear_and_reconfigure(32.0);

        index.upsert_entity(
            entity,
            &GlobalTransform::from(Transform::from_xyz(2.0, 0.0, 2.0)),
            mesh_asset_id(1),
            material_asset_id(2),
        );

        index.upsert_entity(
            entity,
            &GlobalTransform::from(Transform::from_xyz(2.0, 0.0, 2.0)),
            mesh_asset_id(3),
            material_asset_id(4),
        );

        let chunk = index.chunks.values().next().unwrap();
        assert_eq!(chunk.entities, vec![entity]);
        assert_eq!(chunk.mesh_asset_ids, vec![mesh_asset_id(3)]);
        assert_eq!(chunk.material_asset_ids, vec![material_asset_id(4)]);
        assert_eq!(chunk.batches.len(), 1);
    }

    #[test]
    fn removing_last_entity_drops_empty_chunk() {
        let entity = Entity::from_bits(1);
        let mut index = MassInstanceChunkIndex::default();
        index.clear_and_reconfigure(32.0);

        index.upsert_entity(
            entity,
            &GlobalTransform::from(Transform::from_xyz(2.0, 0.0, 2.0)),
            mesh_asset_id(1),
            material_asset_id(2),
        );
        index.remove_entity(entity);

        assert!(index.is_empty());
        assert!(index.chunks.is_empty());
        assert!(index.dirty_chunks.is_empty());
    }
}
