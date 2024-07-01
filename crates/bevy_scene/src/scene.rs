use crate::{DynamicScene, InstanceInfo, SceneSpawnError};
use bevy_asset::Asset;
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::{
    reflect::{AppTypeRegistry, ReflectComponent, ReflectMapEntities, ReflectResource},
    world::World,
};
use bevy_reflect::TypePath;

/// To spawn a scene, you can use either:
/// * [`SceneSpawner::spawn`](crate::SceneSpawner::spawn)
/// * adding the [`SceneBundle`](crate::SceneBundle) to an entity
/// * adding the [`Handle<Scene>`](bevy_asset::Handle) to an entity (the scene will only be
///     visible if the entity already has [`Transform`](bevy_transform::components::Transform) and
///     [`GlobalTransform`](bevy_transform::components::GlobalTransform) components)
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

    /// Write the entities and their corresponding components to the given world.
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
        };

        let type_registry = type_registry.read();

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
}
