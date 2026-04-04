use crate::{DynamicWorldBuilder, WorldAsset, WorldInstanceSpawnError};
use bevy_asset::Asset;
use bevy_ecs::reflect::ReflectResource;
use bevy_ecs::{
    entity::{Entity, EntityHashMap, SceneEntityMapper},
    reflect::{AppTypeRegistry, ReflectComponent},
    world::World,
};
use bevy_reflect::{PartialReflect, TypePath};

use bevy_ecs::component::ComponentCloneBehavior;
use bevy_ecs::relationship::RelationshipHookMode;

#[cfg(feature = "serialize")]
use {crate::serde::DynamicWorldSerializer, bevy_reflect::TypeRegistry, serde::Serialize};

/// A collection of serializable resources and dynamic entities.
///
/// Each dynamic entity in the collection contains its own run-time defined set of components.
/// To spawn a dynamic world, you can use either:
/// * [`WorldInstanceSpawner::spawn_dynamic`](crate::WorldInstanceSpawner::spawn_dynamic)
/// * adding the [`DynamicWorldRoot`](crate::components::DynamicWorldRoot) component to an entity.
/// * using the [`DynamicWorldBuilder`] to construct a `DynamicWorld` from `World`.
#[derive(Asset, TypePath, Default)]
pub struct DynamicWorld {
    /// Resources stored in the dynamic world.
    pub resources: Vec<Box<dyn PartialReflect>>,
    /// Entities contained in the dynamic world.
    pub entities: Vec<DynamicEntity>,
}

/// A reflection-powered serializable representation of an entity and its components.
pub struct DynamicEntity {
    /// The identifier of the entity, unique within a [`DynamicWorld`] (and the world it may have been generated from).
    ///
    /// Components that reference this entity must consistently use this identifier.
    pub entity: Entity,
    /// A vector of boxed components that belong to the given entity and
    /// implement the [`PartialReflect`] trait.
    pub components: Vec<Box<dyn PartialReflect>>,
}

impl DynamicWorld {
    /// Create a new dynamic world from a given world.
    ///
    /// The `type_registry` provides type information for extracting components and resources
    /// through reflection. You can get this registry from the **main** world, using
    /// `main_world.resource::<AppTypeRegistry>().read()` or `app_type_registry_res.read()`
    /// (for `Res<AppTypeRegistry>`). Note: the `world` is unlikely to have a type registry
    /// internally.
    pub fn from_world_asset(world: &WorldAsset, type_registry: &TypeRegistry) -> Self {
        Self::from_world_with(&world.world, type_registry)
    }

    /// Create a new dynamic world from a given world.
    ///
    /// Panics if `world` does not contain [`AppTypeRegistry`]. Use [`Self::from_world_with`] to
    /// handle this case.
    pub fn from_world(world: &World) -> Self {
        let type_registry = world.resource::<AppTypeRegistry>().read();
        Self::from_world_with(world, &type_registry)
    }

    /// Create a new dynamic world from a given world.
    ///
    /// The `type_registry` provides type information for extracting components and resources
    /// through reflection. If the `world` is the "real" world (e.g., not a world in a [`WorldAsset`]),
    /// the `world` will contain the registry, which can be acquired using
    /// `world.resource::<AppTypeRegistry>().read()`. For extracting from "scene worlds", you
    /// will need to get the type registry from the main world (you can clone the `AppTypeRegistry`
    /// out of the world to avoid borrowing the world itself).
    pub fn from_world_with(world: &World, type_registry: &TypeRegistry) -> Self {
        DynamicWorldBuilder::from_world(world, type_registry)
            .extract_entities(
                // we do this instead of a query, in order to completely sidestep default query filters.
                // while we could use `Allow<_>`, this wouldn't account for custom disabled components
                world
                    .archetypes()
                    .iter()
                    .flat_map(bevy_ecs::archetype::Archetype::entities)
                    .map(bevy_ecs::archetype::ArchetypeEntity::id),
            )
            .extract_resources()
            .build()
    }

    /// Write the resources, the dynamic entities, and their corresponding components to the given world.
    ///
    /// This method will return a [`WorldInstanceSpawnError`] if a type either is not registered
    /// in the provided [`AppTypeRegistry`] resource, or doesn't reflect the
    /// [`Component`](bevy_ecs::component::Component) or [`Resource`](bevy_ecs::prelude::Resource) trait.
    pub fn write_to_world_with(
        &self,
        world: &mut World,
        entity_map: &mut EntityHashMap<Entity>,
        type_registry: &TypeRegistry,
    ) -> Result<(), WorldInstanceSpawnError> {
        // First ensure that every entity in the dynamic world has a corresponding world
        // entity in the entity map.
        for dynamic_entity in &self.entities {
            // Fetch the entity with the given entity id from the `entity_map`
            // or spawn a new entity with a transiently unique id if there is
            // no corresponding entry.
            entity_map
                .entry(dynamic_entity.entity)
                .or_insert_with(|| world.spawn_empty().id());
        }

        for dynamic_entity in &self.entities {
            // Fetch the entity with the given entity id from the `entity_map`.
            let entity = *entity_map
                .get(&dynamic_entity.entity)
                .expect("should have previously spawned an empty entity");

            // Apply/ add each component to the given entity.
            for component in &dynamic_entity.components {
                let type_info = component.get_represented_type_info().ok_or_else(|| {
                    WorldInstanceSpawnError::NoRepresentedType {
                        type_path: component.reflect_type_path().to_string(),
                    }
                })?;
                let registration = type_registry.get(type_info.type_id()).ok_or_else(|| {
                    WorldInstanceSpawnError::UnregisteredButReflectedType {
                        type_path: type_info.type_path().to_string(),
                    }
                })?;
                let reflect_component =
                    registration.data::<ReflectComponent>().ok_or_else(|| {
                        WorldInstanceSpawnError::UnregisteredComponent {
                            type_path: type_info.type_path().to_string(),
                        }
                    })?;

                {
                    let component_id = reflect_component.register_component(world);
                    // SAFETY: we registered the component above. the info exists
                    #[expect(unsafe_code, reason = "this is faster")]
                    let component_info =
                        unsafe { world.components().get_info_unchecked(component_id) };
                    if matches!(
                        *component_info.clone_behavior(),
                        ComponentCloneBehavior::Ignore
                    ) {
                        continue;
                    }
                }

                SceneEntityMapper::world_scope(entity_map, world, |world, mapper| {
                    reflect_component.apply_or_insert_mapped(
                        &mut world.entity_mut(entity),
                        component.as_partial_reflect(),
                        type_registry,
                        mapper,
                        RelationshipHookMode::Skip,
                    );
                });
            }
        }

        // Insert resources after all entities have been added to the world.
        // This ensures the entities are available for the resources to reference during mapping.
        for resource in &self.resources {
            let type_info = resource.get_represented_type_info().ok_or_else(|| {
                WorldInstanceSpawnError::NoRepresentedType {
                    type_path: resource.reflect_type_path().to_string(),
                }
            })?;
            let registration = type_registry.get(type_info.type_id()).ok_or_else(|| {
                WorldInstanceSpawnError::UnregisteredButReflectedType {
                    type_path: type_info.type_path().to_string(),
                }
            })?;
            registration.data::<ReflectResource>().ok_or_else(|| {
                WorldInstanceSpawnError::UnregisteredResource {
                    type_path: type_info.type_path().to_string(),
                }
            })?;
            // reflect_resource existing, implies that reflect_component also exists
            let reflect_component = registration
                .data::<ReflectComponent>()
                .expect("ReflectComponent is depended on ReflectResource");

            let resource_id = reflect_component.register_component(world);

            // check if the resource already exists, if not spawn it, otherwise override the value
            let entity = if let Some(entity) = world.resource_entities().get(resource_id) {
                *entity
            } else {
                world.spawn_empty().id()
            };

            SceneEntityMapper::world_scope(entity_map, world, |world, mapper| {
                reflect_component.apply_or_insert_mapped(
                    &mut world.entity_mut(entity),
                    resource.as_partial_reflect(),
                    type_registry,
                    mapper,
                    RelationshipHookMode::Skip,
                );
            });
        }

        Ok(())
    }

    /// Write the resources, the dynamic entities, and their corresponding components to the given world.
    ///
    /// This method will return a [`WorldInstanceSpawnError`] if a type either is not registered
    /// in the world's [`AppTypeRegistry`] resource, or doesn't reflect the
    /// [`Component`](bevy_ecs::component::Component) trait.
    pub fn write_to_world(
        &self,
        world: &mut World,
        entity_map: &mut EntityHashMap<Entity>,
    ) -> Result<(), WorldInstanceSpawnError> {
        let registry = world.resource::<AppTypeRegistry>().clone();
        self.write_to_world_with(world, entity_map, &registry.read())
    }

    // TODO: move to AssetSaver when it is implemented
    /// Serialize this dynamic world into the serialized Bevy world format (`.scn` / `.scn.ron`).
    ///
    /// The serialized Bevy world format is based on [Rusty Object Notation (RON)]. It describes the world
    /// in a human-friendly format. To deserialize the format, use the [`WorldAssetLoader`].
    ///
    /// [`WorldAssetLoader`]: crate::WorldAssetLoader
    /// [Rusty Object Notation (RON)]: https://crates.io/crates/ron
    #[cfg(feature = "serialize")]
    pub fn serialize(&self, registry: &TypeRegistry) -> Result<String, ron::Error> {
        serialize_ron(DynamicWorldSerializer::new(self, registry))
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

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component,
        entity::{Entity, EntityHashMap, EntityMapper, MapEntities},
        hierarchy::ChildOf,
        reflect::{AppTypeRegistry, ReflectComponent, ReflectMapEntities, ReflectResource},
        resource::Resource,
        world::World,
    };

    use bevy_reflect::Reflect;

    use crate::dynamic_world::DynamicWorld;
    use crate::dynamic_world_builder::DynamicWorldBuilder;

    #[derive(Resource, Reflect, MapEntities, Debug)]
    #[reflect(Resource, MapEntities)]
    struct TestResource {
        #[entities]
        entity_a: Entity,
        #[entities]
        entity_b: Entity,
    }

    #[test]
    fn resource_entity_map_maps_entities() {
        let app_type_registry = AppTypeRegistry::default();
        app_type_registry.write().register::<TestResource>();

        let mut source_world = World::new();

        let original_entity_a = source_world.spawn_empty().id();
        let original_entity_b = source_world.spawn_empty().id();

        source_world.insert_resource(TestResource {
            entity_a: original_entity_a,
            entity_b: original_entity_b,
        });

        // Write the dynamic world.
        let dynamic_world = {
            let type_registry = app_type_registry.read();
            DynamicWorldBuilder::from_world(&source_world, &type_registry)
                .extract_resources()
                .extract_entity(original_entity_a)
                .extract_entity(original_entity_b)
                .build()
        };

        let mut entity_map = EntityHashMap::default();
        let mut destination_world = World::new();
        destination_world.insert_resource(app_type_registry);

        dynamic_world
            .write_to_world(&mut destination_world, &mut entity_map)
            .unwrap();

        let &from_entity_a = entity_map.get(&original_entity_a).unwrap();
        let &from_entity_b = entity_map.get(&original_entity_b).unwrap();

        let test_resource = destination_world.get_resource::<TestResource>().unwrap();
        assert_eq!(from_entity_a, test_resource.entity_a);
        assert_eq!(from_entity_b, test_resource.entity_b);
    }

    #[test]
    fn components_not_defined_in_dynamic_world_should_not_be_affected_by_scene_entity_map() {
        // Testing that dynamic world reloading applies EntityMap correctly to MapEntities components.

        // First, we create a simple world with a parent and a child relationship
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        world
            .resource_mut::<AppTypeRegistry>()
            .write()
            .register::<ChildOf>();
        let original_parent_entity = world.spawn_empty().id();
        let original_child_entity = world.spawn_empty().id();
        world
            .entity_mut(original_parent_entity)
            .add_child(original_child_entity);

        // We then write this relationship to a new dynamic world, and then write that dynamic world back to the
        // world to create another parent and child relationship
        let dynamic_world = {
            let type_registry = world.resource::<AppTypeRegistry>().read();
            DynamicWorldBuilder::from_world(&world, &type_registry)
                .extract_entity(original_parent_entity)
                .extract_entity(original_child_entity)
                .build()
        };
        let mut entity_map = EntityHashMap::default();
        dynamic_world
            .write_to_world(&mut world, &mut entity_map)
            .unwrap();

        let &from_dynamic_parent_entity = entity_map.get(&original_parent_entity).unwrap();
        let &from_dynamic_child_entity = entity_map.get(&original_child_entity).unwrap();

        // We then add the parent from the dynamic world as a child of the original child
        // Hierarchy should look like:
        // Original Parent <- Original Child <- Dynamic World Parent <- Dynamic World Child
        world
            .entity_mut(original_child_entity)
            .add_child(from_dynamic_parent_entity);

        // We then reload the dynamic world to make sure that from_dynamic_world_parent_entity's parent component
        // isn't updated with the entity map, since this component isn't defined in the dynamic world.
        // With [`bevy_ecs::hierarchy`], this can cause serious errors and malformed hierarchies.
        dynamic_world
            .write_to_world(&mut world, &mut entity_map)
            .unwrap();

        assert_eq!(
            original_parent_entity,
            world
                .get_entity(original_child_entity)
                .unwrap()
                .get::<ChildOf>()
                .unwrap()
                .parent(),
            "something about reloading the dynamic world is touching entities with the same dynamic world Ids"
        );
        assert_eq!(
            original_child_entity,
            world
                .get_entity(from_dynamic_parent_entity)
                .unwrap()
                .get::<ChildOf>()
                .unwrap()
                .parent(),
            "something about reloading the dynamic world is touching components not defined in the dynamic world but on entities defined in the dynamic world"
        );
        assert_eq!(
            from_dynamic_parent_entity,
            world
                .get_entity(from_dynamic_child_entity)
                .unwrap()
                .get::<ChildOf>()
                .expect("something is wrong with this test, and the dynamic world components don't have a parent/child relationship")
                .parent(),
            "something is wrong with this test or the code reloading dynamic worlds since the relationship between dynamic world entities is broken"
        );
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/14300
    // Fails before the fix in https://github.com/bevyengine/bevy/pull/15405
    #[test]
    fn no_panic_in_map_entities_after_pending_entity_in_hook() {
        #[derive(Default, Component, Reflect)]
        #[reflect(Component)]
        struct A;

        #[derive(Component, Reflect)]
        #[reflect(Component)]
        struct B(pub Entity);

        impl MapEntities for B {
            fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
                self.0 = entity_mapper.get_mapped(self.0);
            }
        }

        let reg = AppTypeRegistry::default();
        {
            let mut reg_write = reg.write();
            reg_write.register::<A>();
            reg_write.register::<B>();
        }

        let mut world = World::new();
        world.insert_resource(reg.clone());
        world.spawn((B(Entity::PLACEHOLDER), A));
        let dynamic_world = DynamicWorld::from_world(&world);

        let mut dst_world = World::new();
        dst_world
            .register_component_hooks::<A>()
            .on_add(|mut world, _| {
                world.commands().spawn_empty();
            });
        dst_world.insert_resource(reg.clone());

        // Should not panic.
        // Prior to fix, the `Entities::alloc` call in
        // `EntityMapper::map_entity` would panic due to pending entities from the observer
        // not having been flushed.
        dynamic_world
            .write_to_world(&mut dst_world, &mut Default::default())
            .unwrap();
    }
}
