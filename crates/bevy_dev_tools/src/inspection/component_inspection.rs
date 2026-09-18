//! Types describing components and component types, gathered by inspecting a [`World`].

use bevy_ecs::{
    component::{ComponentId, StorageType},
    entity::Entity,
    reflect::AppTypeRegistry,
    world::{FromWorld, World},
};
use bevy_platform::collections::HashMap;
use bevy_reflect::{PartialReflect, TypeRegistration};
use bevy_utils::{memory_size::MemorySize, prelude::DebugName};
use core::{
    any::TypeId,
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
};

use crate::inspection::{
    label_resolution::{LabelDefinitionPriority, LabelResolutionRegistry},
    reflection_tools::clone_incomplete,
};

/// The result of inspecting a component on an entity.
///
/// Pair this with [`ComponentTypeMetadata`] for full type information.
#[derive(Debug)]
pub struct ComponentInspection {
    /// The entity that owns the component.
    pub entity: Entity,
    /// The [`ComponentId`] of the component.
    pub component_id: ComponentId,
    /// The type name of the component.
    pub name: DebugName,
    /// The shallow size of the component in memory, excluding heap allocations.
    pub memory_size: MemorySize,
    /// The value of the component as a string, gathered via reflection.
    pub value: Option<String>,
    /// The reflected value of the component.
    pub reflected_value: Option<Box<dyn PartialReflect>>,
}

impl Clone for ComponentInspection {
    fn clone(&self) -> Self {
        let reflected_value = self
            .reflected_value
            .as_deref()
            .and_then(|reflected| clone_incomplete(reflected).ok());

        Self {
            entity: self.entity,
            component_id: self.component_id,
            name: self.name.clone(),
            memory_size: self.memory_size,
            value: self.value.clone(),
            reflected_value,
        }
    }
}

impl Display for ComponentInspection {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let shortname = self.name.shortname();

        match &self.value {
            Some(value) => write!(f, "{shortname} ({}): {value}", self.memory_size),
            None => write!(f, "{shortname} ({})", self.memory_size),
        }
    }
}

/// Metadata about a component type, in a `Send + Sync` form that can be stored and transmitted.
///
/// For the value of a component on an entity, see [`ComponentInspection`].
#[derive(Clone, Debug)]
pub struct ComponentTypeMetadata {
    /// The [`ComponentId`] of the component type.
    pub component_id: ComponentId,
    /// The type name of the component.
    pub name: DebugName,
    /// The [`TypeId`] of the component type, or `None` for dynamic types.
    pub type_id: Option<TypeId>,
    /// The minimum size in bytes of the component type, computed via [`core::alloc::Layout`].
    pub memory_size: MemorySize,
    /// The label definition priority of the component type, if it defines labels.
    pub label_definition_priority: Option<LabelDefinitionPriority>,
    /// Whether the component type is mutable while in the ECS.
    pub mutable: bool,
    /// The storage type of this component.
    pub storage_type: StorageType,
    /// Whether the underlying component type can freely be shared across threads.
    pub is_send_and_sync: bool,
    /// The components that are automatically added alongside this component.
    pub required_components: Vec<ComponentId>,
    /// The registered type information of the component, if it is reflected and registered.
    pub type_registration: Option<TypeRegistration>,
}

impl ComponentTypeMetadata {
    /// Extracts the metadata for the given component type from the world.
    pub fn new(world: &World, component_id: ComponentId) -> Result<Self, ComponentInspectionError> {
        let component_info = world.components().get_info(component_id).ok_or(
            ComponentInspectionError::ComponentIdNotRegistered(component_id),
        )?;

        let type_id = component_info.type_id();

        let type_registration = type_id.and_then(|type_id| {
            world
                .get_resource::<AppTypeRegistry>()
                .and_then(|type_registry| type_registry.read().get(type_id).cloned())
        });

        let label_definition_priority = type_id.and_then(|type_id| {
            world
                .get_resource::<LabelResolutionRegistry>()
                .and_then(|registry| registry.get_priority_by_type_id(type_id))
        });

        Ok(Self {
            component_id,
            name: component_info.name(),
            type_id,
            memory_size: MemorySize::new(component_info.layout().size()),
            label_definition_priority,
            mutable: component_info.mutable(),
            storage_type: component_info.storage_type(),
            is_send_and_sync: component_info.is_send_and_sync(),
            required_components: component_info.required_components().iter_ids().collect(),
            type_registration,
        })
    }
}

impl Display for ComponentTypeMetadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} (Size: {}, Storage: {:?})\n Type Registration: {:?}",
            self.name.shortname(),
            self.memory_size,
            self.storage_type,
            self.type_registration,
        )
    }
}

/// The result of inspecting a component type, rather than a component on an entity.
#[derive(Clone, Debug)]
pub struct ComponentTypeInspection {
    /// The number of entities that have a component of this type.
    pub entity_count: usize,
    /// Metadata about the component type.
    pub metadata: ComponentTypeMetadata,
}

impl Display for ComponentTypeInspection {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}\n Entities with this component: {}",
            self.metadata, self.entity_count
        )
    }
}

/// A cache of component type metadata, keyed by [`ComponentId`].
#[derive(Clone, Debug)]
pub struct ComponentMetadataMap {
    /// The cached metadata.
    pub map: HashMap<ComponentId, ComponentTypeMetadata>,
}

impl ComponentMetadataMap {
    /// Creates a map with metadata for every component type registered in the world.
    pub fn generate(world: &World) -> Self {
        let mut map = HashMap::new();

        for index in 0..world.components().num_registered() {
            let component_id = ComponentId::new(index);
            if let Ok(metadata) = ComponentTypeMetadata::new(world, component_id) {
                map.insert(component_id, metadata);
            }
        }

        Self { map }
    }

    /// Creates an empty [`ComponentMetadataMap`].
    pub fn empty() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Creates a map with metadata for the components currently on the given entity.
    pub fn for_entity(world: &World, entity: Entity) -> Self {
        let mut map = HashMap::new();

        if let Ok(entity_ref) = world.get_entity(entity) {
            for component_id in entity_ref.archetype().components() {
                if let Ok(metadata) = ComponentTypeMetadata::new(world, *component_id) {
                    map.insert(*component_id, metadata);
                }
            }
        }

        Self { map }
    }

    /// Adds entries for component types that are not yet in the map, leaving existing entries alone.
    pub fn update(&mut self, world: &World) {
        for index in 0..world.components().num_registered() {
            let component_id = ComponentId::new(index);
            if !self.map.contains_key(&component_id)
                && let Ok(metadata) = ComponentTypeMetadata::new(world, component_id)
            {
                self.map.insert(component_id, metadata);
            }
        }
    }

    /// Looks up a component type by its full type name.
    pub fn get_component_metadata_by_name(
        &self,
        component_name: &str,
    ) -> Option<(ComponentId, &ComponentTypeMetadata)> {
        self.map.iter().find_map(|(id, metadata)| {
            (&*metadata.name == component_name).then_some((*id, metadata))
        })
    }
}

impl Deref for ComponentMetadataMap {
    type Target = HashMap<ComponentId, ComponentTypeMetadata>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for ComponentMetadataMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl FromWorld for ComponentMetadataMap {
    fn from_world(world: &mut World) -> Self {
        ComponentMetadataMap::generate(world)
    }
}

/// An error that can occur when attempting to inspect a component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentInspectionError {
    /// The component was not found on the entity.
    ComponentNotFound(ComponentId),
    /// The component type was not registered in the world.
    ComponentNotRegistered(&'static str),
    /// The component ID provided was not registered in the world.
    ComponentIdNotRegistered(ComponentId),
}

impl Display for ComponentInspectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            ComponentInspectionError::ComponentNotFound(component_id) => {
                write!(f, "Component with {component_id:?} not found on entity")
            }
            ComponentInspectionError::ComponentNotRegistered(name) => {
                write!(f, "Component type {name} not registered in world")
            }
            ComponentInspectionError::ComponentIdNotRegistered(component_id) => {
                write!(f, "{component_id:?} not registered in world")
            }
        }
    }
}

impl core::error::Error for ComponentInspectionError {}

/// Settings for inspecting a component.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentInspectionSettings {
    /// How much detail to include when inspecting component values.
    pub detail_level: ComponentDetailLevel,
    /// Whether type paths in the value string are kept in full.
    /// When false, every `::` in the formatted value is collapsed, including inside string values.
    pub full_type_names: bool,
    /// Whether the reflected value should be stored in [`ComponentInspection`].
    pub store_reflected_value: bool,
}

impl Default for ComponentInspectionSettings {
    fn default() -> Self {
        Self {
            detail_level: ComponentDetailLevel::Values,
            full_type_names: true,
            store_reflected_value: false,
        }
    }
}

/// The amount of component information to gather when inspecting an entity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum ComponentDetailLevel {
    /// Only component type names are provided.
    Names,
    /// Full component information, including values, is provided.
    #[default]
    Values,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspection::extension_methods::WorldInspectionExtensionTrait;
    use bevy_ecs::{component::Component, reflect::ReflectComponent};
    use bevy_reflect::Reflect;

    #[derive(Component, Reflect, Debug, PartialEq)]
    #[reflect(Component)]
    struct Health(u32);

    fn test_world() -> World {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        world
            .resource_mut::<AppTypeRegistry>()
            .write()
            .register::<Health>();
        world
    }

    #[test]
    fn metadata_map_contains_registered_components() {
        let mut world = test_world();
        world.spawn(Health(7));

        let map = ComponentMetadataMap::generate(&world);
        let component_id = world.components().valid_component_id::<Health>().unwrap();
        let metadata = map.get(&component_id).unwrap();

        assert_eq!(metadata.memory_size, MemorySize::new(4));
        assert!(metadata.mutable);
        assert_eq!(metadata.storage_type, StorageType::Table);
        assert!(metadata.is_send_and_sync);
        assert_eq!(metadata.type_id, Some(TypeId::of::<Health>()));
    }

    #[test]
    fn inspect_component_by_id_reports_value() {
        let mut world = test_world();
        let entity = world.spawn(Health(7)).id();
        let component_id = world.components().valid_component_id::<Health>().unwrap();
        let metadata = ComponentTypeMetadata::new(&world, component_id).unwrap();

        let inspection = world
            .inspect_component_by_id(
                component_id,
                entity,
                &metadata,
                ComponentInspectionSettings::default(),
            )
            .unwrap();

        assert_eq!(inspection.entity, entity);
        assert_eq!(inspection.name.shortname().to_string(), "Health");
        assert!(inspection.value.unwrap().contains('7'));
    }

    #[test]
    fn detail_level_names_omits_value() {
        let mut world = test_world();
        let entity = world.spawn(Health(7)).id();

        let inspection = world
            .inspect_component::<Health>(
                entity,
                ComponentInspectionSettings {
                    detail_level: ComponentDetailLevel::Names,
                    ..Default::default()
                },
            )
            .unwrap();

        assert!(inspection.value.is_none());
    }

    #[test]
    fn reflected_value_is_stored_and_cloned() {
        let mut world = test_world();
        let entity = world.spawn(Health(7)).id();

        let inspection = world
            .inspect_component::<Health>(
                entity,
                ComponentInspectionSettings {
                    store_reflected_value: true,
                    ..Default::default()
                },
            )
            .unwrap();

        assert!(inspection.reflected_value.is_some());

        let cloned = inspection.clone();
        let reflected = cloned.reflected_value.unwrap();
        assert_eq!(
            reflected.try_downcast_ref::<Health>(),
            Some(&Health(7)),
            "the reflected value should survive cloning"
        );
    }

    #[test]
    fn inspect_component_type_counts_entities() {
        let mut world = test_world();
        world.spawn(Health(7));

        let inspection = world.inspect_component_type::<Health>().unwrap();

        assert_eq!(inspection.entity_count, 1);
        assert_eq!(inspection.metadata.name.shortname().to_string(), "Health");
    }

    #[test]
    fn missing_component_returns_error() {
        let mut world = test_world();
        let entity = world.spawn_empty().id();
        let component_id = world.register_component::<Health>();
        let metadata = ComponentTypeMetadata::new(&world, component_id).unwrap();

        let result = world.inspect_component_by_id(
            component_id,
            entity,
            &metadata,
            ComponentInspectionSettings::default(),
        );

        assert_eq!(
            result.unwrap_err(),
            ComponentInspectionError::ComponentNotFound(component_id)
        );
    }

    #[test]
    fn unregistered_component_id_returns_error() {
        let world = test_world();
        let component_id = ComponentId::new(usize::MAX);

        let result = ComponentTypeMetadata::new(&world, component_id);

        assert_eq!(
            result.unwrap_err(),
            ComponentInspectionError::ComponentIdNotRegistered(component_id)
        );
    }

    #[test]
    fn display_contains_short_name() {
        let mut world = test_world();
        let entity = world.spawn(Health(7)).id();

        let inspection = world
            .inspect_component::<Health>(entity, ComponentInspectionSettings::default())
            .unwrap();

        let displayed = inspection.to_string();
        assert!(displayed.contains("Health"), "{displayed}");
        assert!(displayed.contains('7'), "{displayed}");
    }

    #[derive(Component, Reflect, Debug, PartialEq)]
    #[reflect(Component)]
    struct Pathish {
        path: String,
    }

    #[test]
    fn default_settings_keep_double_colons_in_values() {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        world
            .resource_mut::<AppTypeRegistry>()
            .write()
            .register::<Pathish>();

        let entity = world
            .spawn(Pathish {
                path: "bevy::prelude".to_string(),
            })
            .id();

        let inspection = world
            .inspect_component::<Pathish>(entity, ComponentInspectionSettings::default())
            .unwrap();

        assert_eq!(
            inspection.value.unwrap(),
            "bevy_dev_tools::inspection::component_inspection::tests::Pathish {\n  path: \"bevy::prelude\",\n}"
        );
    }
}
