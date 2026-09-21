//! Types describing entities as a whole, gathered by inspecting a [`World`](bevy_ecs::world::World).
//!
//! The per-component counterpart lives in [`component_inspection`](crate::inspection::component_inspection).

use bevy_ecs::{entity::Entity, query::SpawnDetails};
use bevy_utils::memory_size::MemorySize;
use core::fmt::{Display, Formatter};

use crate::inspection::{
    component_inspection::{ComponentInspection, ComponentInspectionSettings},
    label_resolution::EntityLabel,
};

/// The result of inspecting an entity, summarized by its [`Display`] implementation.
#[derive(Clone, Debug)]
pub struct EntityInspection {
    /// The entity being inspected.
    pub entity: Entity,
    /// The label of the entity, if one could be resolved.
    pub label: Option<EntityLabel>,
    /// The sum of the shallow sizes of the entity's components, excluding heap allocations.
    ///
    /// [`None`] if [`include_components`](EntityInspectionSettings::include_components) is false.
    pub total_memory_size: Option<MemorySize>,
    /// The components on the entity, in inspection form.
    pub components: Option<Vec<ComponentInspection>>,
    /// Information about how and when this entity was spawned.
    pub spawn_details: Option<SpawnDetails>,
}

impl Display for EntityInspection {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let label = match &self.label {
            Some(label) => label.as_str(),
            None => "Entity",
        };
        write!(f, "{label} ({})", self.entity)?;

        if let Some(total_memory_size) = &self.total_memory_size {
            write!(f, "\nMemory Size: {total_memory_size}")?;
        }

        if let Some(spawn_details) = self.spawn_details {
            write!(f, "\nSpawned on tick {}", spawn_details.spawn_tick().get())?;

            if let Some(location) = spawn_details.spawned_by().into_option() {
                write!(f, " by {location}")?;
            }
        }

        if let Some(components) = &self.components {
            write!(f, "\nComponents:")?;
            for component in components {
                write!(f, "\n- {component}")?;
            }
        }

        Ok(())
    }
}

/// An error that can occur when attempting to inspect an entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityInspectionError {
    /// The entity does not exist in the world.
    EntityNotFound(Entity),
}

impl Display for EntityInspectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            EntityInspectionError::EntityNotFound(entity) => {
                write!(f, "Entity {entity} not found in world")
            }
        }
    }
}

impl core::error::Error for EntityInspectionError {}

/// Settings for inspecting an individual entity.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityInspectionSettings {
    /// Whether component information should be included in the inspection. Component-based label
    /// resolution is unavailable when it is not.
    pub include_components: bool,
    /// Settings used when inspecting the components on the entity.
    pub component_settings: ComponentInspectionSettings,
}

impl Default for EntityInspectionSettings {
    fn default() -> Self {
        Self {
            include_components: true,
            component_settings: ComponentInspectionSettings::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspection::{
        component_inspection::ComponentMetadataMap,
        extension_methods::WorldInspectionExtensionTrait,
    };
    use bevy_ecs::{component::Component, name::Name, reflect::AppTypeRegistry, world::World};
    use bevy_reflect::Reflect;

    #[derive(Component, Reflect, Debug)]
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
    fn inspect_reports_label_components_and_size() {
        let mut world = test_world();
        let entity = world.spawn((Name::new("Player"), Health(7))).id();

        let inspection = world
            .inspect(entity, EntityInspectionSettings::default())
            .unwrap();

        assert_eq!(inspection.entity, entity);
        assert_eq!(inspection.label.unwrap().as_str(), "Player");

        let components = inspection.components.unwrap();
        assert_eq!(components.len(), 2);

        let expected_size: u64 = components
            .iter()
            .map(|component| component.memory_size.0)
            .sum();
        assert_eq!(
            inspection.total_memory_size,
            Some(MemorySize(expected_size))
        );
    }

    #[test]
    fn inspect_cached_matches_inspect() {
        let mut world = test_world();
        let entity = world.spawn((Name::new("Player"), Health(7))).id();
        let metadata_map = ComponentMetadataMap::generate(&world);

        let settings = EntityInspectionSettings::default();
        let uncached = world.inspect(entity, settings).unwrap();
        let cached = world
            .inspect_cached(entity, &settings, &metadata_map)
            .unwrap();

        assert_eq!(cached.label, uncached.label);
        assert_eq!(cached.total_memory_size, uncached.total_memory_size);
        assert_eq!(
            cached.components.unwrap().len(),
            uncached.components.unwrap().len()
        );
    }

    #[test]
    fn despawned_entity_returns_error() {
        let mut world = test_world();
        let entity = world.spawn(Health(7)).id();
        world.despawn(entity);

        let result = world.inspect(entity, EntityInspectionSettings::default());

        assert_eq!(
            result.unwrap_err(),
            EntityInspectionError::EntityNotFound(entity)
        );
    }

    #[test]
    fn excluding_components_omits_them() {
        let mut world = test_world();
        let entity = world.spawn((Name::new("Player"), Health(7))).id();

        let inspection = world
            .inspect(
                entity,
                EntityInspectionSettings {
                    include_components: false,
                    ..Default::default()
                },
            )
            .unwrap();

        assert!(inspection.components.is_none());
        assert!(inspection.total_memory_size.is_none());
        assert_eq!(inspection.label.unwrap().as_str(), "Player");
    }

    #[test]
    fn display_contains_label_and_components() {
        let mut world = test_world();
        let entity = world.spawn((Name::new("Player"), Health(7))).id();

        let displayed = world
            .inspect(entity, EntityInspectionSettings::default())
            .unwrap()
            .to_string();

        assert!(displayed.contains("Player"), "{displayed}");
        assert!(displayed.contains("Health"), "{displayed}");
    }
}
