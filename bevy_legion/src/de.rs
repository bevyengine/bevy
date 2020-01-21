use crate::{
    entity::{Entity, EntityAllocator},
    storage::{
        ArchetypeData, ArchetypeDescription, Chunkset, ComponentMeta, ComponentTypeId, TagMeta,
        TagStorage, TagTypeId,
    },
    world::World,
};
use serde::{
    self,
    de::{self, DeserializeSeed, Visitor},
    Deserialize, Deserializer,
};
use std::{cell::RefCell, collections::HashMap, ptr::NonNull};

/// Returns a type that implements `serde::DeserializeSeed`.
/// Pass the returned value to your `serde::Deserializer`.
/// The caller must provide an implementation for `WorldDeserializer`.
pub fn deserializable<'a, 'b, WD: WorldDeserializer>(
    world: &'a mut World,
    deserialize_impl: &'b WD,
) -> WorldDeserialize<'a, 'b, WD> {
    WorldDeserialize {
        world,
        user: deserialize_impl,
    }
}

/// Deserializes data into the provided World using the provided `serde::Deserializer`.
/// The caller must provide an implementation for `WorldDeserializer`.
pub fn deserialize<'dd, 'a, 'b, WD: WorldDeserializer, D: Deserializer<'dd>>(
    world: &'a mut World,
    deserialize_impl: &'b WD,
    deserializer: D,
) -> Result<(), <D as Deserializer<'dd>>::Error> {
    let deserializable = deserializable(world, deserialize_impl);
    <WorldDeserialize<WD> as DeserializeSeed>::deserialize(deserializable, deserializer)
}

/// User must implement this trait to deserialize a World.
/// The implementation must match that of the `WorldSerializer` provided
/// when serializing the data that is to be deserialized by this impl.
pub trait WorldDeserializer {
    /// Deserializes an ArchetypeDescription
    fn deserialize_archetype_description<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
    ) -> Result<ArchetypeDescription, <D as Deserializer<'de>>::Error>;
    /// Deserializes component data.
    /// `get_next_storage_fn` will return Some(component_data_ptr, num_elements) until all
    /// reserved memory has been exhausted, whereupon it will return None.
    /// `component_data_ptr` are pointers to reserved memory in chunks
    /// that have been reserved to accomodate the number of entities that were previously deserialized
    /// by `deserialize_entities`.
    ///
    /// # Safety
    ///
    /// The implementation must ensure `get_next_storage_fn` is called until it returns
    /// None, and that all memory returned by `get_next_storage_fn` is properly initialized
    /// before this function returns.
    fn deserialize_components<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
        component_type: &ComponentTypeId,
        component_meta: &ComponentMeta,
        get_next_storage_fn: &mut dyn FnMut() -> Option<(NonNull<u8>, usize)>,
    ) -> Result<(), <D as Deserializer<'de>>::Error>;
    /// Deserializes tag data into a TagStorage.
    fn deserialize_tags<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
        tag_type: &TagTypeId,
        tag_meta: &TagMeta,
        tags: &mut TagStorage,
    ) -> Result<(), <D as Deserializer<'de>>::Error>;
    /// Deserializes entity identifiers into the provided buffer.
    fn deserialize_entities<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
        entity_allocator: &EntityAllocator,
        entities: &mut Vec<Entity>,
    ) -> Result<(), <D as Deserializer<'de>>::Error>;
}

/// Implements `DeserializeSeed` and can be passed to a `serde::Deserializer`.
pub struct WorldDeserialize<'a, 'b, WD: WorldDeserializer> {
    user: &'b WD,
    world: &'a mut World,
}
impl<'de, 'a, 'b, WD: WorldDeserializer> DeserializeSeed<'de> for WorldDeserialize<'a, 'b, WD> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let world_refcell = RefCell::new(self.world);
        deserializer.deserialize_seq(SeqDeserializer(ArchetypeDeserializer {
            user: self.user,
            world: &world_refcell,
        }))?;
        Ok(())
    }
}
#[derive(Deserialize, Debug)]
#[serde(field_identifier, rename_all = "snake_case")]
enum ArchetypeField {
    Description,
    Tags,
    ChunkSets,
}
struct ArchetypeDeserializer<'a, 'b, WD: WorldDeserializer> {
    user: &'b WD,
    world: &'a RefCell<&'a mut World>,
}
impl<'a, 'b, WD: WorldDeserializer> Clone for ArchetypeDeserializer<'a, 'b, WD> {
    fn clone(&self) -> Self {
        Self {
            user: self.user,
            world: self.world,
        }
    }
}
impl<'de, 'a, 'b, WD: WorldDeserializer> DeserializeSeed<'de>
    for ArchetypeDeserializer<'a, 'b, WD>
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        impl<'a, 'b, 'de, WD: WorldDeserializer> Visitor<'de> for ArchetypeDeserializer<'a, 'b, WD> {
            type Value = ();

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Archetype")
            }
            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                let archetype_idx = seq
                    .next_element_seed(ArchetypeDescriptionDeserialize {
                        user: self.user,
                        world: self.world,
                    })?
                    .expect("expected description");
                let mut world = self.world.borrow_mut();
                let archetype_data = &mut world.storage_mut().archetypes_mut()[archetype_idx];
                let chunkset_map = seq
                    .next_element_seed(TagsDeserializer {
                        user: self.user,
                        archetype: archetype_data,
                    })?
                    .expect("expected tags");
                seq.next_element_seed(ChunkSetDeserializer {
                    user: self.user,
                    world: &mut *world,
                    archetype_idx,
                    chunkset_map: &chunkset_map,
                })?
                .expect("expected chunk_sets");
                Ok(())
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut archetype_idx = None;
                let mut chunkset_map = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        ArchetypeField::Description => {
                            archetype_idx =
                                Some(map.next_value_seed(ArchetypeDescriptionDeserialize {
                                    user: self.user,
                                    world: self.world,
                                })?);
                        }
                        ArchetypeField::Tags => {
                            let archetype_idx =
                                archetype_idx.expect("expected archetype description before tags");
                            let mut world = self.world.borrow_mut();
                            let archetype_data =
                                &mut world.storage_mut().archetypes_mut()[archetype_idx];
                            chunkset_map = Some(map.next_value_seed(TagsDeserializer {
                                user: self.user,
                                archetype: archetype_data,
                            })?);
                        }
                        ArchetypeField::ChunkSets => {
                            let archetype_idx = archetype_idx
                                .expect("expected archetype description before chunksets");
                            let mut world = self.world.borrow_mut();
                            map.next_value_seed(ChunkSetDeserializer {
                                user: self.user,
                                world: &mut *world,
                                archetype_idx,
                                chunkset_map: chunkset_map
                                    .as_ref()
                                    .expect("expected tags before chunksets"),
                            })?;
                            return Ok(());
                        }
                    }
                }
                Err(de::Error::missing_field("data"))
            }
        }
        const FIELDS: &[&str] = &["description", "tags", "chunk_sets"];
        deserializer.deserialize_struct("Archetype", FIELDS, self)
    }
}

pub struct SeqDeserializer<T>(T);

impl<'de, T: DeserializeSeed<'de> + Clone> DeserializeSeed<'de> for SeqDeserializer<T> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}
impl<'de, T: DeserializeSeed<'de> + Clone> Visitor<'de> for SeqDeserializer<T> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("sequence of objects")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        while let Some(_) = seq.next_element_seed::<T>(self.0.clone())? {}
        Ok(())
    }
}
struct ArchetypeDescriptionDeserialize<'a, 'b, WD: WorldDeserializer> {
    user: &'b WD,
    world: &'a RefCell<&'a mut World>,
}
impl<'de, 'a, 'b, WD: WorldDeserializer> DeserializeSeed<'de>
    for ArchetypeDescriptionDeserialize<'a, 'b, WD>
{
    type Value = usize;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let archetype_desc = <WD as WorldDeserializer>::deserialize_archetype_description::<D>(
            self.user,
            deserializer,
        )?;
        let mut world = self.world.borrow_mut();
        let storage = world.storage_mut();
        Ok(storage
            .archetypes()
            .iter()
            .position(|a| a.description() == &archetype_desc)
            .unwrap_or_else(|| {
                let (idx, _) = storage.alloc_archetype(archetype_desc);
                idx
            }))
    }
}

type ChunkSetMapping = HashMap<usize, usize>;

struct TagsDeserializer<'a, 'b, WD: WorldDeserializer> {
    user: &'b WD,
    archetype: &'a mut ArchetypeData,
}
impl<'de, 'a, 'b, WD: WorldDeserializer> DeserializeSeed<'de> for TagsDeserializer<'a, 'b, WD> {
    type Value = ChunkSetMapping;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let (mut deserialized_tags, this) = deserializer.deserialize_seq(self)?;
        let tag_types = this.archetype.description().tags().to_vec();
        let mut chunkset_map = ChunkSetMapping::new();
        let tags = this.archetype.tags_mut();
        assert_eq!(tags.0.len(), tag_types.len());

        // To simplify later code, shuffle the &mut tag_storage indices to match tag_types
        let world_tag_storages = {
            let mut world_tag_storages: Vec<&mut TagStorage> = Vec::with_capacity(tag_types.len());
            for (tag_type, tag_storage) in tags.0.iter_mut() {
                let type_idx = tag_types
                    .iter()
                    .position(|(ty, _)| ty == tag_type)
                    .expect("tag type mismatch with Tags");
                unsafe {
                    std::ptr::write(world_tag_storages.as_mut_ptr().add(type_idx), tag_storage);
                }
            }
            unsafe {
                world_tag_storages.set_len(tag_types.len());
            }
            world_tag_storages
        };

        let num_world_values = world_tag_storages.iter().map(|ts| ts.len()).nth(0);
        let num_tag_values = deserialized_tags
            .iter()
            .map(|ts| ts.len())
            .nth(0)
            .unwrap_or(0);
        let mut chunksets_to_add = Vec::new();
        for i in 0..num_tag_values {
            let mut matching_idx = None;
            if let Some(num_world_values) = num_world_values {
                for j in 0..num_world_values {
                    let mut is_matching = true;
                    for tag_idx in 0..tag_types.len() {
                        unsafe {
                            let (de_ptr, stride, _) = deserialized_tags[tag_idx].data_raw();
                            let (world_ptr, _, _) = world_tag_storages[tag_idx].data_raw();
                            let (_, tag_meta) = tag_types[tag_idx];
                            let de_offset = (i * stride) as isize;
                            let world_offset = (j * stride) as isize;
                            if !tag_meta.equals(
                                de_ptr.as_ptr().offset(de_offset),
                                world_ptr.as_ptr().offset(world_offset),
                            ) {
                                is_matching = false;
                                break;
                            }
                        }
                    }
                    if is_matching {
                        matching_idx = Some(j);
                        break;
                    }
                }
            }
            // If we have a matching tag set, we will drop our temporary values manually.
            // All temporary TagStorages in `deserialized_tags` will be forgotten later
            // because we move data into World when allocating a new chunkset
            if let Some(world_idx) = matching_idx {
                chunkset_map.insert(i, world_idx);
                for tag_idx in 0..tag_types.len() {
                    unsafe {
                        let (_, tag_meta) = tag_types[tag_idx];
                        let (de_ptr, stride, _) = deserialized_tags[tag_idx].data_raw();
                        let de_offset = (i * stride) as isize;
                        tag_meta.drop(de_ptr.as_ptr().offset(de_offset) as *mut u8);
                    }
                }
            } else {
                chunksets_to_add.push(i);
            }
        }
        for tag_value_idx in chunksets_to_add {
            let chunkset_idx = this.archetype.alloc_chunk_set(|tags| {
                for (tag_idx, (tag_type, _)) in tag_types.iter().enumerate() {
                    unsafe {
                        let (de_ptr, stride, _) = deserialized_tags[tag_idx].data_raw();
                        let de_offset = (tag_value_idx * stride) as isize;
                        let world_storage = tags
                            .get_mut(*tag_type)
                            .expect("tag_storage should be present after allocating chunk_set");
                        world_storage.push_raw(de_ptr.as_ptr().offset(de_offset));
                    }
                }
            });
            chunkset_map.insert(tag_value_idx, chunkset_idx);
        }
        for tag in deserialized_tags.drain(0..) {
            tag.forget_data();
        }
        if num_tag_values == 0 {
            let chunkset_idx = this.archetype.alloc_chunk_set(|_| {});
            chunkset_map.insert(0, chunkset_idx);
        }
        Ok(chunkset_map)
    }
}

impl<'de, 'a, 'b, WD: WorldDeserializer> Visitor<'de> for TagsDeserializer<'a, 'b, WD> {
    type Value = (Vec<TagStorage>, Self);

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("sequence of objects")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let tag_types = self.archetype.description().tags();
        let mut deserialized_tags = Vec::new();
        for (tag_type, tag_meta) in tag_types {
            let mut tag_storage = TagStorage::new(*tag_meta);
            if seq
                .next_element_seed(TagStorageDeserializer {
                    user: self.user,
                    tag_storage: &mut tag_storage,
                    tag_type: &tag_type,
                    tag_meta: &tag_meta,
                })?
                .is_none()
            {
                break;
            }
            deserialized_tags.push(tag_storage);
        }
        Ok((deserialized_tags, self))
    }
}

struct TagStorageDeserializer<'a, 'b, WD: WorldDeserializer> {
    user: &'b WD,
    tag_storage: &'a mut TagStorage,
    tag_type: &'a TagTypeId,
    tag_meta: &'a TagMeta,
}
impl<'de, 'a, 'b, WD: WorldDeserializer> DeserializeSeed<'de>
    for TagStorageDeserializer<'a, 'b, WD>
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        self.user
            .deserialize_tags(deserializer, self.tag_type, self.tag_meta, self.tag_storage)?;
        Ok(())
    }
}

struct ChunkSetDeserializer<'a, 'b, WD: WorldDeserializer> {
    user: &'b WD,
    world: &'a mut World,
    archetype_idx: usize,
    chunkset_map: &'a ChunkSetMapping,
}
impl<'de, 'a, 'b, WD: WorldDeserializer> DeserializeSeed<'de> for ChunkSetDeserializer<'a, 'b, WD> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, 'a, 'b, WD: WorldDeserializer> Visitor<'de> for ChunkSetDeserializer<'a, 'b, WD> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("sequence of objects")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        for idx in 0.. {
            let chunkset_idx = self.chunkset_map.get(&idx).cloned();
            if seq
                .next_element_seed(ChunkListDeserializer {
                    user: self.user,
                    world: self.world,
                    archetype_idx: self.archetype_idx,
                    chunkset_idx,
                })?
                .is_none()
            {
                break;
            }
        }
        Ok(())
    }
}

struct ChunkListDeserializer<'a, 'b, WD: WorldDeserializer> {
    user: &'b WD,
    world: &'a mut World,
    archetype_idx: usize,
    chunkset_idx: Option<usize>,
}
impl<'de, 'a, 'b, WD: WorldDeserializer> DeserializeSeed<'de>
    for ChunkListDeserializer<'a, 'b, WD>
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, 'a, 'b, WD: WorldDeserializer> Visitor<'de> for ChunkListDeserializer<'a, 'b, WD> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("sequence of struct Chunk")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        loop {
            if seq
                .next_element_seed(ChunkDeserializer {
                    user: self.user,
                    world: self.world,
                    archetype_idx: self.archetype_idx,
                    chunkset_idx: self.chunkset_idx.expect("expected chunkset_idx"),
                })?
                .is_none()
            {
                break;
            }
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug)]
#[serde(field_identifier, rename_all = "lowercase")]
enum ChunkField {
    Entities,
    Components,
}
struct ChunkDeserializer<'a, 'b, WD: WorldDeserializer> {
    user: &'b WD,
    world: &'a mut World,
    archetype_idx: usize,
    chunkset_idx: usize,
}
impl<'de, 'a, 'b, WD: WorldDeserializer> DeserializeSeed<'de> for ChunkDeserializer<'a, 'b, WD> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("Chunk", &["entities", "components"], self)
    }
}

impl<'de, 'a, 'b, WD: WorldDeserializer> Visitor<'de> for ChunkDeserializer<'a, 'b, WD> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct Chunk")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: de::SeqAccess<'de>,
    {
        let chunk_ranges = seq.next_element_seed(EntitiesDeserializer {
            user: self.user,
            world: self.world,
            archetype_idx: self.archetype_idx,
            chunkset_idx: self.chunkset_idx,
        })?;
        seq.next_element_seed(ComponentsDeserializer {
            user: self.user,
            world: self.world,
            archetype_idx: self.archetype_idx,
            chunkset_idx: self.chunkset_idx,
            chunk_ranges: chunk_ranges
                .as_ref()
                .expect("expected entities before components"),
        })?;
        Ok(())
    }
    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        let mut chunk_ranges = None;
        while let Some(key) = map.next_key()? {
            match key {
                ChunkField::Entities => {
                    chunk_ranges = Some(map.next_value_seed(EntitiesDeserializer {
                        user: self.user,
                        world: self.world,
                        archetype_idx: self.archetype_idx,
                        chunkset_idx: self.chunkset_idx,
                    })?);
                }
                ChunkField::Components => {
                    map.next_value_seed(ComponentsDeserializer {
                        user: self.user,
                        world: self.world,
                        archetype_idx: self.archetype_idx,
                        chunkset_idx: self.chunkset_idx,
                        chunk_ranges: chunk_ranges
                            .as_ref()
                            .expect("expected entities before components"),
                    })?;
                }
            }
        }
        Ok(())
    }
}

struct EntitiesDeserializer<'a, 'b, WD: WorldDeserializer> {
    user: &'b WD,
    world: &'a mut World,
    archetype_idx: usize,
    chunkset_idx: usize,
}
impl<'de, 'a, 'b, WD: WorldDeserializer> DeserializeSeed<'de> for EntitiesDeserializer<'a, 'b, WD> {
    type Value = Vec<(usize, usize)>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let mut entities = Vec::new();
        self.user.deserialize_entities(
            deserializer,
            &self.world.entity_allocator,
            &mut entities,
        )?;
        let archetype = &mut self.world.storage_mut().archetypes_mut()[self.archetype_idx];
        let mut chunk_ranges = Vec::new();
        let mut chunk_idx = archetype.get_free_chunk(self.chunkset_idx, entities.len());
        let mut entities_in_chunk = 0;
        for entity in entities {
            let chunk = {
                let chunkset = &mut archetype.chunksets_mut()[self.chunkset_idx];
                let chunk = &mut chunkset[chunk_idx];
                if chunk.is_full() {
                    chunk_ranges.push((chunk_idx, entities_in_chunk));
                    chunk_idx = archetype.get_free_chunk(self.chunkset_idx, 1);
                    let chunkset = &mut archetype.chunksets_mut()[self.chunkset_idx];
                    &mut chunkset[chunk_idx]
                } else {
                    chunk
                }
            };
            chunk.writer().get().0.push(entity);
            entities_in_chunk += 1;
        }
        if entities_in_chunk > 0 {
            chunk_ranges.push((chunk_idx, entities_in_chunk));
        }
        Ok(chunk_ranges)
    }
}
struct ComponentsDeserializer<'a, 'b, WD: WorldDeserializer> {
    user: &'b WD,
    world: &'a mut World,
    archetype_idx: usize,
    chunkset_idx: usize,
    chunk_ranges: &'a Vec<(usize, usize)>,
}
impl<'de, 'a, 'b, WD: WorldDeserializer> DeserializeSeed<'de>
    for ComponentsDeserializer<'a, 'b, WD>
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, 'a, 'b, WD: WorldDeserializer> Visitor<'de> for ComponentsDeserializer<'a, 'b, WD> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("sequence of objects")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let archetype = &mut self.world.storage_mut().archetypes_mut()[self.archetype_idx];
        for idx in 0..archetype.description().components().len() {
            let desc = archetype.description();
            let (comp_type, comp_meta) = desc.components()[idx];
            let mut chunkset = &mut archetype.chunksets_mut()[self.chunkset_idx];
            if seq
                .next_element_seed(ComponentDataDeserializer {
                    user: self.user,
                    comp_type: &comp_type,
                    comp_meta: &comp_meta,
                    chunkset: &mut chunkset,
                    chunk_ranges: self.chunk_ranges,
                })?
                .is_none()
            {
                break;
            }
        }
        Ok(())
    }
}

struct ComponentDataDeserializer<'a, 'b, WD: WorldDeserializer> {
    user: &'b WD,
    comp_type: &'a ComponentTypeId,
    comp_meta: &'a ComponentMeta,
    chunkset: &'a mut Chunkset,
    chunk_ranges: &'a Vec<(usize, usize)>,
}
impl<'de, 'a, 'b, WD: WorldDeserializer> DeserializeSeed<'de>
    for ComponentDataDeserializer<'a, 'b, WD>
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let mut range_idx = 0;
        self.user.deserialize_components(
            deserializer,
            self.comp_type,
            self.comp_meta,
            &mut || -> Option<(NonNull<u8>, usize)> {
                self.chunk_ranges.get(range_idx).map(|chunk_range| {
                    range_idx += 1;
                    let chunk = &mut self.chunkset[chunk_range.0];
                    unsafe {
                        let comp_storage = (&mut *chunk.writer().get().1.get())
                            .get_mut(*self.comp_type)
                            .expect(
                                "expected ComponentResourceSet when deserializing component data",
                            );
                        (
                            comp_storage.writer().reserve_raw(chunk_range.1),
                            chunk_range.1,
                        )
                    }
                })
            },
        )?;
        Ok(())
    }
}
