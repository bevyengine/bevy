use crate::{DynamicSceneBuilder, Scene, SceneSpawnError};
use bevy_asset::Asset;
use bevy_ecs::reflect::{ReflectMapEntities, ReflectResource};
use bevy_ecs::{
    entity::{Entity, EntityHashMap, SceneEntityMapper},
    reflect::{AppTypeRegistry, ReflectComponent},
    world::World,
};
use bevy_reflect::{PartialReflect, TypePath};

use crate::reflect_utils::clone_reflect_value;
use bevy_ecs::component::ComponentCloneBehavior;
use bevy_ecs::relationship::RelationshipHookMode;

#[cfg(feature = "serialize")]
use {
    crate::{ron, serde::SceneSerializer},
    bevy_reflect::TypeRegistry,
    serde::Serialize,
};

/// A collection of serializable resources and dynamic entities.
///
/// Each dynamic entity in the collection contains its own run-time defined set of components.
/// To spawn a dynamic scene, you can use either:
/// * [`SceneSpawner::spawn_dynamic`](crate::SceneSpawner::spawn_dynamic)
/// * adding the [`DynamicSceneRoot`](crate::components::DynamicSceneRoot) component to an entity.
/// * using the [`DynamicSceneBuilder`] to construct a `DynamicScene` from `World`.
#[derive(Asset, TypePath, Default)]
pub struct DynamicScene {
    /// Resources stored in the dynamic scene.
    pub resources: Vec<Box<dyn PartialReflect>>,
    /// Entities contained in the dynamic scene.
    pub entities: Vec<DynamicEntity>,
}

/// A reflection-powered serializable representation of an entity and its components.
pub struct DynamicEntity {
    /// The identifier of the entity, unique within a scene (and the world it may have been generated from).
    ///
    /// Components that reference this entity must consistently use this identifier.
    pub entity: Entity,
    /// A vector of boxed components that belong to the given entity and
    /// implement the [`PartialReflect`] trait.
    pub components: Vec<Box<dyn PartialReflect>>,
}

impl DynamicScene {
    /// Create a new dynamic scene from a given scene.
    pub fn from_scene(scene: &Scene) -> Self {
        Self::from_world(&scene.world)
    }

    /// Create a new dynamic scene from a given world.
    pub fn from_world(world: &World) -> Self {
        DynamicSceneBuilder::from_world(world)
            .extract_entities(
                // we do this instead of a query, in order to completely sidestep default query filters.
                // while we could use `Allows<_>`, this wouldn't account for custom disabled components
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
    /// This method will return a [`SceneSpawnError`] if a type either is not registered
    /// in the provided [`AppTypeRegistry`] resource, or doesn't reflect the
    /// [`Component`](bevy_ecs::component::Component) or [`Resource`](bevy_ecs::prelude::Resource) trait.
    pub fn write_to_world_with(
        &self,
        world: &mut World,
        entity_map: &mut EntityHashMap<Entity>,
        type_registry: &AppTypeRegistry,
    ) -> Result<(), SceneSpawnError> {
        let type_registry = type_registry.read();

        // First ensure that every entity in the scene has a corresponding world
        // entity in the entity map.
        for scene_entity in &self.entities {
            // Fetch the entity with the given entity id from the `entity_map`
            // or spawn a new entity with a transiently unique id if there is
            // no corresponding entry.
            entity_map
                .entry(scene_entity.entity)
                .or_insert_with(|| world.spawn_empty().id());
        }

        for scene_entity in &self.entities {
            // Fetch the entity with the given entity id from the `entity_map`.
            let entity = *entity_map
                .get(&scene_entity.entity)
                .expect("should have previously spawned an empty entity");

            // Apply/ add each component to the given entity.
            for component in &scene_entity.components {
                let type_info = component.get_represented_type_info().ok_or_else(|| {
                    SceneSpawnError::NoRepresentedType {
                        type_path: component.reflect_type_path().to_string(),
                    }
                })?;
                let registration = type_registry.get(type_info.type_id()).ok_or_else(|| {
                    SceneSpawnError::UnregisteredButReflectedType {
                        type_path: type_info.type_path().to_string(),
                    }
                })?;
                let reflect_component =
                    registration.data::<ReflectComponent>().ok_or_else(|| {
                        SceneSpawnError::UnregisteredComponent {
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
                        &type_registry,
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
                SceneSpawnError::NoRepresentedType {
                    type_path: resource.reflect_type_path().to_string(),
                }
            })?;
            let registration = type_registry.get(type_info.type_id()).ok_or_else(|| {
                SceneSpawnError::UnregisteredButReflectedType {
                    type_path: type_info.type_path().to_string(),
                }
            })?;
            let reflect_resource = registration.data::<ReflectResource>().ok_or_else(|| {
                SceneSpawnError::UnregisteredResource {
                    type_path: type_info.type_path().to_string(),
                }
            })?;

            // If this component references entities in the scene, update
            // them to the entities in the world.
            let mut cloned_resource;
            let partial_reflect_resource = if let Some(map_entities) =
                registration.data::<ReflectMapEntities>()
            {
                cloned_resource = clone_reflect_value(resource.as_partial_reflect(), registration);
                SceneEntityMapper::world_scope(entity_map, world, |_, mapper| {
                    map_entities.map_entities(cloned_resource.as_partial_reflect_mut(), mapper);
                });
                cloned_resource.as_partial_reflect()
            } else {
                resource.as_partial_reflect()
            };

            // If the world already contains an instance of the given resource
            // just apply the (possibly) new value, otherwise insert the resource
            reflect_resource.apply_or_insert(world, partial_reflect_resource, &type_registry);
        }

        Ok(())
    }

    /// Write the resources, the dynamic entities, and their corresponding components to the given world.
    ///
    /// This method will return a [`SceneSpawnError`] if a type either is not registered
    /// in the world's [`AppTypeRegistry`] resource, or doesn't reflect the
    /// [`Component`](bevy_ecs::component::Component) trait.
    pub fn write_to_world(
        &self,
        world: &mut World,
        entity_map: &mut EntityHashMap<Entity>,
    ) -> Result<(), SceneSpawnError> {
        let registry = world.resource::<AppTypeRegistry>().clone();
        self.write_to_world_with(world, entity_map, &registry)
    }

    // TODO: move to AssetSaver when it is implemented
    /// Serialize this dynamic scene into the official Bevy scene format (`.scn` / `.scn.ron`).
    ///
    /// The Bevy scene format is based on [Rusty Object Notation (RON)]. It describes the scene
    /// in a human-friendly format. To deserialize the scene, use the [`SceneLoader`].
    ///
    /// [`SceneLoader`]: crate::SceneLoader
    /// [Rusty Object Notation (RON)]: https://crates.io/crates/ron
    #[cfg(feature = "serialize")]
    pub fn serialize(&self, registry: &TypeRegistry) -> Result<String, ron::Error> {
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

    use crate::dynamic_scene::DynamicScene;
    use crate::dynamic_scene_builder::DynamicSceneBuilder;

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
        let type_registry = AppTypeRegistry::default();
        type_registry.write().register::<TestResource>();

        let mut source_world = World::new();
        source_world.insert_resource(type_registry.clone());

        let original_entity_a = source_world.spawn_empty().id();
        let original_entity_b = source_world.spawn_empty().id();

        source_world.insert_resource(TestResource {
            entity_a: original_entity_a,
            entity_b: original_entity_b,
        });

        // Write the scene.
        let scene = DynamicSceneBuilder::from_world(&source_world)
            .extract_resources()
            .extract_entity(original_entity_a)
            .extract_entity(original_entity_b)
            .build();

        let mut entity_map = EntityHashMap::default();
        let mut destination_world = World::new();
        destination_world.insert_resource(type_registry);

        scene
            .write_to_world(&mut destination_world, &mut entity_map)
            .unwrap();

        let &from_entity_a = entity_map.get(&original_entity_a).unwrap();
        let &from_entity_b = entity_map.get(&original_entity_b).unwrap();

        let test_resource = destination_world.get_resource::<TestResource>().unwrap();
        assert_eq!(from_entity_a, test_resource.entity_a);
        assert_eq!(from_entity_b, test_resource.entity_b);
    }

    #[test]
    fn components_not_defined_in_scene_should_not_be_affected_by_scene_entity_map() {
        // Testing that scene reloading applies EntityMap correctly to MapEntities components.

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

        // We then write this relationship to a new scene, and then write that scene back to the
        // world to create another parent and child relationship
        let scene = DynamicSceneBuilder::from_world(&world)
            .extract_entity(original_parent_entity)
            .extract_entity(original_child_entity)
            .build();
        let mut entity_map = EntityHashMap::default();
        scene.write_to_world(&mut world, &mut entity_map).unwrap();

        let &from_scene_parent_entity = entity_map.get(&original_parent_entity).unwrap();
        let &from_scene_child_entity = entity_map.get(&original_child_entity).unwrap();

        // We then add the parent from the scene as a child of the original child
        // Hierarchy should look like:
        // Original Parent <- Original Child <- Scene Parent <- Scene Child
        world
            .entity_mut(original_child_entity)
            .add_child(from_scene_parent_entity);

        // We then reload the scene to make sure that from_scene_parent_entity's parent component
        // isn't updated with the entity map, since this component isn't defined in the scene.
        // With [`bevy_ecs::hierarchy`], this can cause serious errors and malformed hierarchies.
        scene.write_to_world(&mut world, &mut entity_map).unwrap();

        assert_eq!(
            original_parent_entity,
            world
                .get_entity(original_child_entity)
                .unwrap()
                .get::<ChildOf>()
                .unwrap()
                .parent(),
            "something about reloading the scene is touching entities with the same scene Ids"
        );
        assert_eq!(
            original_child_entity,
            world
                .get_entity(from_scene_parent_entity)
                .unwrap()
                .get::<ChildOf>()
                .unwrap()
                .parent(),
            "something about reloading the scene is touching components not defined in the scene but on entities defined in the scene"
        );
        assert_eq!(
            from_scene_parent_entity,
            world
                .get_entity(from_scene_child_entity)
                .unwrap()
                .get::<ChildOf>()
                .expect("something is wrong with this test, and the scene components don't have a parent/child relationship")
                .parent(),
            "something is wrong with this test or the code reloading scenes since the relationship between scene entities is broken"
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

        let mut scene_world = World::new();
        scene_world.insert_resource(reg.clone());
        scene_world.spawn((B(Entity::PLACEHOLDER), A));
        let scene = DynamicScene::from_world(&scene_world);

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
        scene
            .write_to_world(&mut dst_world, &mut Default::default())
            .unwrap();
    }
}
