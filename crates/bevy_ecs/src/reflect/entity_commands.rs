use crate::{
    entity::Entity,
    prelude::Mut,
    reflect::{AppTypeRegistry, ReflectBundle, ReflectComponent},
    system::{EntityCommands, Resource},
    world::{Command, World},
};
use alloc::borrow::Cow;
use bevy_reflect::{PartialReflect, TypeRegistry};
use core::marker::PhantomData;

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
    ///     [`Component`](crate::component::Component) or [`Bundle`](crate::bundle::Bundle).
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
    ///         .insert_reflect(prefab.data.clone_value());
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

    /// Removes from the entity the component or bundle with the given type name registered in [`AppTypeRegistry`].
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
    fn remove_reflect(&mut self, component_type_name: impl Into<Cow<'static, str>>) -> &mut Self;
    /// Same as [`remove_reflect`](ReflectCommandExt::remove_reflect), but using the `T` resource as type registry instead of
    /// `AppTypeRegistry`.
    fn remove_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component_type_name: impl Into<Cow<'static, str>>,
    ) -> &mut Self;
}

impl ReflectCommandExt for EntityCommands<'_> {
    fn insert_reflect(&mut self, component: Box<dyn PartialReflect>) -> &mut Self {
        self.commands.queue(InsertReflect {
            entity: self.entity,
            component,
        });
        self
    }

    fn insert_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component: Box<dyn PartialReflect>,
    ) -> &mut Self {
        self.commands.queue(InsertReflectWithRegistry::<T> {
            entity: self.entity,
            _t: PhantomData,
            component,
        });
        self
    }

    fn remove_reflect(&mut self, component_type_path: impl Into<Cow<'static, str>>) -> &mut Self {
        self.commands.queue(RemoveReflect {
            entity: self.entity,
            component_type_path: component_type_path.into(),
        });
        self
    }

    fn remove_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component_type_name: impl Into<Cow<'static, str>>,
    ) -> &mut Self {
        self.commands.queue(RemoveReflectWithRegistry::<T> {
            entity: self.entity,
            _t: PhantomData,
            component_type_name: component_type_name.into(),
        });
        self
    }
}

/// Helper function to add a reflect component or bundle to a given entity
fn insert_reflect(
    world: &mut World,
    entity: Entity,
    type_registry: &TypeRegistry,
    component: Box<dyn PartialReflect>,
) {
    let type_info = component
        .get_represented_type_info()
        .expect("component should represent a type.");
    let type_path = type_info.type_path();
    let Some(mut entity) = world.get_entity_mut(entity) else {
        panic!("error[B0003]: Could not insert a reflected component (of type {type_path}) for entity {entity:?} because it doesn't exist in this World. See: https://bevyengine.org/learn/errors/b0003");
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

/// A [`Command`] that adds the boxed reflect component or bundle to an entity using the data in
/// [`AppTypeRegistry`].
///
/// See [`ReflectCommandExt::insert_reflect`] for details.
pub struct InsertReflect {
    /// The entity on which the component will be inserted.
    pub entity: Entity,
    /// The reflect [`Component`](crate::component::Component) or [`Bundle`](crate::bundle::Bundle)
    /// that will be added to the entity.
    pub component: Box<dyn PartialReflect>,
}

impl Command for InsertReflect {
    fn apply(self, world: &mut World) {
        let registry = world.get_resource::<AppTypeRegistry>().unwrap().clone();
        insert_reflect(world, self.entity, &registry.read(), self.component);
    }
}

/// A [`Command`] that adds the boxed reflect component or bundle to an entity using the data in the provided
/// [`Resource`] that implements [`AsRef<TypeRegistry>`].
///
/// See [`ReflectCommandExt::insert_reflect_with_registry`] for details.
pub struct InsertReflectWithRegistry<T: Resource + AsRef<TypeRegistry>> {
    /// The entity on which the component will be inserted.
    pub entity: Entity,
    pub _t: PhantomData<T>,
    /// The reflect [`Component`](crate::component::Component) that will be added to the entity.
    pub component: Box<dyn PartialReflect>,
}

impl<T: Resource + AsRef<TypeRegistry>> Command for InsertReflectWithRegistry<T> {
    fn apply(self, world: &mut World) {
        world.resource_scope(|world, registry: Mut<T>| {
            let registry: &TypeRegistry = registry.as_ref().as_ref();
            insert_reflect(world, self.entity, registry, self.component);
        });
    }
}

/// Helper function to remove a reflect component or bundle from a given entity
fn remove_reflect(
    world: &mut World,
    entity: Entity,
    type_registry: &TypeRegistry,
    component_type_path: Cow<'static, str>,
) {
    let Some(mut entity) = world.get_entity_mut(entity) else {
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

/// A [`Command`] that removes the component or bundle of the same type as the given type name from
/// the provided entity.
///
/// See [`ReflectCommandExt::remove_reflect`] for details.
pub struct RemoveReflect {
    /// The entity from which the component will be removed.
    pub entity: Entity,
    /// The [`Component`](crate::component::Component) or [`Bundle`](crate::bundle::Bundle)
    /// type name that will be used to remove a component
    /// of the same type from the entity.
    pub component_type_path: Cow<'static, str>,
}

impl Command for RemoveReflect {
    fn apply(self, world: &mut World) {
        let registry = world.get_resource::<AppTypeRegistry>().unwrap().clone();
        remove_reflect(
            world,
            self.entity,
            &registry.read(),
            self.component_type_path,
        );
    }
}

/// A [`Command`] that removes the component or bundle of the same type as the given type name from
/// the provided entity using the provided [`Resource`] that implements [`AsRef<TypeRegistry>`].
///
/// See [`ReflectCommandExt::remove_reflect_with_registry`] for details.
pub struct RemoveReflectWithRegistry<T: Resource + AsRef<TypeRegistry>> {
    /// The entity from which the component will be removed.
    pub entity: Entity,
    pub _t: PhantomData<T>,
    /// The [`Component`](crate::component::Component) or [`Bundle`](crate::bundle::Bundle)
    /// type name that will be used to remove a component
    /// of the same type from the entity.
    pub component_type_name: Cow<'static, str>,
}

impl<T: Resource + AsRef<TypeRegistry>> Command for RemoveReflectWithRegistry<T> {
    fn apply(self, world: &mut World) {
        world.resource_scope(|world, registry: Mut<T>| {
            let registry: &TypeRegistry = registry.as_ref().as_ref();
            remove_reflect(world, self.entity, registry, self.component_type_name);
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        bundle::Bundle,
        component::Component,
        prelude::{AppTypeRegistry, ReflectComponent},
        reflect::{ReflectBundle, ReflectCommandExt},
        system::{Commands, SystemState},
        world::World,
    };
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

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn PartialReflect>;
        let boxed_reflect_component_a_clone = boxed_reflect_component_a.clone_value();

        commands
            .entity(entity)
            .insert_reflect(boxed_reflect_component_a);
        commands
            .entity(entity2)
            .insert_reflect(boxed_reflect_component_a_clone);
        system_state.apply(&mut world);

        assert_eq!(
            world.entity(entity).get::<ComponentA>(),
            world.entity(entity2).get::<ComponentA>()
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
