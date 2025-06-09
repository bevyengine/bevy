use crate::{
    entity::Entity,
    prelude::Mut,
    reflect::{AppTypeRegistry, ReflectBundle, ReflectComponent},
    resource::Resource,
    system::EntityCommands,
    world::{EntityWorldMut, World},
};
use alloc::{borrow::Cow, boxed::Box};
use bevy_reflect::{PartialReflect, TypeRegistry};

/// An extension trait for [`EntityCommands`] for reflection related functions
pub trait ReflectCommandExt {
    /// Adds the given boxed reflect component or bundle to the entity using the reflection data in
    /// [`AppTypeRegistry`].
    ///
    /// This will overwrite any previous component(s) of the same type.
    ///
    /// # Panics
    ///
    /// - If the entity doesn't exist.
    /// - If [`AppTypeRegistry`] does not have the reflection data for the given
    ///   [`Component`](crate::component::Component) or [`Bundle`](crate::bundle::Bundle).
    /// - If the component or bundle data is invalid. See [`PartialReflect::apply`] for further details.
    /// - If [`AppTypeRegistry`] is not present in the [`World`].
    ///
    /// # Note
    ///
    /// Prefer to use the typed [`EntityCommands::insert`] if possible. Adding a reflected component
    /// is much slower.
    ///
    /// # Example
    ///
    /// ```
    /// // Note that you need to register the component type in the AppTypeRegistry prior to using
    /// // reflection. You can use the helpers on the App with `app.register_type::<ComponentA>()`
    /// // or write to the TypeRegistry directly to register all your components
    ///
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::reflect::{ReflectCommandExt, ReflectBundle};
    /// # use bevy_reflect::{FromReflect, FromType, Reflect, TypeRegistry};
    /// // A resource that can hold any component that implements reflect as a boxed reflect component
    /// #[derive(Resource)]
    /// struct Prefab {
    ///     data: Box<dyn Reflect>,
    /// }
    /// #[derive(Component, Reflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentA(u32);
    ///
    /// #[derive(Component, Reflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentB(String);
    ///
    /// #[derive(Bundle, Reflect, Default)]
    /// #[reflect(Bundle)]
    /// struct BundleA {
    ///     a: ComponentA,
    ///     b: ComponentB,
    /// }
    ///
    /// fn insert_reflect_component(
    ///     mut commands: Commands,
    ///     mut prefab: ResMut<Prefab>
    ///     ) {
    ///     // Create a set of new boxed reflect components to use
    ///     let boxed_reflect_component_a: Box<dyn Reflect> = Box::new(ComponentA(916));
    ///     let boxed_reflect_component_b: Box<dyn Reflect>  = Box::new(ComponentB("NineSixteen".to_string()));
    ///     let boxed_reflect_bundle_a: Box<dyn Reflect> = Box::new(BundleA {
    ///         a: ComponentA(24),
    ///         b: ComponentB("Twenty-Four".to_string()),
    ///     });
    ///
    ///     // You can overwrite the component in the resource with either ComponentA or ComponentB
    ///     prefab.data = boxed_reflect_component_a;
    ///     prefab.data = boxed_reflect_component_b;
    ///
    ///     // Or even with BundleA
    ///     prefab.data = boxed_reflect_bundle_a;
    ///
    ///     // No matter which component or bundle is in the resource and without knowing the exact type, you can
    ///     // use the insert_reflect entity command to insert that component/bundle into an entity.
    ///     commands
    ///         .spawn_empty()
    ///         .insert_reflect(prefab.data.reflect_clone().unwrap().into_partial_reflect());
    /// }
    /// ```
    fn insert_reflect(&mut self, component: Box<dyn PartialReflect>) -> &mut Self;

    /// Same as [`insert_reflect`](ReflectCommandExt::insert_reflect), but using the `T` resource as type registry instead of
    /// `AppTypeRegistry`.
    ///
    /// # Panics
    ///
    /// - If the given [`Resource`] is not present in the [`World`].
    ///
    /// # Note
    ///
    /// - The given [`Resource`] is removed from the [`World`] before the command is applied.
    fn insert_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component: Box<dyn PartialReflect>,
    ) -> &mut Self;

    /// Removes from the entity the component or bundle with the given type path registered in [`AppTypeRegistry`].
    ///
    /// If the type is a bundle, it will remove any components in that bundle regardless if the entity
    /// contains all the components.
    ///
    /// Does nothing if the type is a component and the entity does not have a component of the same type,
    /// if the type is a bundle and the entity does not contain any of the components in the bundle,
    /// if [`AppTypeRegistry`] does not contain the reflection data for the given component,
    /// or if the entity does not exist.
    ///
    /// # Note
    ///
    /// Prefer to use the typed [`EntityCommands::remove`] if possible. Removing a reflected component
    /// is much slower.
    ///
    /// # Example
    ///
    /// ```
    /// // Note that you need to register the component/bundle type in the AppTypeRegistry prior to using
    /// // reflection. You can use the helpers on the App with `app.register_type::<ComponentA>()`
    /// // or write to the TypeRegistry directly to register all your components and bundles
    ///
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::reflect::{ReflectCommandExt, ReflectBundle};
    /// # use bevy_reflect::{FromReflect, FromType, Reflect, TypeRegistry};
    ///
    /// // A resource that can hold any component or bundle that implements reflect as a boxed reflect
    /// #[derive(Resource)]
    /// struct Prefab{
    ///     entity: Entity,
    ///     data: Box<dyn Reflect>,
    /// }
    /// #[derive(Component, Reflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Reflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentB(String);
    /// #[derive(Bundle, Reflect, Default)]
    /// #[reflect(Bundle)]
    /// struct BundleA {
    ///     a: ComponentA,
    ///     b: ComponentB,
    /// }
    ///
    /// fn remove_reflect_component(
    ///     mut commands: Commands,
    ///     prefab: Res<Prefab>
    ///     ) {
    ///     // Prefab can hold any boxed reflect component or bundle. In this case either
    ///     // ComponentA, ComponentB, or BundleA. No matter which component or bundle is in the resource though,
    ///     // we can attempt to remove any component (or set of components in the case of a bundle)
    ///     // of that same type from an entity.
    ///     commands.entity(prefab.entity)
    ///         .remove_reflect(prefab.data.reflect_type_path().to_owned());
    /// }
    /// ```
    fn remove_reflect(&mut self, component_type_path: impl Into<Cow<'static, str>>) -> &mut Self;
    /// Same as [`remove_reflect`](ReflectCommandExt::remove_reflect), but using the `T` resource as type registry instead of
    /// `AppTypeRegistry`.
    fn remove_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component_type_path: impl Into<Cow<'static, str>>,
    ) -> &mut Self;
}

impl ReflectCommandExt for EntityCommands<'_> {
    fn insert_reflect(&mut self, component: Box<dyn PartialReflect>) -> &mut Self {
        self.queue(move |mut entity: EntityWorldMut| {
            entity.insert_reflect(component);
        })
    }

    fn insert_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component: Box<dyn PartialReflect>,
    ) -> &mut Self {
        self.queue(move |mut entity: EntityWorldMut| {
            entity.insert_reflect_with_registry::<T>(component);
        })
    }

    fn remove_reflect(&mut self, component_type_path: impl Into<Cow<'static, str>>) -> &mut Self {
        let component_type_path: Cow<'static, str> = component_type_path.into();
        self.queue(move |mut entity: EntityWorldMut| {
            entity.remove_reflect(component_type_path);
        })
    }

    fn remove_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component_type_path: impl Into<Cow<'static, str>>,
    ) -> &mut Self {
        let component_type_path: Cow<'static, str> = component_type_path.into();
        self.queue(move |mut entity: EntityWorldMut| {
            entity.remove_reflect_with_registry::<T>(component_type_path);
        })
    }
}

impl<'w> EntityWorldMut<'w> {
    /// Adds the given boxed reflect component or bundle to the entity using the reflection data in
    /// [`AppTypeRegistry`].
    ///
    /// This will overwrite any previous component(s) of the same type.
    ///
    /// # Panics
    ///
    /// - If the entity has been despawned while this `EntityWorldMut` is still alive.
    /// - If [`AppTypeRegistry`] does not have the reflection data for the given
    ///   [`Component`](crate::component::Component) or [`Bundle`](crate::bundle::Bundle).
    /// - If the component or bundle data is invalid. See [`PartialReflect::apply`] for further details.
    /// - If [`AppTypeRegistry`] is not present in the [`World`].
    ///
    /// # Note
    ///
    /// Prefer to use the typed [`EntityWorldMut::insert`] if possible. Adding a reflected component
    /// is much slower.
    pub fn insert_reflect(&mut self, component: Box<dyn PartialReflect>) -> &mut Self {
        self.assert_not_despawned();
        let entity_id = self.id();
        self.world_scope(|world| {
            world.resource_scope(|world, registry: Mut<AppTypeRegistry>| {
                let type_registry = &registry.as_ref().read();
                insert_reflect_with_registry_ref(world, entity_id, type_registry, component);
            });
            world.flush();
        });
        self.update_location();
        self
    }

    /// Same as [`insert_reflect`](EntityWorldMut::insert_reflect), but using
    /// the `T` resource as type registry instead of [`AppTypeRegistry`].
    ///
    /// This will overwrite any previous component(s) of the same type.
    ///
    /// # Panics
    ///
    /// - If the entity has been despawned while this `EntityWorldMut` is still alive.
    /// - If the given [`Resource`] does not have the reflection data for the given
    ///   [`Component`](crate::component::Component) or [`Bundle`](crate::bundle::Bundle).
    /// - If the component or bundle data is invalid. See [`PartialReflect::apply`] for further details.
    /// - If the given [`Resource`] is not present in the [`World`].
    pub fn insert_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component: Box<dyn PartialReflect>,
    ) -> &mut Self {
        self.assert_not_despawned();
        let entity_id = self.id();
        self.world_scope(|world| {
            world.resource_scope(|world, registry: Mut<T>| {
                let type_registry = registry.as_ref().as_ref();
                insert_reflect_with_registry_ref(world, entity_id, type_registry, component);
            });
            world.flush();
        });
        self.update_location();
        self
    }

    /// Removes from the entity the component or bundle with the given type path registered in [`AppTypeRegistry`].
    ///
    /// If the type is a bundle, it will remove any components in that bundle regardless if the entity
    /// contains all the components.
    ///
    /// Does nothing if the type is a component and the entity does not have a component of the same type,
    /// if the type is a bundle and the entity does not contain any of the components in the bundle,
    /// or if [`AppTypeRegistry`] does not contain the reflection data for the given component.
    ///
    /// # Panics
    ///
    /// - If the entity has been despawned while this `EntityWorldMut` is still alive.
    /// - If [`AppTypeRegistry`] is not present in the [`World`].
    ///
    /// # Note
    ///
    /// Prefer to use the typed [`EntityCommands::remove`] if possible. Removing a reflected component
    /// is much slower.
    pub fn remove_reflect(&mut self, component_type_path: Cow<'static, str>) -> &mut Self {
        self.assert_not_despawned();
        let entity_id = self.id();
        self.world_scope(|world| {
            world.resource_scope(|world, registry: Mut<AppTypeRegistry>| {
                let type_registry = &registry.as_ref().read();
                remove_reflect_with_registry_ref(
                    world,
                    entity_id,
                    type_registry,
                    component_type_path,
                );
            });
            world.flush();
        });
        self.update_location();
        self
    }

    /// Same as [`remove_reflect`](EntityWorldMut::remove_reflect), but using
    /// the `T` resource as type registry instead of `AppTypeRegistry`.
    ///
    /// If the given type is a bundle, it will remove any components in that bundle regardless if the entity
    /// contains all the components.
    ///
    /// Does nothing if the type is a component and the entity does not have a component of the same type,
    /// if the type is a bundle and the entity does not contain any of the components in the bundle,
    /// or if [`AppTypeRegistry`] does not contain the reflection data for the given component.
    ///
    /// # Panics
    ///
    /// - If the entity has been despawned while this `EntityWorldMut` is still alive.
    /// - If [`AppTypeRegistry`] is not present in the [`World`].
    pub fn remove_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component_type_path: Cow<'static, str>,
    ) -> &mut Self {
        self.assert_not_despawned();
        let entity_id = self.id();
        self.world_scope(|world| {
            world.resource_scope(|world, registry: Mut<T>| {
                let type_registry = registry.as_ref().as_ref();
                remove_reflect_with_registry_ref(
                    world,
                    entity_id,
                    type_registry,
                    component_type_path,
                );
            });
            world.flush();
        });
        self.update_location();
        self
    }
}

/// Helper function to add a reflect component or bundle to a given entity
fn insert_reflect_with_registry_ref(
    world: &mut World,
    entity: Entity,
    type_registry: &TypeRegistry,
    component: Box<dyn PartialReflect>,
) {
    let type_info = component
        .get_represented_type_info()
        .expect("component should represent a type.");
    let type_path = type_info.type_path();
    let Ok(mut entity) = world.get_entity_mut(entity) else {
        panic!("error[B0003]: Could not insert a reflected component (of type {type_path}) for entity {entity}, which {}. See: https://bevy.org/learn/errors/b0003",
        world.entities().entity_does_not_exist_error_details(entity));
    };
    let Some(type_registration) = type_registry.get(type_info.type_id()) else {
        panic!("`{type_path}` should be registered in type registry via `App::register_type<{type_path}>`");
    };

    if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
        reflect_component.insert(&mut entity, component.as_partial_reflect(), type_registry);
    } else if let Some(reflect_bundle) = type_registration.data::<ReflectBundle>() {
        reflect_bundle.insert(&mut entity, component.as_partial_reflect(), type_registry);
    } else {
        panic!("`{type_path}` should have #[reflect(Component)] or #[reflect(Bundle)]");
    }
}

/// Helper function to remove a reflect component or bundle from a given entity
fn remove_reflect_with_registry_ref(
    world: &mut World,
    entity: Entity,
    type_registry: &TypeRegistry,
    component_type_path: Cow<'static, str>,
) {
    let Ok(mut entity) = world.get_entity_mut(entity) else {
        return;
    };
    let Some(type_registration) = type_registry.get_with_type_path(&component_type_path) else {
        return;
    };
    if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
        reflect_component.remove(&mut entity);
    } else if let Some(reflect_bundle) = type_registration.data::<ReflectBundle>() {
        reflect_bundle.remove(&mut entity);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        bundle::Bundle,
        component::Component,
        prelude::{AppTypeRegistry, ReflectComponent},
        reflect::{ReflectBundle, ReflectCommandExt},
        system::{Commands, SystemState},
        world::World,
    };
    use alloc::{borrow::ToOwned, boxed::Box};
    use bevy_ecs_macros::Resource;
    use bevy_reflect::{PartialReflect, Reflect, TypeRegistry};

    #[derive(Resource)]
    struct TypeRegistryResource {
        type_registry: TypeRegistry,
    }

    impl AsRef<TypeRegistry> for TypeRegistryResource {
        fn as_ref(&self) -> &TypeRegistry {
            &self.type_registry
        }
    }

    #[derive(Component, Reflect, Default, PartialEq, Eq, Debug)]
    #[reflect(Component)]
    struct ComponentA(u32);

    #[derive(Component, Reflect, Default, PartialEq, Eq, Debug)]
    #[reflect(Component)]
    struct ComponentB(u32);

    #[derive(Bundle, Reflect, Default, Debug, PartialEq)]
    #[reflect(Bundle)]
    struct BundleA {
        a: ComponentA,
        b: ComponentB,
    }

    #[test]
    fn insert_reflected() {
        let mut world = World::new();

        let type_registry = AppTypeRegistry::default();
        {
            let mut registry = type_registry.write();
            registry.register::<ComponentA>();
            registry.register_type_data::<ComponentA, ReflectComponent>();
        }
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands.spawn_empty().id();
        let entity2 = commands.spawn_empty().id();
        let entity3 = commands.spawn_empty().id();

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn PartialReflect>;
        let boxed_reflect_component_a_clone = boxed_reflect_component_a.reflect_clone().unwrap();
        let boxed_reflect_component_a_dynamic = boxed_reflect_component_a.to_dynamic();

        commands
            .entity(entity)
            .insert_reflect(boxed_reflect_component_a);
        commands
            .entity(entity2)
            .insert_reflect(boxed_reflect_component_a_clone.into_partial_reflect());
        commands
            .entity(entity3)
            .insert_reflect(boxed_reflect_component_a_dynamic);
        system_state.apply(&mut world);

        assert_eq!(
            world.entity(entity).get::<ComponentA>(),
            world.entity(entity2).get::<ComponentA>(),
        );
        assert_eq!(
            world.entity(entity).get::<ComponentA>(),
            world.entity(entity3).get::<ComponentA>(),
        );
    }

    #[test]
    fn insert_reflected_with_registry() {
        let mut world = World::new();

        let mut type_registry = TypeRegistryResource {
            type_registry: TypeRegistry::new(),
        };

        type_registry.type_registry.register::<ComponentA>();
        type_registry
            .type_registry
            .register_type_data::<ComponentA, ReflectComponent>();
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands.spawn_empty().id();

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn PartialReflect>;

        commands
            .entity(entity)
            .insert_reflect_with_registry::<TypeRegistryResource>(boxed_reflect_component_a);
        system_state.apply(&mut world);

        assert_eq!(
            world.entity(entity).get::<ComponentA>(),
            Some(&ComponentA(916))
        );
    }

    #[test]
    fn remove_reflected() {
        let mut world = World::new();

        let type_registry = AppTypeRegistry::default();
        {
            let mut registry = type_registry.write();
            registry.register::<ComponentA>();
            registry.register_type_data::<ComponentA, ReflectComponent>();
        }
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands.spawn(ComponentA(0)).id();

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;

        commands
            .entity(entity)
            .remove_reflect(boxed_reflect_component_a.reflect_type_path().to_owned());
        system_state.apply(&mut world);

        assert_eq!(world.entity(entity).get::<ComponentA>(), None);
    }

    #[test]
    fn remove_reflected_with_registry() {
        let mut world = World::new();

        let mut type_registry = TypeRegistryResource {
            type_registry: TypeRegistry::new(),
        };

        type_registry.type_registry.register::<ComponentA>();
        type_registry
            .type_registry
            .register_type_data::<ComponentA, ReflectComponent>();
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands.spawn(ComponentA(0)).id();

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;

        commands
            .entity(entity)
            .remove_reflect_with_registry::<TypeRegistryResource>(
                boxed_reflect_component_a.reflect_type_path().to_owned(),
            );
        system_state.apply(&mut world);

        assert_eq!(world.entity(entity).get::<ComponentA>(), None);
    }

    #[test]
    fn insert_reflect_bundle() {
        let mut world = World::new();

        let type_registry = AppTypeRegistry::default();
        {
            let mut registry = type_registry.write();
            registry.register::<BundleA>();
            registry.register_type_data::<BundleA, ReflectBundle>();
        }
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands.spawn_empty().id();
        let bundle = Box::new(BundleA {
            a: ComponentA(31),
            b: ComponentB(20),
        }) as Box<dyn PartialReflect>;
        commands.entity(entity).insert_reflect(bundle);

        system_state.apply(&mut world);

        assert_eq!(world.get::<ComponentA>(entity), Some(&ComponentA(31)));
        assert_eq!(world.get::<ComponentB>(entity), Some(&ComponentB(20)));
    }

    #[test]
    fn insert_reflect_bundle_with_registry() {
        let mut world = World::new();

        let mut type_registry = TypeRegistryResource {
            type_registry: TypeRegistry::new(),
        };

        type_registry.type_registry.register::<BundleA>();
        type_registry
            .type_registry
            .register_type_data::<BundleA, ReflectBundle>();
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands.spawn_empty().id();
        let bundle = Box::new(BundleA {
            a: ComponentA(31),
            b: ComponentB(20),
        }) as Box<dyn PartialReflect>;

        commands
            .entity(entity)
            .insert_reflect_with_registry::<TypeRegistryResource>(bundle);
        system_state.apply(&mut world);

        assert_eq!(world.get::<ComponentA>(entity), Some(&ComponentA(31)));
        assert_eq!(world.get::<ComponentB>(entity), Some(&ComponentB(20)));
    }

    #[test]
    fn remove_reflected_bundle() {
        let mut world = World::new();

        let type_registry = AppTypeRegistry::default();
        {
            let mut registry = type_registry.write();
            registry.register::<BundleA>();
            registry.register_type_data::<BundleA, ReflectBundle>();
        }
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands
            .spawn(BundleA {
                a: ComponentA(31),
                b: ComponentB(20),
            })
            .id();

        let boxed_reflect_bundle_a = Box::new(BundleA {
            a: ComponentA(1),
            b: ComponentB(23),
        }) as Box<dyn Reflect>;

        commands
            .entity(entity)
            .remove_reflect(boxed_reflect_bundle_a.reflect_type_path().to_owned());
        system_state.apply(&mut world);

        assert_eq!(world.entity(entity).get::<ComponentA>(), None);
        assert_eq!(world.entity(entity).get::<ComponentB>(), None);
    }

    #[test]
    fn remove_reflected_bundle_with_registry() {
        let mut world = World::new();

        let mut type_registry = TypeRegistryResource {
            type_registry: TypeRegistry::new(),
        };

        type_registry.type_registry.register::<BundleA>();
        type_registry
            .type_registry
            .register_type_data::<BundleA, ReflectBundle>();
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands
            .spawn(BundleA {
                a: ComponentA(31),
                b: ComponentB(20),
            })
            .id();

        let boxed_reflect_bundle_a = Box::new(BundleA {
            a: ComponentA(1),
            b: ComponentB(23),
        }) as Box<dyn Reflect>;

        commands
            .entity(entity)
            .remove_reflect_with_registry::<TypeRegistryResource>(
                boxed_reflect_bundle_a.reflect_type_path().to_owned(),
            );
        system_state.apply(&mut world);

        assert_eq!(world.entity(entity).get::<ComponentA>(), None);
        assert_eq!(world.entity(entity).get::<ComponentB>(), None);
    }
}
