//! Define commands and traits for inserting and removing reflected components.
use std::{any, borrow::Cow, marker::PhantomData};

use bevy_reflect::Reflect;

use crate::prelude::Mut;
use crate::reflect::{AppTypeRegistry, ReadTypeRegistry};
use crate::system::{Command, EntityCommands, Resource};
use crate::{entity::Entity, reflect::ReflectComponent, world::World};

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
    fn insert_reflect(&mut self, component: Box<dyn Reflect>) -> &mut Self {
        self.insert_reflect_with_registry::<AppTypeRegistry>(component)
    }

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
    fn insert_reflect_with_registry<T: Resource + ReadTypeRegistry>(
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
    fn remove_reflect(&mut self, component_type_name: impl Into<Cow<'static, str>>) -> &mut Self {
        self.remove_reflect_with_registry::<AppTypeRegistry>(component_type_name)
    }
    /// Same as [`remove_reflect`](ReflectCommandExt::remove_reflect), but using the `T` resource as type registry instead of
    /// `AppTypeRegistry`.
    fn remove_reflect_with_registry<T: Resource + ReadTypeRegistry>(
        &mut self,
        component_type_name: impl Into<Cow<'static, str>>,
    ) -> &mut Self;
}

impl<'w, 's, 'a> ReflectCommandExt for EntityCommands<'w, 's, 'a> {
    fn insert_reflect_with_registry<T: Resource + ReadTypeRegistry>(
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

    fn remove_reflect_with_registry<T: Resource + ReadTypeRegistry>(
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

/// A [`Command`] that adds the boxed reflect component to an entity using the data in the provided
/// [`Resource`] that implements [`ReadTypeRegistry`].
///
/// See [`ReflectCommandExt::insert_reflect_with_registry`] for details.
pub struct InsertReflectWithRegistry<T: Resource + ReadTypeRegistry> {
    /// The entity on which the component will be inserted.
    pub entity: Entity,
    pub _t: PhantomData<T>,
    /// The reflect [`Component`](crate::component::Component) that will be added to the entity.
    pub component: Box<dyn Reflect>,
}

impl<T: Resource + ReadTypeRegistry> Command for InsertReflectWithRegistry<T> {
    fn apply(self, world: &mut World) {
        let type_name = self.component.type_name();
        let entity = self.entity;
        let registry_name = any::type_name::<T>();
        let Some(resource) = world.get_resource::<T>() else {
            panic!("Can't insert reflect component {type_name} on entity {entity:?} because the resource {registry_name} doesn't exist.");
        };
        let registry = resource.type_registry();
        let Some(entry) = registry.get_with_name(type_name) else {
            panic!("Can't insert reflect component {type_name} on entity {entity:?} because it isn't registered in {registry_name}. Try adding it to the type registry with `app.register_type::<{type_name}>()`.");
        };
        let Some(reflect_component) = entry.data::<ReflectComponent>().cloned() else {
            panic!("Can't insert reflect component {type_name} on entity {entity:?} because the component's ReflectComponent {type_name} isn't registered in {registry_name}. Make sure to add the `#[reflect(Component)]` attribute to {type_name}'s type declaration.");
        };
        drop(registry);

        let Some(mut entity) = world.get_entity_mut(entity) else {
            panic!("error[B0003]: Can't insert reflect component {type_name} on non-existent entity {entity:?}.");
        };
        reflect_component.insert(&mut entity, &*self.component);
    }
}

/// A [`Command`] that removes the component of the same type as the given component type name from
/// the provided entity using the provided [`Resource`] that implements [`ReadTypeRegistry`].
///
/// See [`ReflectCommandExt::remove_reflect_with_registry`] for details.
pub struct RemoveReflectWithRegistry<T: Resource + ReadTypeRegistry> {
    /// The entity from which the component will be removed.
    pub entity: Entity,
    pub _t: PhantomData<T>,
    /// The [`Component`](crate::component::Component) type name that will be used to remove a component
    /// of the same type from the entity.
    pub component_type_name: Cow<'static, str>,
}

impl<T: Resource + ReadTypeRegistry> Command for RemoveReflectWithRegistry<T> {
    fn apply(self, world: &mut World) {
        world.resource_scope(|world, registry: Mut<T>| {
            let registry = registry.type_registry();
            let Some(mut entity) = world.get_entity_mut(self.entity) else {
                return;
            };
            let Some(type_registration) = registry.get_with_name(&self.component_type_name) else {
                return;
            };
            let Some(reflect_component) = type_registration.data::<ReflectComponent>() else {
                return;
            };
            reflect_component.remove(&mut entity);
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
