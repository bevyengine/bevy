use crate::{DynamicEntity, DynamicScene};
use bevy_ecs::component::ComponentId;
use bevy_ecs::{
    prelude::Entity,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    world::World,
};
use bevy_reflect::Reflect;
use bevy_utils::default;
use std::collections::BTreeMap;

/// A [`DynamicScene`] builder, used to build a scene from a [`World`] by extracting some entities and resources.
///
/// # Entity Order
///
/// Extracted entities will always be stored in ascending order based on their [id](Entity::index).
/// This means that inserting `Entity(1v0)` then `Entity(0v0)` will always result in the entities
/// being ordered as `[Entity(0v0), Entity(1v0)]`.
///
/// # Example
/// ```
/// # use bevy_scene::DynamicSceneBuilder;
/// # use bevy_ecs::reflect::AppTypeRegistry;
/// # use bevy_ecs::{
/// #     component::Component, prelude::Entity, query::With, reflect::ReflectComponent, world::World,
/// # };
/// # use bevy_reflect::Reflect;
/// # #[derive(Component, Reflect, Default, Eq, PartialEq, Debug)]
/// # #[reflect(Component)]
/// # struct ComponentA;
/// # let mut world = World::default();
/// # world.init_resource::<AppTypeRegistry>();
/// # let entity = world.spawn(ComponentA).id();
/// let mut builder = DynamicSceneBuilder::from_world(&world);
/// builder.extract_entity(entity);
/// let dynamic_scene = builder.build();
/// ```
pub struct DynamicSceneBuilder<'w> {
    extracted_resources: BTreeMap<ComponentId, Box<dyn Reflect>>,
    extracted_scene: BTreeMap<Entity, DynamicEntity>,
    type_registry: AppTypeRegistry,
    original_world: &'w World,
}

impl<'w> DynamicSceneBuilder<'w> {
    /// Prepare a builder that will extract entities and their component from the given [`World`].
    /// All components registered in that world's [`AppTypeRegistry`] resource will be extracted.
    pub fn from_world(world: &'w World) -> Self {
        Self {
            extracted_resources: default(),
            extracted_scene: default(),
            type_registry: world.resource::<AppTypeRegistry>().clone(),
            original_world: world,
        }
    }

    /// Prepare a builder that will extract entities and their component from the given [`World`].
    /// Only components registered in the given [`AppTypeRegistry`] will be extracted.
    pub fn from_world_with_type_registry(world: &'w World, type_registry: AppTypeRegistry) -> Self {
        Self {
            extracted_resources: default(),
            extracted_scene: default(),
            type_registry,
            original_world: world,
        }
    }

    /// Consume the builder, producing a [`DynamicScene`].
    ///
    /// To make sure the dynamic scene doesn't contain entities without any components, call
    /// [`Self::remove_empty_entities`] before building the scene.
    pub fn build(self) -> DynamicScene {
        DynamicScene {
            resources: self.extracted_resources.into_values().collect(),
            entities: self.extracted_scene.into_values().collect(),
        }
    }

    /// Extract one entity from the builder's [`World`].
    ///
    /// Re-extracting an entity that was already extracted will have no effect.
    pub fn extract_entity(&mut self, entity: Entity) -> &mut Self {
        self.extract_entities(std::iter::once(entity))
    }

    /// Despawns all entities with no components.
    ///
    /// These were likely created because none of their components were present in the provided type registry upon extraction.
    pub fn remove_empty_entities(&mut self) -> &mut Self {
        self.extracted_scene
            .retain(|_, entity| !entity.components.is_empty());

        self
    }

    /// Extract entities from the builder's [`World`].
    ///
    /// Re-extracting an entity that was already extracted will have no effect.
    ///
    /// Extracting entities can be used to extract entities from a query:
    /// ```
    /// # use bevy_scene::DynamicSceneBuilder;
    /// # use bevy_ecs::reflect::AppTypeRegistry;
    /// # use bevy_ecs::{
    /// #     component::Component, prelude::Entity, query::With, reflect::ReflectComponent, world::World,
    /// # };
    /// # use bevy_reflect::Reflect;
    /// #[derive(Component, Default, Reflect)]
    /// #[reflect(Component)]
    /// struct MyComponent;
    ///
    /// # let mut world = World::default();
    /// # world.init_resource::<AppTypeRegistry>();
    /// # let _entity = world.spawn(MyComponent).id();
    /// let mut query = world.query_filtered::<Entity, With<MyComponent>>();
    ///
    /// let mut builder = DynamicSceneBuilder::from_world(&world);
    /// builder.extract_entities(query.iter(&world));
    /// let scene = builder.build();
    /// ```
    pub fn extract_entities(&mut self, entities: impl Iterator<Item = Entity>) -> &mut Self {
        let type_registry = self.type_registry.read();

        for entity in entities {
            if self.extracted_scene.contains_key(&entity) {
                continue;
            }

            let mut entry = DynamicEntity {
                entity,
                components: Vec::new(),
            };

            let original_entity = self.original_world.entity(entity);
            for component_id in original_entity.archetype().components() {
                let mut extract_and_push = || {
                    let type_id = self
                        .original_world
                        .components()
                        .get_info(component_id)?
                        .type_id()?;
                    let component = type_registry
                        .get(type_id)?
                        .data::<ReflectComponent>()?
                        .reflect(original_entity)?;
                    entry.components.push(component.clone_value());
                    Some(())
                };
                extract_and_push();
            }
            self.extracted_scene.insert(entity, entry);
        }

        drop(type_registry);
        self
    }

    /// Extract resources from the builder's [`World`].
    ///
    /// Only resources registered in the builder's [`AppTypeRegistry`] will be extracted.
    /// Re-extracting a resource that was already extracted will have no effect.
    /// ```
    /// # use bevy_scene::DynamicSceneBuilder;
    /// # use bevy_ecs::reflect::AppTypeRegistry;
    /// # use bevy_ecs::prelude::{ReflectResource, Resource, World};
    /// # use bevy_reflect::Reflect;
    /// #[derive(Resource, Default, Reflect)]
    /// #[reflect(Resource)]
    /// struct MyResource;
    ///
    /// # let mut world = World::default();
    /// # world.init_resource::<AppTypeRegistry>();
    /// world.insert_resource(MyResource);
    ///
    /// let mut builder = DynamicSceneBuilder::from_world(&world);
    /// builder.extract_resources();
    /// let scene = builder.build();
    /// ```
    pub fn extract_resources(&mut self) -> &mut Self {
        let type_registry = self.type_registry.read();
        for (component_id, _) in self.original_world.storages().resources.iter() {
            let mut extract_and_push = || {
                let type_id = self
                    .original_world
                    .components()
                    .get_info(component_id)?
                    .type_id()?;
                let resource = type_registry
                    .get(type_id)?
                    .data::<ReflectResource>()?
                    .reflect(self.original_world)?;
                self.extracted_resources
                    .insert(component_id, resource.clone_value());
                Some(())
            };
            extract_and_push();
        }

        drop(type_registry);
        self
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component, prelude::Entity, prelude::Resource, query::With,
        reflect::AppTypeRegistry, reflect::ReflectComponent, reflect::ReflectResource,
        world::World,
    };

    use bevy_reflect::Reflect;

    use super::DynamicSceneBuilder;

    #[derive(Component, Reflect, Default, Eq, PartialEq, Debug)]
    #[reflect(Component)]
    struct ComponentA;

    #[derive(Component, Reflect, Default, Eq, PartialEq, Debug)]
    #[reflect(Component)]
    struct ComponentB;

    #[derive(Resource, Reflect, Default, Eq, PartialEq, Debug)]
    #[reflect(Resource)]
    struct ResourceA;

    #[test]
    fn extract_one_entity() {
        let mut world = World::default();

        let atr = AppTypeRegistry::default();
        atr.write().register::<ComponentA>();
        world.insert_resource(atr);

        let entity = world.spawn((ComponentA, ComponentB)).id();

        let mut builder = DynamicSceneBuilder::from_world(&world);
        builder.extract_entity(entity);
        let scene = builder.build();

        assert_eq!(scene.entities.len(), 1);
        assert_eq!(scene.entities[0].entity, entity);
        assert_eq!(scene.entities[0].components.len(), 1);
        assert!(scene.entities[0].components[0].represents::<ComponentA>());
    }

    #[test]
    fn extract_one_entity_twice() {
        let mut world = World::default();

        let atr = AppTypeRegistry::default();
        atr.write().register::<ComponentA>();
        world.insert_resource(atr);

        let entity = world.spawn((ComponentA, ComponentB)).id();

        let mut builder = DynamicSceneBuilder::from_world(&world);
        builder.extract_entity(entity);
        builder.extract_entity(entity);
        let scene = builder.build();

        assert_eq!(scene.entities.len(), 1);
        assert_eq!(scene.entities[0].entity, entity);
        assert_eq!(scene.entities[0].components.len(), 1);
        assert!(scene.entities[0].components[0].represents::<ComponentA>());
    }

    #[test]
    fn extract_one_entity_two_components() {
        let mut world = World::default();

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<ComponentA>();
            register.register::<ComponentB>();
        }
        world.insert_resource(atr);

        let entity = world.spawn((ComponentA, ComponentB)).id();

        let mut builder = DynamicSceneBuilder::from_world(&world);
        builder.extract_entity(entity);
        let scene = builder.build();

        assert_eq!(scene.entities.len(), 1);
        assert_eq!(scene.entities[0].entity, entity);
        assert_eq!(scene.entities[0].components.len(), 2);
        assert!(scene.entities[0].components[0].represents::<ComponentA>());
        assert!(scene.entities[0].components[1].represents::<ComponentB>());
    }

    #[test]
    fn extract_entity_order() {
        let mut world = World::default();
        world.init_resource::<AppTypeRegistry>();

        // Spawn entities in order
        let entity_a = world.spawn_empty().id();
        let entity_b = world.spawn_empty().id();
        let entity_c = world.spawn_empty().id();
        let entity_d = world.spawn_empty().id();

        let mut builder = DynamicSceneBuilder::from_world(&world);

        // Insert entities out of order
        builder.extract_entity(entity_b);
        builder.extract_entities([entity_d, entity_a].into_iter());
        builder.extract_entity(entity_c);

        let mut entities = builder.build().entities.into_iter();

        // Assert entities are ordered
        assert_eq!(entity_a, entities.next().map(|e| e.entity).unwrap());
        assert_eq!(entity_b, entities.next().map(|e| e.entity).unwrap());
        assert_eq!(entity_c, entities.next().map(|e| e.entity).unwrap());
        assert_eq!(entity_d, entities.next().map(|e| e.entity).unwrap());
    }

    #[test]
    fn extract_query() {
        let mut world = World::default();

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<ComponentA>();
            register.register::<ComponentB>();
        }
        world.insert_resource(atr);

        let entity_a_b = world.spawn((ComponentA, ComponentB)).id();
        let entity_a = world.spawn(ComponentA).id();
        let _entity_b = world.spawn(ComponentB).id();

        let mut query = world.query_filtered::<Entity, With<ComponentA>>();
        let mut builder = DynamicSceneBuilder::from_world(&world);
        builder.extract_entities(query.iter(&world));
        let scene = builder.build();

        assert_eq!(scene.entities.len(), 2);
        let mut scene_entities = vec![scene.entities[0].entity, scene.entities[1].entity];
        scene_entities.sort();
        assert_eq!(scene_entities, [entity_a_b, entity_a]);
    }

    #[test]
    fn remove_componentless_entity() {
        let mut world = World::default();

        let atr = AppTypeRegistry::default();
        atr.write().register::<ComponentA>();
        world.insert_resource(atr);

        let entity_a = world.spawn(ComponentA).id();
        let entity_b = world.spawn(ComponentB).id();

        let mut builder = DynamicSceneBuilder::from_world(&world);
        builder.extract_entities([entity_a, entity_b].into_iter());
        builder.remove_empty_entities();
        let scene = builder.build();

        assert_eq!(scene.entities.len(), 1);
        assert_eq!(scene.entities[0].entity, entity_a);
    }

    #[test]
    fn extract_one_resource() {
        let mut world = World::default();

        let atr = AppTypeRegistry::default();
        atr.write().register::<ResourceA>();
        world.insert_resource(atr);

        world.insert_resource(ResourceA);

        let mut builder = DynamicSceneBuilder::from_world(&world);
        builder.extract_resources();
        let scene = builder.build();

        assert_eq!(scene.resources.len(), 1);
        assert!(scene.resources[0].represents::<ResourceA>());
    }

    #[test]
    fn extract_one_resource_twice() {
        let mut world = World::default();

        let atr = AppTypeRegistry::default();
        atr.write().register::<ResourceA>();
        world.insert_resource(atr);

        world.insert_resource(ResourceA);

        let mut builder = DynamicSceneBuilder::from_world(&world);
        builder.extract_resources();
        builder.extract_resources();
        let scene = builder.build();

        assert_eq!(scene.resources.len(), 1);
        assert!(scene.resources[0].represents::<ResourceA>());
    }
}
