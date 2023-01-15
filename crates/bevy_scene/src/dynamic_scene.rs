use crate::{DynamicSceneBuilder, Scene, SceneSpawnError};
use anyhow::Result;
use bevy_app::AppTypeRegistry;
use bevy_ecs::{
    entity::EntityMap,
    reflect::{ReflectComponent, ReflectMapEntities},
    world::World,
};
use bevy_reflect::{Reflect, TypeRegistryArc, TypeUuid};

#[cfg(feature = "serialize")]
use crate::serde::SceneSerializer;
#[cfg(feature = "serialize")]
use serde::Serialize;

/// A collection of serializable dynamic entities, each with its own run-time defined set of components.
/// To spawn a dynamic scene, you can use either:
/// * [`SceneSpawner::spawn_dynamic`](crate::SceneSpawner::spawn_dynamic)
/// * adding the [`DynamicSceneBundle`](crate::DynamicSceneBundle) to an entity
/// * adding the [`Handle<DynamicScene>`](bevy_asset::Handle) to an entity (the scene will only be
/// visible if the entity already has [`Transform`](bevy_transform::components::Transform) and
/// [`GlobalTransform`](bevy_transform::components::GlobalTransform) components)
#[derive(Default, TypeUuid)]
#[uuid = "749479b1-fb8c-4ff8-a775-623aa76014f5"]
pub struct DynamicScene {
    pub entities: Vec<DynamicEntity>,
}

/// A reflection-powered serializable representation of an entity and its components.
pub struct DynamicEntity {
    /// The transiently unique identifier of a corresponding `Entity`.
    pub entity: u32,
    /// A vector of boxed components that belong to the given entity and
    /// implement the `Reflect` trait.
    pub components: Vec<Box<dyn Reflect>>,
}

impl DynamicScene {
    /// Create a new dynamic scene from a given scene.
    pub fn from_scene(scene: &Scene, type_registry: &AppTypeRegistry) -> Self {
        Self::from_world(&scene.world, type_registry)
    }

    /// Create a new dynamic scene from a given world.
    pub fn from_world(world: &World, type_registry: &AppTypeRegistry) -> Self {
        let mut builder =
            DynamicSceneBuilder::from_world_with_type_registry(world, type_registry.clone());

        builder.extract_entities(world.iter_entities().map(|entity| entity.id()));

        builder.build()
    }

    /// Write the dynamic entities and their corresponding components to the given world.
    ///
    /// This method will return a [`SceneSpawnError`] if a type either is not registered
    /// in the provided [`AppTypeRegistry`] resource, or doesn't reflect the
    /// [`Component`](bevy_ecs::component::Component) trait.
    pub fn write_to_world_with(
        &self,
        world: &mut World,
        entity_map: &mut EntityMap,
        type_registry: &AppTypeRegistry,
    ) -> Result<(), SceneSpawnError> {
        let type_registry = type_registry.read();

        for scene_entity in &self.entities {
            // Fetch the entity with the given entity id from the `entity_map`
            // or spawn a new entity with a transiently unique id if there is
            // no corresponding entry.
            let entity = *entity_map
                .entry(bevy_ecs::entity::Entity::from_raw(scene_entity.entity))
                .or_insert_with(|| world.spawn_empty().id());

            // Apply/ add each component to the given entity.
            for component in &scene_entity.components {
                let registration = type_registry
                    .get_with_name(component.type_name())
                    .ok_or_else(|| SceneSpawnError::UnregisteredType {
                        type_name: component.type_name().to_string(),
                    })?;
                let reflect_component =
                    registration.data::<ReflectComponent>().ok_or_else(|| {
                        SceneSpawnError::UnregisteredComponent {
                            type_name: component.type_name().to_string(),
                        }
                    })?;

                // If the entity already has the given component attached,
                // just apply the (possibly) new value, otherwise add the
                // component to the entity.
                reflect_component.apply_or_insert(world, entity, &**component);
            }
        }

        for registration in type_registry.iter() {
            if let Some(map_entities_reflect) = registration.data::<ReflectMapEntities>() {
                map_entities_reflect
                    .map_entities(world, entity_map)
                    .unwrap();
            }
        }

        Ok(())
    }

    /// Write the dynamic entities and their corresponding components to the given world.
    ///
    /// This method will return a [`SceneSpawnError`] if a type either is not registered
    /// in the world's [`AppTypeRegistry`] resource, or doesn't reflect the
    /// [`Component`](bevy_ecs::component::Component) trait.
    pub fn write_to_world(
        &self,
        world: &mut World,
        entity_map: &mut EntityMap,
    ) -> Result<(), SceneSpawnError> {
        let registry = world.resource::<AppTypeRegistry>().clone();
        self.write_to_world_with(world, entity_map, &registry)
    }

    // TODO: move to AssetSaver when it is implemented
    /// Serialize this dynamic scene into rust object notation (ron).
    #[cfg(feature = "serialize")]
    pub fn serialize_ron(&self, registry: &TypeRegistryArc) -> Result<String, ron::Error> {
        serialize_ron(SceneSerializer::new(self, registry))
    }
}

/// Serialize a given Rust data structure into rust object notation (ron).
#[cfg(feature = "serialize")]
pub fn serialize_ron<S>(serialize: S) -> Result<String, ron::Error>
where
    S: Serialize,
{
    let pretty_config = ron::ser::PrettyConfig::default()
        .indentor("  ".to_string())
        .new_line("\n".to_string());
    ron::ser::to_string_pretty(&serialize, pretty_config)
}
