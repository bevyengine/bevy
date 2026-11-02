//! Serde helpers for the inspection types, used via `#[serde(with = "...")]`.

use bevy_ecs::{query::SpawnDetails, reflect::AppTypeRegistry, world::World};
use bevy_reflect::{serde::TypedReflectSerializer, PartialReflect};
use serde::{ser::SerializeStruct, Serializer};

/// Serializes a [`ComponentId`](bevy_ecs::component::ComponentId) as its integer index.
pub mod component_id {
    use bevy_ecs::component::ComponentId;
    use serde::{Deserialize, Serialize};

    /// Serializes a [`ComponentId`] as its index.
    pub fn serialize<S>(id: &ComponentId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        id.index().serialize(serializer)
    }

    /// Deserializes a [`ComponentId`] from its index.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ComponentId, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let index = usize::deserialize(deserializer)?;
        Ok(ComponentId::new(index))
    }
}

/// Serializes an [`ArchetypeId`](bevy_ecs::archetype::ArchetypeId) as its integer index.
pub mod archetype_id {
    use bevy_ecs::archetype::ArchetypeId;
    use serde::{Deserialize, Serialize};

    /// Serializes an [`ArchetypeId`] as its index.
    pub fn serialize<S>(id: &ArchetypeId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        id.index().serialize(serializer)
    }

    /// Deserializes an [`ArchetypeId`] from its index.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ArchetypeId, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let index = usize::deserialize(deserializer)?;
        Ok(ArchetypeId::new(index))
    }
}

/// Serializes a slice of [`ComponentId`](bevy_ecs::component::ComponentId)s as a sequence of indexes.
pub mod slice_component_id {
    use bevy_ecs::component::ComponentId;
    use serde::{ser::SerializeSeq, Deserialize};

    /// Serializes a slice of [`ComponentId`]s as a sequence of indexes.
    pub fn serialize<S>(ids: &[ComponentId], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(ids.len()))?;
        for id in ids {
            seq.serialize_element(&id.index())?;
        }
        seq.end()
    }

    /// Deserializes a sequence of indexes into a `Vec<ComponentId>`.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<ComponentId>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let indexes: Vec<usize> = Vec::deserialize(deserializer)?;
        Ok(indexes.into_iter().map(ComponentId::new).collect())
    }
}

/// Serializes a [`DebugName`](bevy_utils::prelude::DebugName) as a string.
pub mod debug_name {
    use bevy_utils::prelude::DebugName;
    use serde::{Deserialize, Serialize};

    /// Serializes a [`DebugName`] as a string.
    pub fn serialize<S>(name: &DebugName, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        name.to_string().serialize(serializer)
    }

    /// Deserializes a [`DebugName`] from a string.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DebugName, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Ok(DebugName::owned(name))
    }
}

/// Serializes an `Option<Vec<DebugName>>` as an optional sequence of strings.
pub mod option_vec_debug_name {
    use bevy_utils::prelude::DebugName;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Serializes an `Option<Vec<DebugName>>` as an optional sequence of strings.
    pub fn serialize<S>(value: &Option<Vec<DebugName>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(debug_names) => {
                let strings: Vec<String> = debug_names.iter().map(DebugName::to_string).collect();
                serializer.serialize_some(&strings)
            }
            None => serializer.serialize_none(),
        }
    }

    /// Deserializes an `Option<Vec<DebugName>>` from an optional sequence of strings.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<DebugName>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let optional_strings: Option<Vec<String>> = Option::deserialize(deserializer)?;
        Ok(optional_strings.map(|strings| strings.into_iter().map(DebugName::owned).collect()))
    }
}

/// Serializes a [`StorageType`](bevy_ecs::component::StorageType) as its variant name.
pub mod storage_type {
    use bevy_ecs::component::StorageType;
    use serde::{Deserialize, Serialize};

    /// Serializes a [`StorageType`] as its variant name.
    pub fn serialize<S>(storage_type: &StorageType, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let name = match storage_type {
            StorageType::Table => "Table",
            StorageType::SparseSet => "SparseSet",
        };
        name.serialize(serializer)
    }

    /// Deserializes a [`StorageType`] from its variant name.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<StorageType, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        match name.as_str() {
            "Table" => Ok(StorageType::Table),
            "SparseSet" => Ok(StorageType::SparseSet),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["Table", "SparseSet"],
            )),
        }
    }
}

/// Serializes a `HashMap<ComponentId, ComponentTypeMetadata>` with integer indexes as keys.
pub mod hash_map_component_id_component_type_metadata {
    use crate::inspection::component_inspection::ComponentTypeMetadata;
    use bevy_ecs::component::ComponentId;
    use bevy_platform::collections::HashMap;
    use serde::{ser::SerializeMap, Deserialize, Deserializer, Serializer};

    /// Serializes the map, using each key's index as the entry key.
    pub fn serialize<S>(
        map: &HashMap<ComponentId, ComponentTypeMetadata>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut entries = serializer.serialize_map(Some(map.len()))?;
        for (key, value) in map {
            entries.serialize_entry(&key.index(), value)?;
        }
        entries.end()
    }

    /// Deserializes a map of indexes into a `HashMap<ComponentId, ComponentTypeMetadata>`.
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<ComponentId, ComponentTypeMetadata>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let index_to_metadata: HashMap<usize, ComponentTypeMetadata> =
            HashMap::deserialize(deserializer)?;
        Ok(index_to_metadata
            .into_iter()
            .map(|(key, value)| (ComponentId::new(key), value))
            .collect())
    }
}

/// Serializes [`SpawnDetails`] as a struct with a `tick` and an optional `location`.
pub fn serialize_spawn_details<S>(
    spawn_details: &SpawnDetails,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let location = spawn_details
        .spawned_by()
        .into_option()
        .map(ToString::to_string);

    let mut state = serializer.serialize_struct("SpawnDetails", 2)?;
    state.serialize_field("tick", &spawn_details.spawn_tick().get())?;
    state.serialize_field("location", &location)?;
    state.end()
}

/// Serializes an `Option<SpawnDetails>`. [`SpawnDetails`] cannot be deserialized.
pub fn serialize_option_spawn_details<S>(
    spawn_details: &Option<SpawnDetails>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    struct Wrapper<'a>(&'a SpawnDetails);

    impl serde::Serialize for Wrapper<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serialize_spawn_details(self.0, serializer)
        }
    }

    match spawn_details {
        Some(spawn_details) => serializer.serialize_some(&Wrapper(spawn_details)),
        None => serializer.serialize_none(),
    }
}

/// Serializes a reflected value to JSON using the world's [`AppTypeRegistry`].
/// Returns [`None`] if the type is unregistered or cannot be serialized.
pub fn reflect_to_json_value(
    world: &World,
    value: &dyn PartialReflect,
) -> Option<serde_json::Value> {
    let type_registry = world.get_resource::<AppTypeRegistry>()?;
    let type_registry = type_registry.read();
    let serializer = TypedReflectSerializer::new(value, &type_registry);

    serde_json::to_value(&serializer).ok()
}

#[cfg(all(test, feature = "serialize"))]
mod tests {
    use super::*;
    use crate::inspection::{
        component_inspection::{
            ComponentInspection, ComponentInspectionSettings, ComponentMetadataMap,
            ComponentTypeMetadata,
        },
        entity_inspection::{EntityInspection, EntityInspectionSettings},
        extension_methods::WorldInspectionExtensionTrait,
        resource_inspection::{ResourceInspection, ResourceInspectionSettings},
    };
    use bevy_ecs::{
        archetype::ArchetypeId,
        component::{Component, ComponentId, StorageType},
        name::Name,
        reflect::{ReflectComponent, ReflectResource},
        resource::Resource,
    };
    use bevy_reflect::Reflect;
    use bevy_utils::prelude::DebugName;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Component, Reflect, Debug, PartialEq)]
    #[reflect(Component)]
    struct Health(u32);

    #[derive(Resource, Reflect, Debug)]
    #[reflect(Resource)]
    struct Score(u32);

    fn test_world() -> World {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        {
            let registry = world.resource_mut::<AppTypeRegistry>();
            let mut registry = registry.write();
            registry.register::<Health>();
            registry.register::<Score>();
        }
        world
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ComponentIdWrapper(#[serde(with = "component_id")] ComponentId);

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ArchetypeIdWrapper(#[serde(with = "archetype_id")] ArchetypeId);

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct SliceComponentIdWrapper(#[serde(with = "slice_component_id")] Vec<ComponentId>);

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct DebugNameWrapper(#[serde(with = "debug_name")] DebugName);

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct OptionVecDebugNameWrapper(
        #[serde(with = "option_vec_debug_name")] Option<Vec<DebugName>>,
    );

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct StorageTypeWrapper(#[serde(with = "storage_type")] StorageType);

    fn round_trip<T: Serialize + for<'de> Deserialize<'de>>(value: &T) -> T {
        let json = serde_json::to_string(value).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn component_id_round_trips_as_index() {
        let wrapper = ComponentIdWrapper(ComponentId::new(3));

        assert_eq!(serde_json::to_string(&wrapper).unwrap(), "3");
        assert_eq!(round_trip(&wrapper), wrapper);
    }

    #[test]
    fn archetype_id_round_trips_as_index() {
        let wrapper = ArchetypeIdWrapper(ArchetypeId::new(2));

        assert_eq!(serde_json::to_string(&wrapper).unwrap(), "2");
        assert_eq!(round_trip(&wrapper), wrapper);
    }

    #[test]
    fn slice_component_id_round_trips_as_indexes() {
        let wrapper = SliceComponentIdWrapper(vec![ComponentId::new(0), ComponentId::new(5)]);

        assert_eq!(serde_json::to_string(&wrapper).unwrap(), "[0,5]");
        assert_eq!(round_trip(&wrapper), wrapper);
    }

    #[test]
    fn debug_name_round_trips_as_string() {
        let wrapper = DebugNameWrapper(DebugName::owned("my_crate::Health".to_string()));

        assert_eq!(round_trip(&wrapper), wrapper);
    }

    #[test]
    fn option_vec_debug_name_round_trips() {
        let some = OptionVecDebugNameWrapper(Some(vec![DebugName::owned("Health".to_string())]));
        let none = OptionVecDebugNameWrapper(None);

        assert_eq!(round_trip(&some), some);
        assert_eq!(serde_json::to_string(&none).unwrap(), "null");
        assert_eq!(round_trip(&none), none);
    }

    #[test]
    fn storage_type_round_trips_as_name() {
        let table = StorageTypeWrapper(StorageType::Table);
        let sparse_set = StorageTypeWrapper(StorageType::SparseSet);

        assert_eq!(serde_json::to_string(&table).unwrap(), "\"Table\"");
        assert_eq!(round_trip(&table), table);
        assert_eq!(serde_json::to_string(&sparse_set).unwrap(), "\"SparseSet\"");
        assert_eq!(round_trip(&sparse_set), sparse_set);
    }

    #[test]
    fn unknown_storage_type_is_rejected() {
        let result: Result<StorageTypeWrapper, _> = serde_json::from_str("\"Rows\"");

        assert!(result.is_err());
    }

    #[test]
    fn component_metadata_map_round_trips_key_set() {
        let mut world = test_world();
        world.spawn(Health(7));

        let map = ComponentMetadataMap::generate(&world);
        let restored: ComponentMetadataMap = round_trip(&map);

        let mut original_keys: Vec<usize> = map.keys().map(|key| key.index()).collect();
        let mut restored_keys: Vec<usize> = restored.keys().map(|key| key.index()).collect();
        original_keys.sort_unstable();
        restored_keys.sort_unstable();

        assert_eq!(original_keys, restored_keys);
        assert!(!original_keys.is_empty());
    }

    #[test]
    fn component_type_metadata_round_trips() {
        let mut world = test_world();
        world.spawn(Health(7));
        let component_id = world.components().valid_component_id::<Health>().unwrap();

        let metadata = ComponentTypeMetadata::new(&world, component_id).unwrap();
        let restored: ComponentTypeMetadata = round_trip(&metadata);

        assert_eq!(restored.component_id, metadata.component_id);
        assert_eq!(restored.memory_size, metadata.memory_size);
        assert_eq!(restored.storage_type, metadata.storage_type);
        assert_eq!(restored.mutable, metadata.mutable);
        assert!(restored.type_id.is_none());
        assert!(restored.type_registration.is_none());
    }

    #[test]
    fn component_inspection_round_trips() {
        let mut world = test_world();
        let entity = world.spawn(Health(7)).id();

        let inspection = world
            .inspect_component::<Health>(entity, ComponentInspectionSettings::default())
            .unwrap();
        let restored: ComponentInspection = round_trip(&inspection);

        assert_eq!(restored.entity, entity);
        assert_eq!(restored.component_id, inspection.component_id);
        assert_eq!(restored.value, inspection.value);
        assert!(restored.reflected_value.is_none());
    }

    #[test]
    fn serialized_value_is_filled_when_requested() {
        let mut world = test_world();
        let entity = world.spawn(Health(7)).id();

        let inspection = world
            .inspect_component::<Health>(
                entity,
                ComponentInspectionSettings {
                    include_serialized_value: true,
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(inspection.serialized_value, Some(json!(7)));
    }

    #[test]
    fn serialized_value_is_none_by_default() {
        let mut world = test_world();
        let entity = world.spawn(Health(7)).id();

        let inspection = world
            .inspect_component::<Health>(entity, ComponentInspectionSettings::default())
            .unwrap();

        assert!(inspection.serialized_value.is_none());
    }

    #[test]
    fn resource_serialized_value_is_filled_when_requested() {
        let mut world = test_world();
        world.insert_resource(Score(7));

        let inspection = world
            .inspect_resource::<Score>(ResourceInspectionSettings {
                include_serialized_value: true,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(inspection.serialized_value, Some(json!(7)));
    }

    #[test]
    fn resource_inspection_round_trips() {
        let mut world = test_world();
        world.insert_resource(Score(7));

        let inspection = world
            .inspect_resource::<Score>(ResourceInspectionSettings::default())
            .unwrap();
        let restored: ResourceInspection = round_trip(&inspection);

        assert_eq!(restored.component_id, inspection.component_id);
        assert_eq!(restored.value, inspection.value);
        assert!(restored.type_id.is_none());
    }

    #[test]
    fn entity_inspection_serializes_spawn_details() {
        let mut world = test_world();
        let entity = world.spawn((Name::new("Player"), Health(7))).id();

        let inspection = world
            .inspect(entity, EntityInspectionSettings::default())
            .unwrap();
        let value = serde_json::to_value(&inspection).unwrap();

        assert!(value["spawn_details"]["tick"].is_number());
        assert!(value["spawn_details"].get("location").is_some());

        let restored: EntityInspection = serde_json::from_value(value).unwrap();
        assert_eq!(restored.entity, entity);
        assert!(restored.spawn_details.is_none());
        assert_eq!(
            restored.components.unwrap().len(),
            inspection.components.unwrap().len()
        );
    }
}
