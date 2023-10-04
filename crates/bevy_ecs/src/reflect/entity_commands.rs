use crate::prelude::Mut;
use crate::reflect::AppTypeRegistry;
use crate::system::{Command, EntityCommands, Resource};
use crate::{entity::Entity, reflect::ReflectComponent, world::World};
use bevy_reflect::{Reflect, TypeRegistry};
use std::borrow::Cow;
use std::marker::PhantomData;

/// An extension trait for [`EntityCommands`] for reflection related functions
pub trait ReflectCommandExt {
    /// Adds the given boxed reflect component to the entity using the reflection data in
    /// [`AppTypeRegistry`].
    ///
    /// This will overwrite any previous component of the same type.
    ///
    /// # Panics
    ///
    /// - If the entity doesn't exist.
    /// - If [`AppTypeRegistry`] does not have the reflection data for the given [`Component`](crate::component::Component).
    /// - If the component data is invalid. See [`Reflect::apply`] for further details.
    /// - If [`AppTypeRegistry`] is not present in the [`World`].
    ///
    /// # Note
    ///
    /// Prefer to use the typed [`EntityCommands::insert`] if possible. Adding a reflected component
    /// is much slower.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Note that you need to register the component type in the AppTypeRegistry prior to using
    /// // reflection. You can use the helpers on the App with `app.register_type::<ComponentA>()`
    /// // or write to the TypeRegistry directly to register all your components
    ///
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::reflect::ReflectCommandExt;
    /// # use bevy_reflect::{FromReflect, FromType, Reflect, TypeRegistry};
    /// // A resource that can hold any component that implements reflect as a boxed reflect component
    /// #[derive(Resource)]
    /// struct Prefab{
    ///     component: Box<dyn Reflect>,
    /// }
    /// #[derive(Component, Reflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentA(u32);
    ///
    /// #[derive(Component, Reflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentB(String);
    ///
    /// fn insert_reflect_component(
    ///     mut commands: Commands,
    ///     mut prefab: ResMut<Prefab>
    ///     ) {
    ///     // Create a set of new boxed reflect components to use
    ///     let boxed_reflect_component_a: Box<dyn Reflect> = Box::new(ComponentA(916));
    ///     let boxed_reflect_component_b: Box<dyn Reflect>  = Box::new(ComponentB("NineSixteen".to_string()));
    ///
    ///     // You can overwrite the component in the resource with either ComponentA or ComponentB
    ///     prefab.component = boxed_reflect_component_a;
    ///     prefab.component = boxed_reflect_component_b;
    ///     
    ///     // No matter which component is in the resource and without knowing the exact type, you can
    ///     // use the insert_reflect entity command to insert that component into an entity.
    ///     commands
    ///         .spawn_empty()
    ///         .insert_reflect(prefab.component.clone_value());
    /// }
    ///
    /// ```
    fn insert_reflect(&mut self, component: Box<dyn Reflect>) -> &mut Self;

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
        component: Box<dyn Reflect>,
    ) -> &mut Self;

    /// Removes from the entity the component with the given type name registered in [`AppTypeRegistry`].
    ///
    /// Does nothing if the entity does not have a component of the same type, if [`AppTypeRegistry`]
    /// does not contain the reflection data for the given component, or if the entity does not exist.
    ///
    /// # Note
    ///
    /// Prefer to use the typed [`EntityCommands::remove`] if possible. Removing a reflected component
    /// is much slower.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Note that you need to register the component type in the AppTypeRegistry prior to using
    /// // reflection. You can use the helpers on the App with `app.register_type::<ComponentA>()`
    /// // or write to the TypeRegistry directly to register all your components
    ///
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::reflect::ReflectCommandExt;
    /// # use bevy_reflect::{FromReflect, FromType, Reflect, TypeRegistry};
    ///
    /// // A resource that can hold any component that implements reflect as a boxed reflect component
    /// #[derive(Resource)]
    /// struct Prefab{
    ///     entity: Entity,
    ///     component: Box<dyn Reflect>,
    /// }
    /// #[derive(Component, Reflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Reflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentB(String);
    ///
    /// fn remove_reflect_component(
    ///     mut commands: Commands,
    ///     prefab: Res<Prefab>
    ///     ) {
    ///     // Prefab can hold any boxed reflect component. In this case either
    ///     // ComponentA or ComponentB. No matter which component is in the resource though,
    ///     // we can attempt to remove any component of that same type from an entity.
    ///     commands.entity(prefab.entity)
    ///         .remove_reflect(prefab.component.type_name().to_owned());
    /// }
    ///
    /// ```
    fn remove_reflect(&mut self, component_type_name: impl Into<Cow<'static, str>>) -> &mut Self;
    /// Same as [`remove_reflect`](ReflectCommandExt::remove_reflect), but using the `T` resource as type registry instead of
    /// `AppTypeRegistry`.
    fn remove_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component_type_name: impl Into<Cow<'static, str>>,
    ) -> &mut Self;
}

impl<'w, 's, 'a> ReflectCommandExt for EntityCommands<'w, 's, 'a> {
    fn insert_reflect(&mut self, component: Box<dyn Reflect>) -> &mut Self {
        self.commands.add(InsertReflect {
            entity: self.entity,
            component,
        });
        self
    }

    fn insert_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component: Box<dyn Reflect>,
    ) -> &mut Self {
        self.commands.add(InsertReflectWithRegistry::<T> {
            entity: self.entity,
            _t: PhantomData,
            component,
        });
        self
    }

    fn remove_reflect(&mut self, component_type_name: impl Into<Cow<'static, str>>) -> &mut Self {
        self.commands.add(RemoveReflect {
            entity: self.entity,
            component_type_name: component_type_name.into(),
        });
        self
    }

    fn remove_reflect_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component_type_name: impl Into<Cow<'static, str>>,
    ) -> &mut Self {
        self.commands.add(RemoveReflectWithRegistry::<T> {
            entity: self.entity,
            _t: PhantomData,
            component_type_name: component_type_name.into(),
        });
        self
    }
}

/// Helper function to add a reflect component to a given entity
fn insert_reflect(
    world: &mut World,
    entity: Entity,
    type_registry: &TypeRegistry,
    component: Box<dyn Reflect>,
) {
    let type_info = component.type_name();
    let Some(mut entity) = world.get_entity_mut(entity) else {
        panic!("error[B0003]: Could not insert a reflected component (of type {}) for entity {entity:?} because it doesn't exist in this World.", component.type_name());
    };
    let Some(type_registration) = type_registry.get_with_name(type_info) else {
        panic!("Could not get type registration (for component type {}) because it doesn't exist in the TypeRegistry.", component.type_name());
    };
    let Some(reflect_component) = type_registration.data::<ReflectComponent>() else {
        panic!("Could not get ReflectComponent data (for component type {}) because it doesn't exist in this TypeRegistration.", component.type_name());
    };
    reflect_component.insert(&mut entity, &*component);
}

/// A [`Command`] that adds the boxed reflect component to an entity using the data in
/// [`AppTypeRegistry`].
///
/// See [`ReflectCommandExt::insert_reflect`] for details.
pub struct InsertReflect {
    /// The entity on which the component will be inserted.
    pub entity: Entity,
    /// The reflect [`Component`](crate::component::Component) that will be added to the entity.
    pub component: Box<dyn Reflect>,
}

impl Command for InsertReflect {
    fn apply(self, world: &mut World) {
        let registry = world.get_resource::<AppTypeRegistry>().unwrap().clone();
        insert_reflect(world, self.entity, &registry.read(), self.component);
    }
}

/// A [`Command`] that adds the boxed reflect component to an entity using the data in the provided
/// [`Resource`] that implements [`AsRef<TypeRegistry>`].
///
/// See [`ReflectCommandExt::insert_reflect_with_registry`] for details.
pub struct InsertReflectWithRegistry<T: Resource + AsRef<TypeRegistry>> {
    /// The entity on which the component will be inserted.
    pub entity: Entity,
    pub _t: PhantomData<T>,
    /// The reflect [`Component`](crate::component::Component) that will be added to the entity.
    pub component: Box<dyn Reflect>,
}

impl<T: Resource + AsRef<TypeRegistry>> Command for InsertReflectWithRegistry<T> {
    fn apply(self, world: &mut World) {
        world.resource_scope(|world, registry: Mut<T>| {
            let registry: &TypeRegistry = registry.as_ref().as_ref();
            insert_reflect(world, self.entity, registry, self.component);
        });
    }
}

/// Helper function to remove a reflect component from a given entity
fn remove_reflect(
    world: &mut World,
    entity: Entity,
    type_registry: &TypeRegistry,
    component_type_name: Cow<'static, str>,
) {
    let Some(mut entity) = world.get_entity_mut(entity) else {
        return;
    };
    let Some(type_registration) = type_registry.get_with_name(&component_type_name) else {
        return;
    };
    let Some(reflect_component) = type_registration.data::<ReflectComponent>() else {
        return;
    };
    reflect_component.remove(&mut entity);
}

/// A [`Command`] that removes the component of the same type as the given component type name from
/// the provided entity.
///
/// See [`ReflectCommandExt::remove_reflect`] for details.
pub struct RemoveReflect {
    /// The entity from which the component will be removed.
    pub entity: Entity,
    /// The [`Component`](crate::component::Component) type name that will be used to remove a component
    /// of the same type from the entity.
    pub component_type_name: Cow<'static, str>,
}

impl Command for RemoveReflect {
    fn apply(self, world: &mut World) {
        let registry = world.get_resource::<AppTypeRegistry>().unwrap().clone();
        remove_reflect(
            world,
            self.entity,
            &registry.read(),
            self.component_type_name,
        );
    }
}

/// A [`Command`] that removes the component of the same type as the given component type name from
/// the provided entity using the provided [`Resource`] that implements [`AsRef<TypeRegistry>`].
///
/// See [`ReflectCommandExt::remove_reflect_with_registry`] for details.
pub struct RemoveReflectWithRegistry<T: Resource + AsRef<TypeRegistry>> {
    /// The entity from which the component will be removed.
    pub entity: Entity,
    pub _t: PhantomData<T>,
    /// The [`Component`](crate::component::Component) type name that will be used to remove a component
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
    use crate::prelude::{AppTypeRegistry, ReflectComponent};
    use crate::reflect::ReflectCommandExt;
    use crate::system::{Commands, SystemState};
    use crate::{self as bevy_ecs, component::Component, world::World};
    use bevy_ecs_macros::Resource;
    use bevy_reflect::{Reflect, TypeRegistry};

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

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;

        commands
            .entity(entity)
            .insert_reflect(boxed_reflect_component_a);
        system_state.apply(&mut world);

        assert_eq!(
            world.entity(entity).get::<ComponentA>(),
            Some(&ComponentA(916))
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

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;

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
            .remove_reflect(boxed_reflect_component_a.type_name().to_owned());
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
                boxed_reflect_component_a.type_name().to_owned(),
            );
        system_state.apply(&mut world);

        assert_eq!(world.entity(entity).get::<ComponentA>(), None);
    }
}
