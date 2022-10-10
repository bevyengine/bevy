use crate::{DynamicEntity, DynamicScene};
use bevy_app::AppTypeRegistry;
use bevy_ecs::{prelude::Entity, reflect::ReflectComponent, world::World};
use bevy_utils::{default, HashMap};

/// A [`DynamicScene`] builder, used to build a scene from a [`World`] by extracting some entities.
///
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
    scene: HashMap<u32, DynamicEntity>,
    type_registry: AppTypeRegistry,
    world: &'w World,
}

impl<'w> DynamicSceneBuilder<'w> {
    /// Prepare a builder that will extract entities and their component from the given [`World`].
    /// All components registered in that world's [`AppTypeRegistry`] resource will be extracted.
    pub fn from_world(world: &'w World) -> Self {
        Self {
            scene: default(),
            type_registry: world.resource::<AppTypeRegistry>().clone(),
            world,
        }
    }

    /// Prepare a builder that will extract entities and their component from the given [`World`].
    /// Only components registered in the given [`AppTypeRegistry`] will be extracted.
    pub fn from_world_with_type_registry(world: &'w World, type_registry: AppTypeRegistry) -> Self {
        Self {
            scene: default(),
            type_registry,
            world,
        }
    }

    /// Consume the builder, producing a [`DynamicScene`].
    pub fn build(self) -> DynamicScene {
        DynamicScene {
            entities: self.scene.into_values().collect(),
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
            if self.scene.contains_key(&entity.id()) {
                continue;
            }

            let mut entry = DynamicEntity {
                entity: entity.id(),
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

            self.scene.insert(entity.id(), entry);
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
        assert_eq!(scene.entities[0].entity, entity.id());
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
        assert_eq!(scene.entities[0].entity, entity.id());
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
        assert_eq!(scene.entities[0].entity, entity.id());
        assert_eq!(scene.entities[0].components.len(), 2);
        assert!(scene.entities[0].components[0].represents::<ComponentA>());
        assert!(scene.entities[0].components[1].represents::<ComponentB>());
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
        assert_eq!(scene_entities, [entity_a_b.id(), entity_a.id()]);
    }
}
