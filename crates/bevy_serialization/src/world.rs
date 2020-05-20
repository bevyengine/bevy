// adapted from https://github.com/TomGillen/legion/blob/master/examples/serde.rs

use legion::{
    entity::{EntityIndex, GuidEntityAllocator},
    prelude::*,
    storage::{
        ArchetypeDescription, ComponentMeta, ComponentResourceSet, ComponentTypeId, TagMeta,
        TagStorage, TagTypeId,
    },
};
use serde::{
    de::{self, DeserializeSeed, IgnoredAny, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{
    cell::RefCell, collections::HashMap, iter::FromIterator, marker::PhantomData, num::Wrapping,
    ptr::NonNull,
};

struct ComponentDeserializer<'de, T: Deserialize<'de>> {
    ptr: *mut T,
    _marker: PhantomData<&'de T>,
}

impl<'de, T: Deserialize<'de> + 'static> DeserializeSeed<'de> for ComponentDeserializer<'de, T> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = <T as Deserialize<'de>>::deserialize(deserializer)?;
        unsafe {
            std::ptr::write(self.ptr, value);
        }
        Ok(())
    }
}

struct ComponentSeqDeserializer<'a, T> {
    get_next_storage_fn: &'a mut dyn FnMut() -> Option<(NonNull<u8>, usize)>,
    _marker: PhantomData<T>,
}

impl<'de, 'a, T: for<'b> Deserialize<'b> + 'static> DeserializeSeed<'de>
    for ComponentSeqDeserializer<'a, T>
{
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}
impl<'de, 'a, T: for<'b> Deserialize<'b> + 'static> Visitor<'de>
    for ComponentSeqDeserializer<'a, T>
{
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("sequence of objects")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let size = seq.size_hint();
        for _ in 0..size.unwrap_or(std::usize::MAX) {
            match (self.get_next_storage_fn)() {
                Some((storage_ptr, storage_len)) => {
                    let storage_ptr = storage_ptr.as_ptr() as *mut T;
                    for idx in 0..storage_len {
                        let element_ptr = unsafe { storage_ptr.offset(idx as isize) };

                        if let None = seq.next_element_seed(ComponentDeserializer {
                            ptr: element_ptr,
                            _marker: PhantomData,
                        })? {
                            panic!(
                                "expected {} elements in chunk but only {} found",
                                storage_len, idx
                            );
                        }
                    }
                }
                None => {
                    if let Some(_) = seq.next_element::<IgnoredAny>()? {
                        panic!("unexpected element when there was no storage space available");
                    } else {
                        // No more elements and no more storage - that's what we want!
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct ComponentRegistration {
    pub ty: ComponentTypeId,
    pub comp_serialize_fn: fn(&ComponentResourceSet, &mut dyn FnMut(&dyn erased_serde::Serialize)),
    pub individual_comp_serialize_fn:
        fn(&ComponentResourceSet, usize, &mut dyn FnMut(&dyn erased_serde::Serialize)),
    pub comp_deserialize_fn: fn(
        deserializer: &mut dyn erased_serde::Deserializer,
        get_next_storage_fn: &mut dyn FnMut() -> Option<(NonNull<u8>, usize)>,
    ) -> Result<(), erased_serde::Error>,
    pub register_comp_fn: fn(&mut ArchetypeDescription),
}

impl ComponentRegistration {
    pub fn of<T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static>() -> Self {
        Self {
            ty: ComponentTypeId::of::<T>(),
            comp_serialize_fn: |comp_storage, serialize_fn| {
                // it's safe because we know this is the correct type due to lookup
                let slice = unsafe { comp_storage.data_slice::<T>() };
                serialize_fn(&*slice);
            },
            individual_comp_serialize_fn: |comp_storage, index: usize, serialize_fn| {
                // it's safe because we know this is the correct type due to lookup
                let slice = unsafe { comp_storage.data_slice::<T>() };
                serialize_fn(&slice[index]);
            },
            comp_deserialize_fn: |deserializer, get_next_storage_fn| {
                let comp_seq_deser = ComponentSeqDeserializer::<T> {
                    get_next_storage_fn,
                    _marker: PhantomData,
                };
                comp_seq_deser.deserialize(deserializer)?;
                Ok(())
            },
            register_comp_fn: |desc| {
                desc.register_component::<T>();
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SerializedArchetypeDescription {
    tag_types: Vec<String>,
    component_types: Vec<String>,
}

pub struct SerializeImpl {
    pub comp_types: HashMap<String, ComponentRegistration>,
}

impl SerializeImpl {
    pub fn new(component_registrations: &[ComponentRegistration]) -> Self {
        SerializeImpl {
            comp_types: HashMap::from_iter(
                component_registrations
                    .iter()
                    .map(|reg| (reg.ty.0.to_string(), reg.clone())),
            ),
        }
    }
}

impl legion::serialize::ser::WorldSerializer for SerializeImpl {
    fn can_serialize_tag(&self, _ty: &TagTypeId, _meta: &TagMeta) -> bool {
        false
    }
    fn can_serialize_component(&self, ty: &ComponentTypeId, _meta: &ComponentMeta) -> bool {
        self.comp_types.get(ty.0).is_some()
    }
    fn serialize_archetype_description<S: Serializer>(
        &self,
        serializer: S,
        archetype_desc: &ArchetypeDescription,
    ) -> Result<S::Ok, S::Error> {
        let tags_to_serialize = archetype_desc
            .tags()
            .iter()
            .map(|(tag_type_id, _)| tag_type_id.0.to_string())
            .collect::<Vec<_>>();
        let components_to_serialize = archetype_desc
            .components()
            .iter()
            .map(|(component_type_id, _)| component_type_id.0.to_string())
            .collect::<Vec<_>>();
        SerializedArchetypeDescription {
            tag_types: tags_to_serialize,
            component_types: components_to_serialize,
        }
        .serialize(serializer)
    }
    fn serialize_components<S: Serializer>(
        &self,
        serializer: S,
        component_type: &ComponentTypeId,
        _component_meta: &ComponentMeta,
        components: &ComponentResourceSet,
    ) -> Result<S::Ok, S::Error> {
        if let Some(reg) = self.comp_types.get(component_type.0) {
            let result = RefCell::new(None);
            let serializer = RefCell::new(Some(serializer));
            {
                let mut result_ref = result.borrow_mut();
                (reg.comp_serialize_fn)(components, &mut |serialize| {
                    result_ref.replace(erased_serde::serialize(
                        serialize,
                        serializer.borrow_mut().take().unwrap(),
                    ));
                });
            }
            return result.borrow_mut().take().unwrap();
        }
        panic!(
            "received unserializable type {:?}, this should be filtered by can_serialize",
            component_type
        );
    }
    fn serialize_tags<S: Serializer>(
        &self,
        _serializer: S,
        tag_type: &TagTypeId,
        _tag_meta: &TagMeta,
        _tags: &TagStorage,
    ) -> Result<S::Ok, S::Error> {
        panic!(
            "received unserializable type {:?}, this should be filtered by can_serialize",
            tag_type
        );
    }
    fn serialize_entities<S: Serializer>(
        &self,
        serializer: S,
        entities: &[Entity],
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(entities.iter().map(|e| e.index()))
    }
}

pub struct DeserializeImpl<'a> {
    pub comp_types: &'a HashMap<String, ComponentRegistration>,
}

impl<'a> DeserializeImpl<'a> {
    pub fn new(component_types: &'a HashMap<String, ComponentRegistration>) -> Self {
        DeserializeImpl {
            comp_types: component_types,
        }
    }
}

impl<'a> legion::serialize::de::WorldDeserializer for DeserializeImpl<'a> {
    fn deserialize_archetype_description<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
    ) -> Result<ArchetypeDescription, <D as Deserializer<'de>>::Error> {
        let serialized_desc =
            <SerializedArchetypeDescription as Deserialize>::deserialize(deserializer)?;
        let mut desc = ArchetypeDescription::default();

        for comp in serialized_desc.component_types {
            if let Some(reg) = self.comp_types.get(&comp) {
                (reg.register_comp_fn)(&mut desc);
            }
        }
        Ok(desc)
    }
    fn deserialize_components<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
        component_type: &ComponentTypeId,
        _component_meta: &ComponentMeta,
        get_next_storage_fn: &mut dyn FnMut() -> Option<(NonNull<u8>, usize)>,
    ) -> Result<(), <D as Deserializer<'de>>::Error> {
        if let Some(reg) = self.comp_types.get(component_type.0) {
            let mut erased = erased_serde::Deserializer::erase(deserializer);
            (reg.comp_deserialize_fn)(&mut erased, get_next_storage_fn)
                .map_err(<<D as serde::Deserializer<'de>>::Error as serde::de::Error>::custom)?;
        } else {
            <IgnoredAny>::deserialize(deserializer)?;
        }
        Ok(())
    }
    fn deserialize_tags<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
        _tag_type: &TagTypeId,
        _tag_meta: &TagMeta,
        _tags: &mut TagStorage,
    ) -> Result<(), <D as Deserializer<'de>>::Error> {
        <IgnoredAny>::deserialize(deserializer)?;
        Ok(())
    }
    fn deserialize_entities<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
        entity_allocator: &GuidEntityAllocator,
        entities: &mut Vec<Entity>,
    ) -> Result<(), <D as Deserializer<'de>>::Error> {
        let entity_indices = <Vec<EntityIndex> as Deserialize>::deserialize(deserializer)?;
        entity_allocator.push_next_ids(entity_indices.iter().map(|i| Entity::new(*i, Wrapping(0))));
        for _index in entity_indices {
            let entity = entity_allocator.create_entity();
            entities.push(entity);
        }
        Ok(())
    }
}
