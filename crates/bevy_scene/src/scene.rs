use std::sync::RwLockReadGuard;

use crate::{DynamicScene, InstanceInfo, SceneSpawnError};
use bevy_asset::Asset;
use bevy_ecs::component::ComponentId;
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::{
    reflect::{AppTypeRegistry, ReflectComponent, ReflectMapEntities, ReflectResource},
    world::World,
};
use bevy_reflect::{TypePath, TypeRegistry};

/// To spawn a scene, you can use either:
/// * [`SceneSpawner::spawn`](crate::SceneSpawner::spawn)
/// * adding the [`SceneBundle`](crate::SceneBundle) to an entity
/// * adding the [`Handle<Scene>`](bevy_asset::Handle) to an entity (the scene will only be
/// visible if the entity already has [`Transform`](bevy_transform::components::Transform) and
/// [`GlobalTransform`](bevy_transform::components::GlobalTransform) components)
#[derive(Asset, TypePath, Debug)]
pub struct Scene {
    /// The world of the scene, containing its entities and resources.
    pub world: World,
}

impl Scene {
    /// Creates a new scene with the given world.
    pub fn new(world: World) -> Self {
        Self { world }
    }

    /// Create a new scene from a given dynamic scene.
    pub fn from_dynamic_scene(
        dynamic_scene: &DynamicScene,
        type_registry: &AppTypeRegistry,
    ) -> Result<Scene, SceneSpawnError> {
        let mut world = World::new();
        let mut entity_map = EntityHashMap::default();
        dynamic_scene.write_to_world_with(&mut world, &mut entity_map, type_registry)?;

        Ok(Self { world })
    }

    /// Clone the scene.
    ///
    /// This method will return a [`SceneSpawnError`] if a type either is not registered in the
    /// provided [`AppTypeRegistry`] or doesn't reflect the [`Component`](bevy_ecs::component::Component) trait.
    pub fn clone_with(&self, type_registry: &AppTypeRegistry) -> Result<Scene, SceneSpawnError> {
        let mut new_world = World::new();
        self.write_to_world_with(&mut new_world, type_registry)?;
        Ok(Self { world: new_world })
    }

    /// Writes all resources/entities and their corresponding components in the scene world to new
    /// entities in the given world. On success returns an instance mapping entities in the scene world
    /// to entities in the instance world.
    ///
    /// This method will return a [`SceneSpawnError`] if a type either is not registered in the
    /// provided [`AppTypeRegistry`] or doesn't reflect the [`Component`](bevy_ecs::component::Component) trait.
    pub fn write_to_world_with(
        &self,
        world: &mut World,
        type_registry: &AppTypeRegistry,
    ) -> Result<InstanceInfo, SceneSpawnError> {
        let mut instance_info = InstanceInfo {
            entity_map: EntityHashMap::default(),
            world_id: world.id(),
        };

        let type_registry = type_registry.read();

        self.update_resources(world, &type_registry)?;

        for archetype in self.world.archetypes().iter() {
            for scene_entity in archetype.entities() {
                let entity = *instance_info
                    .entity_map
                    .entry(scene_entity.id())
                    .or_insert_with(|| world.spawn_empty().id());
                for component_id in archetype.components() {
                    let component_info = self
                        .world
                        .components()
                        .get_info(component_id)
                        .expect("component_ids in archetypes should have ComponentInfo");

                    let reflect_component = type_registry
                        .get(component_info.type_id().unwrap())
                        .ok_or_else(|| SceneSpawnError::UnregisteredType {
                            std_type_name: component_info.name().to_string(),
                        })
                        .and_then(|registration| {
                            registration.data::<ReflectComponent>().ok_or_else(|| {
                                SceneSpawnError::UnregisteredComponent {
                                    type_path: registration.type_info().type_path().to_string(),
                                }
                            })
                        })?;
                    reflect_component.copy(
                        &self.world,
                        world,
                        scene_entity.id(),
                        entity,
                        &type_registry,
                    );
                }
            }
        }

        for registration in type_registry.iter() {
            if let Some(map_entities_reflect) = registration.data::<ReflectMapEntities>() {
                map_entities_reflect.map_all_entities(world, &mut instance_info.entity_map);
            }
        }

        Ok(instance_info)
    }

    /// Writes all resources/entities and their corresponding components in the scene world to existing
    /// entities in the given world (as long as they are in the scene instance entity map).
    ///
    /// This method will return a [`SceneSpawnError`] if a type either is not registered in the
    /// provided [`AppTypeRegistry`] or doesn't reflect the [`Component`](bevy_ecs::component::Component) trait.
    pub fn overwrite_in_world_using_instance(
        &self,
        world: &mut World,
        type_registry: &AppTypeRegistry,
        instance_info: &InstanceInfo,
    ) -> Result<(), SceneSpawnError> {
        assert_eq!(
            world.id(),
            instance_info.world_id,
            "instance world should match world you're trying to overwrite"
        );

        let type_registry = type_registry.read();

        self.update_resources(world, &type_registry)?;

        for archetype in self.world.archetypes().iter() {
            for scene_entity in archetype.entities() {
                if let Some(entity) = instance_info.entity_map.get(&scene_entity.id()).cloned() {
                    let components: Vec<ComponentId> = world
                        .get_or_spawn(entity)
                        .expect("tried to overwrite an entity with the wrong generation - generation should match scene instance").archetype().components().collect();

                    for component_id in components.into_iter() {
                        let component_info = self
                            .world
                            .components()
                            .get_info(component_id)
                            .expect("component_ids in archetypes should have ComponentInfo");

                        let reflect_component = type_registry
                            .get(component_info.type_id().unwrap())
                            .ok_or_else(|| SceneSpawnError::UnregisteredType {
                                std_type_name: component_info.name().to_string(),
                            })
                            .and_then(|registration| {
                                registration.data::<ReflectComponent>().ok_or_else(|| {
                                    SceneSpawnError::UnregisteredComponent {
                                        type_path: registration.type_info().type_path().to_string(),
                                    }
                                })
                            })?;
                        reflect_component.copy(
                            &self.world,
                            world,
                            scene_entity.id(),
                            entity,
                            &type_registry,
                        );
                    }
                };
            }
        }

        Ok(())
    }

    fn update_resources(
        &self,
        world: &mut World,
        type_registry: &RwLockReadGuard<TypeRegistry>,
    ) -> Result<(), SceneSpawnError> {
        // Resources archetype
        for (component_id, resource_data) in self.world.storages().resources.iter() {
            if !resource_data.is_present() {
                continue;
            }

            let component_info = self
                .world
                .components()
                .get_info(component_id)
                .expect("component_ids in archetypes should have ComponentInfo");

            let type_id = component_info
                .type_id()
                .expect("reflected resources must have a type_id");

            let registration =
                type_registry
                    .get(type_id)
                    .ok_or_else(|| SceneSpawnError::UnregisteredType {
                        std_type_name: component_info.name().to_string(),
                    })?;
            let reflect_resource = registration.data::<ReflectResource>().ok_or_else(|| {
                SceneSpawnError::UnregisteredResource {
                    type_path: registration.type_info().type_path().to_string(),
                }
            })?;
            reflect_resource.copy(&self.world, world, &type_registry);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::component::Component;
    use bevy_reflect::Reflect;

    use super::*;

    #[derive(Component, Reflect, PartialEq, Eq, Debug, Clone, Copy)]
    #[reflect(Component)]
    pub struct A(u32);

    #[test]
    fn overwrite_test() {
        let mut main_world = World::default();

        let app_type_registry = AppTypeRegistry::default();
        let mut type_registry = app_type_registry.write();
        type_registry.register::<A>();

        let mut scene = Scene::new(World::default());

        // spawn an entity with some component
        let scene_entity = scene.world.spawn(A(0)).id();

        // take a snapshot of the scene
        let snapshot = scene.clone_with(&app_type_registry).unwrap();

        // update component value in entity
        scene.world.entity_mut(scene_entity).insert(A(1));

        // write the scene we've constructed to the main_world
        let instance = scene
            .write_to_world_with(&mut main_world, &app_type_registry)
            .unwrap();

        let world_entity = instance.entity_map.get(&scene_entity).cloned().unwrap();

        assert_eq!(main_world.get::<A>(world_entity).cloned().unwrap(), A(1));

        // load an older snapshot but reuse the same entities
        snapshot
            .overwrite_in_world_using_instance(&mut main_world, &app_type_registry, &instance)
            .unwrap();

        assert_eq!(main_world.get::<A>(world_entity).cloned().unwrap(), A(0));
    }
}
