use crate::{DynamicEntity, DynamicScene};
use bevy_app::AppTypeRegistry;
use bevy_ecs::{prelude::Entity, reflect::ReflectComponent, world::World};
use bevy_utils::default;
use std::collections::BTreeMap;

/// A [`DynamicScene`] builder, used to build a scene from a [`World`] by extracting some entities.
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
/// # use bevy_app::AppTypeRegistry;
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
    entities: BTreeMap<u32, DynamicEntity>,
    type_registry: AppTypeRegistry,
    world: &'w World,
}

impl<'w> DynamicSceneBuilder<'w> {
    /// Prepare a builder that will extract entities and their component from the given [`World`].
    /// All components registered in that world's [`AppTypeRegistry`] resource will be extracted.
    pub fn from_world(world: &'w World) -> Self {
        Self {
            entities: default(),
            type_registry: world.resource::<AppTypeRegistry>().clone(),
            world,
        }
    }

    /// Prepare a builder that will extract entities and their component from the given [`World`].
    /// Only components registered in the given [`AppTypeRegistry`] will be extracted.
    pub fn from_world_with_type_registry(world: &'w World, type_registry: AppTypeRegistry) -> Self {
        Self {
            entities: default(),
            type_registry,
            world,
        }
    }

    /// Consume the builder, producing a [`DynamicScene`].
    pub fn build(self) -> DynamicScene {
        DynamicScene {
            entities: self.entities.into_values().collect(),
        }
    }

    /// Extract one entity from the builder's [`World`].
    ///
    /// Re-extracting an entity that was already extracted will have no effect.
    pub fn extract_entity(&mut self, entity: Entity) -> &mut Self {
        self.extract_entities(std::iter::once(entity))
    }

    /// Extract entities from the builder's [`World`].
    ///
    /// Re-extracting an entity that was already extracted will have no effect.
    ///
    /// Extracting entities can be used to extract entities from a query:
    /// ```
    /// # use bevy_scene::DynamicSceneBuilder;
    /// # use bevy_app::AppTypeRegistry;
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
            let index = entity.index();

            if self.entities.contains_key(&index) {
                continue;
            }

            let mut entry = DynamicEntity {
                entity: index,
                components: Vec::new(),
            };

            for component_id in self.world.entity(entity).archetype().components() {
                let reflect_component = self
                    .world
                    .components()
                    .get_info(component_id)
                    .and_then(|info| type_registry.get(info.type_id().unwrap()))
                    .and_then(|registration| registration.data::<ReflectComponent>());

                if let Some(reflect_component) = reflect_component {
                    if let Some(component) = reflect_component.reflect(self.world, entity) {
                        entry.components.push(component.clone_value());
                    }
                }
            }
            self.entities.insert(index, entry);
        }

        drop(type_registry);
        self
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::AppTypeRegistry;
    use bevy_ecs::{
        component::Component, prelude::Entity, query::With, reflect::ReflectComponent, world::World,
    };

    use bevy_reflect::Reflect;

    use super::DynamicSceneBuilder;

    #[derive(Component, Reflect, Default, Eq, PartialEq, Debug)]
    #[reflect(Component)]
    struct ComponentA;
    #[derive(Component, Reflect, Default, Eq, PartialEq, Debug)]
    #[reflect(Component)]
    struct ComponentB;

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
        assert_eq!(scene.entities[0].entity, entity.index());
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
        assert_eq!(scene.entities[0].entity, entity.index());
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
        assert_eq!(scene.entities[0].entity, entity.index());
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
        assert_eq!(entity_a.index(), entities.next().map(|e| e.entity).unwrap());
        assert_eq!(entity_b.index(), entities.next().map(|e| e.entity).unwrap());
        assert_eq!(entity_c.index(), entities.next().map(|e| e.entity).unwrap());
        assert_eq!(entity_d.index(), entities.next().map(|e| e.entity).unwrap());
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
        assert_eq!(scene_entities, [entity_a_b.index(), entity_a.index()]);
    }
}
