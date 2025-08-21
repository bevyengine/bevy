use core::any::TypeId;

use crate::reflect_utils::clone_reflect_value;
use crate::{DynamicEntity, DynamicScene, SceneFilter};
use alloc::collections::BTreeMap;
use bevy_ecs::{
    component::{Component, ComponentId},
    entity_disabling::DefaultQueryFilters,
    prelude::Entity,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    resource::Resource,
    world::World,
};
use bevy_reflect::PartialReflect;
use bevy_utils::default;

/// A [`DynamicScene`] builder, used to build a scene from a [`World`] by extracting some entities and resources.
///
/// # Component Extraction
///
/// By default, all components registered with [`ReflectComponent`] type data in a world's [`AppTypeRegistry`] will be extracted.
/// (this type data is added automatically during registration if [`Reflect`] is derived with the `#[reflect(Component)]` attribute).
/// This can be changed by [specifying a filter](DynamicSceneBuilder::with_component_filter) or by explicitly
/// [allowing](DynamicSceneBuilder::allow_component)/[denying](DynamicSceneBuilder::deny_component) certain components.
///
/// Extraction happens immediately and uses the filter as it exists during the time of extraction.
///
/// # Resource Extraction
///
/// By default, all resources registered with [`ReflectResource`] type data in a world's [`AppTypeRegistry`] will be extracted.
/// (this type data is added automatically during registration if [`Reflect`] is derived with the `#[reflect(Resource)]` attribute).
/// This can be changed by [specifying a filter](DynamicSceneBuilder::with_resource_filter) or by explicitly
/// [allowing](DynamicSceneBuilder::allow_resource)/[denying](DynamicSceneBuilder::deny_resource) certain resources.
///
/// Extraction happens immediately and uses the filter as it exists during the time of extraction.
///
/// # Entity Order
///
/// Extracted entities will always be stored in ascending order based on their [index](Entity::index).
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
/// let dynamic_scene = DynamicSceneBuilder::from_world(&world).extract_entity(entity).build();
/// ```
///
/// [`Reflect`]: bevy_reflect::Reflect
pub struct DynamicSceneBuilder<'w> {
    extracted_resources: BTreeMap<ComponentId, Box<dyn PartialReflect>>,
    extracted_scene: BTreeMap<Entity, DynamicEntity>,
    component_filter: SceneFilter,
    resource_filter: SceneFilter,
    original_world: &'w World,
}

impl<'w> DynamicSceneBuilder<'w> {
    /// Prepare a builder that will extract entities and their component from the given [`World`].
    pub fn from_world(world: &'w World) -> Self {
        Self {
            extracted_resources: default(),
            extracted_scene: default(),
            component_filter: SceneFilter::default(),
            resource_filter: SceneFilter::default(),
            original_world: world,
        }
    }

    /// Specify a custom component [`SceneFilter`] to be used with this builder.
    #[must_use]
    pub fn with_component_filter(mut self, filter: SceneFilter) -> Self {
        self.component_filter = filter;
        self
    }

    /// Specify a custom resource [`SceneFilter`] to be used with this builder.
    #[must_use]
    pub fn with_resource_filter(mut self, filter: SceneFilter) -> Self {
        self.resource_filter = filter;
        self
    }

    /// Updates the filter to allow all component and resource types.
    ///
    /// This is useful for resetting the filter so that types may be selectively denied
    /// with [`deny_component`](`Self::deny_component`) and [`deny_resource`](`Self::deny_resource`).
    pub fn allow_all(mut self) -> Self {
        self.component_filter = SceneFilter::allow_all();
        self.resource_filter = SceneFilter::allow_all();
        self
    }

    /// Updates the filter to deny all component and resource types.
    ///
    /// This is useful for resetting the filter so that types may be selectively allowed
    /// with [`allow_component`](`Self::allow_component`) and [`allow_resource`](`Self::allow_resource`).
    pub fn deny_all(mut self) -> Self {
        self.component_filter = SceneFilter::deny_all();
        self.resource_filter = SceneFilter::deny_all();
        self
    }

    /// Allows the given component type, `T`, to be included in the generated scene.
    ///
    /// This method may be called multiple times for any number of components.
    ///
    /// This is the inverse of [`deny_component`](Self::deny_component).
    /// If `T` has already been denied, then it will be removed from the denylist.
    #[must_use]
    pub fn allow_component<T: Component>(mut self) -> Self {
        self.component_filter = self.component_filter.allow::<T>();
        self
    }

    /// Denies the given component type, `T`, from being included in the generated scene.
    ///
    /// This method may be called multiple times for any number of components.
    ///
    /// This is the inverse of [`allow_component`](Self::allow_component).
    /// If `T` has already been allowed, then it will be removed from the allowlist.
    #[must_use]
    pub fn deny_component<T: Component>(mut self) -> Self {
        self.component_filter = self.component_filter.deny::<T>();
        self
    }

    /// Updates the filter to allow all component types.
    ///
    /// This is useful for resetting the filter so that types may be selectively [denied].
    ///
    /// [denied]: Self::deny_component
    #[must_use]
    pub fn allow_all_components(mut self) -> Self {
        self.component_filter = SceneFilter::allow_all();
        self
    }

    /// Updates the filter to deny all component types.
    ///
    /// This is useful for resetting the filter so that types may be selectively [allowed].
    ///
    /// [allowed]: Self::allow_component
    #[must_use]
    pub fn deny_all_components(mut self) -> Self {
        self.component_filter = SceneFilter::deny_all();
        self
    }

    /// Allows the given resource type, `T`, to be included in the generated scene.
    ///
    /// This method may be called multiple times for any number of resources.
    ///
    /// This is the inverse of [`deny_resource`](Self::deny_resource).
    /// If `T` has already been denied, then it will be removed from the denylist.
    #[must_use]
    pub fn allow_resource<T: Resource>(mut self) -> Self {
        self.resource_filter = self.resource_filter.allow::<T>();
        self
    }

    /// Denies the given resource type, `T`, from being included in the generated scene.
    ///
    /// This method may be called multiple times for any number of resources.
    ///
    /// This is the inverse of [`allow_resource`](Self::allow_resource).
    /// If `T` has already been allowed, then it will be removed from the allowlist.
    #[must_use]
    pub fn deny_resource<T: Resource>(mut self) -> Self {
        self.resource_filter = self.resource_filter.deny::<T>();
        self
    }

    /// Updates the filter to allow all resource types.
    ///
    /// This is useful for resetting the filter so that types may be selectively [denied].
    ///
    /// [denied]: Self::deny_resource
    #[must_use]
    pub fn allow_all_resources(mut self) -> Self {
        self.resource_filter = SceneFilter::allow_all();
        self
    }

    /// Updates the filter to deny all resource types.
    ///
    /// This is useful for resetting the filter so that types may be selectively [allowed].
    ///
    /// [allowed]: Self::allow_resource
    #[must_use]
    pub fn deny_all_resources(mut self) -> Self {
        self.resource_filter = SceneFilter::deny_all();
        self
    }

    /// Consume the builder, producing a [`DynamicScene`].
    ///
    /// To make sure the dynamic scene doesn't contain entities without any components, call
    /// [`Self::remove_empty_entities`] before building the scene.
    #[must_use]
    pub fn build(self) -> DynamicScene {
        DynamicScene {
            resources: self.extracted_resources.into_values().collect(),
            entities: self.extracted_scene.into_values().collect(),
        }
    }

    /// Extract one entity from the builder's [`World`].
    ///
    /// Re-extracting an entity that was already extracted will have no effect.
    #[must_use]
    pub fn extract_entity(self, entity: Entity) -> Self {
        self.extract_entities(core::iter::once(entity))
    }

    /// Despawns all entities with no components.
    ///
    /// These were likely created because none of their components were present in the provided type registry upon extraction.
    #[must_use]
    pub fn remove_empty_entities(mut self) -> Self {
        self.extracted_scene
            .retain(|_, entity| !entity.components.is_empty());

        self
    }

    /// Extract entities from the builder's [`World`].
    ///
    /// Re-extracting an entity that was already extracted will have no effect.
    ///
    /// To control which components are extracted, use the [`allow`] or
    /// [`deny`] helper methods.
    ///
    /// This method may be used to extract entities from a query:
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
    /// let scene = DynamicSceneBuilder::from_world(&world)
    ///     .extract_entities(query.iter(&world))
    ///     .build();
    /// ```
    ///
    /// Note that components extracted from queried entities must still pass through the filter if one is set.
    ///
    /// [`allow`]: Self::allow_component
    /// [`deny`]: Self::deny_component
    #[must_use]
    pub fn extract_entities(mut self, entities: impl Iterator<Item = Entity>) -> Self {
        let type_registry = self.original_world.resource::<AppTypeRegistry>().read();

        for entity in entities {
            if self.extracted_scene.contains_key(&entity) {
                continue;
            }

            let mut entry = DynamicEntity {
                entity,
                components: Vec::new(),
            };

            let original_entity = self.original_world.entity(entity);
            for component_id in original_entity.archetype().iter_components() {
                let mut extract_and_push = || {
                    let type_id = self
                        .original_world
                        .components()
                        .get_info(component_id)?
                        .type_id()?;

                    let is_denied = self.component_filter.is_denied_by_id(type_id);

                    if is_denied {
                        // Component is either in the denylist or _not_ in the allowlist
                        return None;
                    }

                    let type_registration = type_registry.get(type_id)?;

                    let component = type_registration
                        .data::<ReflectComponent>()?
                        .reflect(original_entity)?;

                    let component =
                        clone_reflect_value(component.as_partial_reflect(), type_registration);

                    entry.components.push(component);
                    Some(())
                };
                extract_and_push();
            }
            self.extracted_scene.insert(entity, entry);
        }

        self
    }

    /// Extract resources from the builder's [`World`].
    ///
    /// Re-extracting a resource that was already extracted will have no effect.
    ///
    /// To control which resources are extracted, use the [`allow_resource`] or
    /// [`deny_resource`] helper methods.
    ///
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
    /// let mut builder = DynamicSceneBuilder::from_world(&world).extract_resources();
    /// let scene = builder.build();
    /// ```
    ///
    /// [`allow_resource`]: Self::allow_resource
    /// [`deny_resource`]: Self::deny_resource
    #[must_use]
    pub fn extract_resources(mut self) -> Self {
        // Don't extract the DefaultQueryFilters resource
        let original_world_dqf_id = self
            .original_world
            .components()
            .get_valid_resource_id(TypeId::of::<DefaultQueryFilters>());

        let type_registry = self.original_world.resource::<AppTypeRegistry>().read();

        for (component_id, _) in self.original_world.storages().resources.iter() {
            if Some(component_id) == original_world_dqf_id {
                continue;
            }
            let mut extract_and_push = || {
                let type_id = self
                    .original_world
                    .components()
                    .get_info(component_id)?
                    .type_id()?;

                let is_denied = self.resource_filter.is_denied_by_id(type_id);

                if is_denied {
                    // Resource is either in the denylist or _not_ in the allowlist
                    return None;
                }

                let type_registration = type_registry.get(type_id)?;

                let resource = type_registration
                    .data::<ReflectResource>()?
                    .reflect(self.original_world)
                    .ok()?;

                let resource =
                    clone_reflect_value(resource.as_partial_reflect(), type_registration);

                self.extracted_resources.insert(component_id, resource);
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
        component::Component,
        prelude::{Entity, Resource},
        query::With,
        reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
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

    #[derive(Resource, Reflect, Default, Eq, PartialEq, Debug)]
    #[reflect(Resource)]
    struct ResourceB;

    #[test]
    fn extract_one_entity() {
        let mut world = World::default();

        let atr = AppTypeRegistry::default();
        atr.write().register::<ComponentA>();
        world.insert_resource(atr);

        let entity = world.spawn((ComponentA, ComponentB)).id();

        let scene = DynamicSceneBuilder::from_world(&world)
            .extract_entity(entity)
            .build();

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

        let scene = DynamicSceneBuilder::from_world(&world)
            .extract_entity(entity)
            .extract_entity(entity)
            .build();

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

        let scene = DynamicSceneBuilder::from_world(&world)
            .extract_entity(entity)
            .build();

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

        // Insert entities out of order
        let builder = DynamicSceneBuilder::from_world(&world)
            .extract_entity(entity_b)
            .extract_entities([entity_d, entity_a].into_iter())
            .extract_entity(entity_c);

        let mut entities = builder.build().entities.into_iter();

        // Assert entities are ordered
        assert_eq!(entity_d, entities.next().map(|e| e.entity).unwrap());
        assert_eq!(entity_c, entities.next().map(|e| e.entity).unwrap());
        assert_eq!(entity_b, entities.next().map(|e| e.entity).unwrap());
        assert_eq!(entity_a, entities.next().map(|e| e.entity).unwrap());
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
        let scene = DynamicSceneBuilder::from_world(&world)
            .extract_entities(query.iter(&world))
            .build();

        assert_eq!(scene.entities.len(), 2);
        let mut scene_entities = vec![scene.entities[0].entity, scene.entities[1].entity];
        scene_entities.sort();
        assert_eq!(scene_entities, [entity_a, entity_a_b]);
    }

    #[test]
    fn remove_componentless_entity() {
        let mut world = World::default();

        let atr = AppTypeRegistry::default();
        atr.write().register::<ComponentA>();
        world.insert_resource(atr);

        let entity_a = world.spawn(ComponentA).id();
        let entity_b = world.spawn(ComponentB).id();

        let scene = DynamicSceneBuilder::from_world(&world)
            .extract_entities([entity_a, entity_b].into_iter())
            .remove_empty_entities()
            .build();

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

        let scene = DynamicSceneBuilder::from_world(&world)
            .extract_resources()
            .build();

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

        let scene = DynamicSceneBuilder::from_world(&world)
            .extract_resources()
            .extract_resources()
            .build();

        assert_eq!(scene.resources.len(), 1);
        assert!(scene.resources[0].represents::<ResourceA>());
    }

    #[test]
    fn should_extract_allowed_components() {
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
        let entity_b = world.spawn(ComponentB).id();

        let scene = DynamicSceneBuilder::from_world(&world)
            .allow_component::<ComponentA>()
            .extract_entities([entity_a_b, entity_a, entity_b].into_iter())
            .build();

        assert_eq!(scene.entities.len(), 3);
        assert!(scene.entities[2].components[0].represents::<ComponentA>());
        assert!(scene.entities[1].components[0].represents::<ComponentA>());
        assert_eq!(scene.entities[0].components.len(), 0);
    }

    #[test]
    fn should_not_extract_denied_components() {
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
        let entity_b = world.spawn(ComponentB).id();

        let scene = DynamicSceneBuilder::from_world(&world)
            .deny_component::<ComponentA>()
            .extract_entities([entity_a_b, entity_a, entity_b].into_iter())
            .build();

        assert_eq!(scene.entities.len(), 3);
        assert!(scene.entities[0].components[0].represents::<ComponentB>());
        assert_eq!(scene.entities[1].components.len(), 0);
        assert!(scene.entities[2].components[0].represents::<ComponentB>());
    }

    #[test]
    fn should_extract_allowed_resources() {
        let mut world = World::default();

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<ResourceA>();
            register.register::<ResourceB>();
        }
        world.insert_resource(atr);

        world.insert_resource(ResourceA);
        world.insert_resource(ResourceB);

        let scene = DynamicSceneBuilder::from_world(&world)
            .allow_resource::<ResourceA>()
            .extract_resources()
            .build();

        assert_eq!(scene.resources.len(), 1);
        assert!(scene.resources[0].represents::<ResourceA>());
    }

    #[test]
    fn should_not_extract_denied_resources() {
        let mut world = World::default();

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<ResourceA>();
            register.register::<ResourceB>();
        }
        world.insert_resource(atr);

        world.insert_resource(ResourceA);
        world.insert_resource(ResourceB);

        let scene = DynamicSceneBuilder::from_world(&world)
            .deny_resource::<ResourceA>()
            .extract_resources()
            .build();

        assert_eq!(scene.resources.len(), 1);
        assert!(scene.resources[0].represents::<ResourceB>());
    }

    #[test]
    fn should_use_from_reflect() {
        #[derive(Resource, Component, Reflect)]
        #[reflect(Resource, Component)]
        struct SomeType(i32);

        let mut world = World::default();
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<SomeType>();
        }
        world.insert_resource(atr);

        world.insert_resource(SomeType(123));
        let entity = world.spawn(SomeType(123)).id();

        let scene = DynamicSceneBuilder::from_world(&world)
            .extract_resources()
            .extract_entities(vec![entity].into_iter())
            .build();

        let component = &scene.entities[0].components[0];
        assert!(component
            .try_as_reflect()
            .expect("component should be concrete due to `FromReflect`")
            .is::<SomeType>());

        let resource = &scene.resources[0];
        assert!(resource
            .try_as_reflect()
            .expect("resource should be concrete due to `FromReflect`")
            .is::<SomeType>());
    }
}
