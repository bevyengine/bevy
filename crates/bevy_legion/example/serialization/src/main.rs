use legion::{
    entity::EntityAllocator,
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
use std::{any::TypeId, cell::RefCell, collections::HashMap, marker::PhantomData, ptr::NonNull};
use type_uuid::TypeUuid;

#[derive(TypeUuid, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[uuid = "5fd8256d-db36-4fe2-8211-c7b3446e1927"]
struct Pos(f32, f32, f32);
#[derive(TypeUuid, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[uuid = "14dec17f-ae14-40a3-8e44-e487fc423287"]
struct Vel(f32, f32, f32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct Unregistered(f32, f32, f32);

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
                        let element_ptr = unsafe { storage_ptr.add(idx) };

                        if seq
                            .next_element_seed(ComponentDeserializer {
                                ptr: element_ptr,
                                _marker: PhantomData,
                            })?
                            .is_none()
                        {
                            panic!(
                                "expected {} elements in chunk but only {} found",
                                storage_len, idx
                            );
                        }
                    }
                }
                None => {
                    if seq.next_element::<IgnoredAny>()?.is_some() {
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
struct TagRegistration {
    uuid: type_uuid::Bytes,
    ty: TypeId,
    tag_serialize_fn: fn(&TagStorage, &mut dyn FnMut(&dyn erased_serde::Serialize)),
    tag_deserialize_fn: fn(
        deserializer: &mut dyn erased_serde::Deserializer,
        &mut TagStorage,
    ) -> Result<(), erased_serde::Error>,
    register_tag_fn: fn(&mut ArchetypeDescription),
}

impl TagRegistration {
    fn of<
        T: TypeUuid
            + Serialize
            + for<'de> Deserialize<'de>
            + PartialEq
            + Clone
            + Send
            + Sync
            + 'static,
    >() -> Self {
        Self {
            uuid: T::UUID,
            ty: TypeId::of::<T>(),
            tag_serialize_fn: |tag_storage, serialize_fn| {
                // it's safe because we know this is the correct type due to lookup
                let slice = unsafe { tag_storage.data_slice::<T>() };
                serialize_fn(&&*slice);
            },
            tag_deserialize_fn: |deserializer, tag_storage| {
                // TODO implement visitor to avoid allocation of Vec
                let tag_vec = <Vec<T> as Deserialize>::deserialize(deserializer)?;
                for tag in tag_vec {
                    // Tag types should line up, making this safe
                    unsafe {
                        tag_storage.push(tag);
                    }
                }
                Ok(())
            },
            register_tag_fn: |desc| {
                desc.register_tag::<T>();
            },
        }
    }
}

#[derive(Clone)]
struct ComponentRegistration {
    uuid: type_uuid::Bytes,
    ty: TypeId,
    comp_serialize_fn: fn(&ComponentResourceSet, &mut dyn FnMut(&dyn erased_serde::Serialize)),
    comp_deserialize_fn: fn(
        deserializer: &mut dyn erased_serde::Deserializer,
        get_next_storage_fn: &mut dyn FnMut() -> Option<(NonNull<u8>, usize)>,
    ) -> Result<(), erased_serde::Error>,
    register_comp_fn: fn(&mut ArchetypeDescription),
}

impl ComponentRegistration {
    fn of<T: TypeUuid + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static>() -> Self {
        Self {
            uuid: T::UUID,
            ty: TypeId::of::<T>(),
            comp_serialize_fn: |comp_storage, serialize_fn| {
                // it's safe because we know this is the correct type due to lookup
                let slice = unsafe { comp_storage.data_slice::<T>() };
                serialize_fn(&*slice);
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
    tag_types: Vec<type_uuid::Bytes>,
    component_types: Vec<type_uuid::Bytes>,
}

struct SerializeImpl {
    tag_types: HashMap<TypeId, TagRegistration>,
    comp_types: HashMap<TypeId, ComponentRegistration>,
    entity_map: RefCell<HashMap<Entity, uuid::Bytes>>,
}
impl legion::serialize::ser::WorldSerializer for SerializeImpl {
    fn can_serialize_tag(&self, ty: &TagTypeId, _meta: &TagMeta) -> bool {
        self.tag_types.get(&ty.0).is_some()
    }
    fn can_serialize_component(&self, ty: &ComponentTypeId, _meta: &ComponentMeta) -> bool {
        self.comp_types.get(&ty.0).is_some()
    }
    fn serialize_archetype_description<S: Serializer>(
        &self,
        serializer: S,
        archetype_desc: &ArchetypeDescription,
    ) -> Result<S::Ok, S::Error> {
        let tags_to_serialize = archetype_desc
            .tags()
            .iter()
            .filter_map(|(ty, _)| self.tag_types.get(&ty.0))
            .map(|reg| reg.uuid)
            .collect::<Vec<_>>();
        let components_to_serialize = archetype_desc
            .components()
            .iter()
            .filter_map(|(ty, _)| self.comp_types.get(&ty.0))
            .map(|reg| reg.uuid)
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
        if let Some(reg) = self.comp_types.get(&component_type.0) {
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
        serializer: S,
        tag_type: &TagTypeId,
        _tag_meta: &TagMeta,
        tags: &TagStorage,
    ) -> Result<S::Ok, S::Error> {
        if let Some(reg) = self.tag_types.get(&tag_type.0) {
            let result = RefCell::new(None);
            let serializer = RefCell::new(Some(serializer));
            {
                let mut result_ref = result.borrow_mut();
                (reg.tag_serialize_fn)(tags, &mut |serialize| {
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
            tag_type
        );
    }
    fn serialize_entities<S: Serializer>(
        &self,
        serializer: S,
        entities: &[Entity],
    ) -> Result<S::Ok, S::Error> {
        let mut uuid_map = self.entity_map.borrow_mut();
        serializer.collect_seq(entities.iter().map(|e| {
            *uuid_map
                .entry(*e)
                .or_insert_with(|| *uuid::Uuid::new_v4().as_bytes())
        }))
    }
}

struct DeserializeImpl {
    tag_types: HashMap<TypeId, TagRegistration>,
    comp_types: HashMap<TypeId, ComponentRegistration>,
    tag_types_by_uuid: HashMap<type_uuid::Bytes, TagRegistration>,
    comp_types_by_uuid: HashMap<type_uuid::Bytes, ComponentRegistration>,
    entity_map: RefCell<HashMap<uuid::Bytes, Entity>>,
}
impl legion::serialize::de::WorldDeserializer for DeserializeImpl {
    fn deserialize_archetype_description<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
    ) -> Result<ArchetypeDescription, <D as Deserializer<'de>>::Error> {
        let serialized_desc =
            <SerializedArchetypeDescription as Deserialize>::deserialize(deserializer)?;
        let mut desc = ArchetypeDescription::default();
        for tag in serialized_desc.tag_types {
            if let Some(reg) = self.tag_types_by_uuid.get(&tag) {
                (reg.register_tag_fn)(&mut desc);
            }
        }
        for comp in serialized_desc.component_types {
            if let Some(reg) = self.comp_types_by_uuid.get(&comp) {
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
        if let Some(reg) = self.comp_types.get(&component_type.0) {
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
        tag_type: &TagTypeId,
        _tag_meta: &TagMeta,
        tags: &mut TagStorage,
    ) -> Result<(), <D as Deserializer<'de>>::Error> {
        if let Some(reg) = self.tag_types.get(&tag_type.0) {
            let mut erased = erased_serde::Deserializer::erase(deserializer);
            (reg.tag_deserialize_fn)(&mut erased, tags)
                .map_err(<<D as serde::Deserializer<'de>>::Error as serde::de::Error>::custom)?;
        } else {
            <IgnoredAny>::deserialize(deserializer)?;
        }
        Ok(())
    }
    fn deserialize_entities<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
        entity_allocator: &EntityAllocator,
        entities: &mut Vec<Entity>,
    ) -> Result<(), <D as Deserializer<'de>>::Error> {
        let entity_uuids = <Vec<uuid::Bytes> as Deserialize>::deserialize(deserializer)?;
        let mut entity_map = self.entity_map.borrow_mut();
        for id in entity_uuids {
            let entity = entity_allocator.create_entity();
            entity_map.insert(id, entity);
            entities.push(entity);
        }
        Ok(())
    }
}

fn main() {
    // create world
    let universe = Universe::new();
    let mut world = universe.create_world();

    // Pos and Vel are both serializable, so all components in this chunkset will be serialized
    world.insert(
        (),
        vec![
            (Pos(1., 2., 3.), Vel(1., 2., 3.)),
            (Pos(1., 2., 3.), Vel(1., 2., 3.)),
            (Pos(1., 2., 3.), Vel(1., 2., 3.)),
            (Pos(1., 2., 3.), Vel(1., 2., 3.)),
        ],
    );
    // Unserializable components are not serialized, so only the Pos components should be serialized in this chunkset
    for _ in 0..1000 {
        world.insert(
            (Pos(4., 5., 6.), Unregistered(4., 5., 6.)),
            vec![
                (Pos(1., 2., 3.), Unregistered(4., 5., 6.)),
                (Pos(1., 2., 3.), Unregistered(4., 5., 6.)),
                (Pos(1., 2., 3.), Unregistered(4., 5., 6.)),
                (Pos(1., 2., 3.), Unregistered(4., 5., 6.)),
            ],
        );
    }
    // Entities with no serializable components are not serialized, so this entire chunkset should be skipped in the output
    world.insert(
        (Unregistered(4., 5., 6.),),
        vec![(Unregistered(4., 5., 6.),), (Unregistered(4., 5., 6.),)],
    );

    let comp_registrations = [
        ComponentRegistration::of::<Pos>(),
        ComponentRegistration::of::<Vel>(),
    ];
    let tag_registrations = [TagRegistration::of::<Pos>(), TagRegistration::of::<Vel>()];

    use std::iter::FromIterator;
    let ser_helper = SerializeImpl {
        comp_types: HashMap::from_iter(comp_registrations.iter().map(|reg| (reg.ty, reg.clone()))),
        tag_types: HashMap::from_iter(tag_registrations.iter().map(|reg| (reg.ty, reg.clone()))),
        entity_map: RefCell::new(HashMap::new()),
    };

    let serializable = legion::serialize::ser::serializable_world(&world, &ser_helper);
    let serialized_data = serde_json::to_string(&serializable).unwrap();
    let de_helper = DeserializeImpl {
        tag_types_by_uuid: HashMap::from_iter(
            ser_helper
                .tag_types
                .iter()
                .map(|reg| (reg.1.uuid, reg.1.clone())),
        ),
        comp_types_by_uuid: HashMap::from_iter(
            ser_helper
                .comp_types
                .iter()
                .map(|reg| (reg.1.uuid, reg.1.clone())),
        ),
        tag_types: ser_helper.tag_types,
        comp_types: ser_helper.comp_types,
        // re-use the entity-uuid mapping
        entity_map: RefCell::new(HashMap::from_iter(
            ser_helper
                .entity_map
                .into_inner()
                .into_iter()
                .map(|(e, uuid)| (uuid, e)),
        )),
    };
    let mut deserialized_world = universe.create_world();
    let mut deserializer = serde_json::Deserializer::from_str(&serialized_data);
    legion::serialize::de::deserialize(&mut deserialized_world, &de_helper, &mut deserializer)
        .unwrap();
    let ser_helper = SerializeImpl {
        tag_types: de_helper.tag_types,
        comp_types: de_helper.comp_types,
        // re-use the entity-uuid mapping
        entity_map: RefCell::new(HashMap::from_iter(
            de_helper
                .entity_map
                .into_inner()
                .into_iter()
                .map(|(uuid, e)| (e, uuid)),
        )),
    };
    let serializable = legion::serialize::ser::serializable_world(&deserialized_world, &ser_helper);
    let roundtrip_data = serde_json::to_string(&serializable).unwrap();
    assert_eq!(roundtrip_data, serialized_data);
}
