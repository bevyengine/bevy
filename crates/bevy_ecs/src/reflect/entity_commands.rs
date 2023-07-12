use crate::reflect::AppTypeRegistry;
use crate::system::{Command, EntityCommands, Resource};
use crate::{entity::Entity, reflect::ReflectComponent, world::World};
use bevy_reflect::{Reflect, TypeRegistry};
use std::marker::PhantomData;

/// An extension trait for [`EntityCommands`] for reflection related functions
pub trait EntityCommandsReflectExtension {
    /// Inserts the given boxed reflect component to the entity using the reflection data in
    /// [`AppTypeRegistry`].
    ///
    /// This will overwrite any previous component of the same type.
    ///
    /// # Panics
    ///
    /// - If the entity doesn't exist.
    /// - If [`AppTypeRegistry`] does not have the reflection data for the given [Component](crate::component::Component).
    /// - If the component data is invalid. See [`Reflect::apply`] for further details.
    /// - If [`AppTypeRegistry`] is not present in the [`World`].
    ///
    /// # Note
    ///
    /// Prefer to use the typed [`EntityCommands::insert`] if possible as it is optimized for insertions
    /// compared to reflection which requires more overhead.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::reflect::EntityCommandsReflectExtension;
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
    /// fn insert_reflected_component(
    ///     mut commands: Commands,
    ///     mut prefab: ResMut<Prefab>
    ///     ) {
    ///     // Create a set of new boxed reflect components to use
    ///     let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;
    ///     let boxed_reflect_component_b = Box::new(ComponentB("NineSixteen".to_string())) as Box<dyn Reflect>;
    ///
    ///     // You can overwrite the component in the resource with either ComponentA or ComponentB
    ///     prefab.component = boxed_reflect_component_a;
    ///     prefab.component = boxed_reflect_component_b;
    ///     
    ///     // No matter which component is in the resource and without knowing the exact type, you can
    ///     // use the insert_reflected entity command to insert that component into an entity.
    ///     commands
    ///         .spawn_empty()
    ///         .insert_reflected(prefab.component.clone_value());
    /// }
    ///
    /// ```
    fn insert_reflected(&mut self, component: Box<dyn Reflect>) -> &mut Self;

    /// Inserts the given boxed reflect component to the entity using the reflection data in the
    /// provided [`TypeRegistry`] [Resource](Resource)..
    ///
    /// This will overwrite any previous component of the same type.
    ///
    /// # Panics
    ///
    /// - If the entity doesn't exist.
    /// - If the given [`TypeRegistry`] does not have the reflection data for the given [Component](crate::component::Component).
    /// - If the component data is invalid. See [`Reflect::apply`] for further details.
    /// - If the given [`Resource`] is not present in the [`World`].
    ///
    /// # Note
    ///
    /// - Prefer to use the typed [`EntityCommands::insert`] if possible as it is optimized for insertions
    /// compared to reflection which requires more overhead.
    /// - The given [`Resource`] is removed from the [`World`] before the command is applied.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::reflect::EntityCommandsReflectExtension;
    /// # use bevy_reflect::{FromReflect, FromType, Reflect, TypeRegistry};
    ///
    /// // A custom TypeRegistry Resource
    /// #[derive(Resource)]
    /// struct TypeRegistryResource {
    ///     type_registry: TypeRegistry,
    /// }
    ///
    /// impl AsRef<TypeRegistry> for TypeRegistryResource {
    ///     fn as_ref(&self) -> &TypeRegistry {
    ///         &self.type_registry
    ///     }
    /// }
    /// // A resource that can hold any component that implements reflect as a boxed reflect component
    /// #[derive(Resource)]
    /// struct Prefab{
    ///     component: Box<dyn Reflect>,
    /// }
    /// #[derive(Component, Reflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Reflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentB(String);
    ///
    /// fn insert_reflected_component(
    ///     mut commands: Commands,
    ///     mut prefab: ResMut<Prefab>
    ///     ) {
    ///     // Create a set of new boxed reflect components to use
    ///     let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;
    ///     let boxed_reflect_component_b = Box::new(ComponentB("NineSixteen".to_string())) as Box<dyn Reflect>;
    ///
    ///     // You can overwrite the component in the resource with either ComponentA or ComponentB
    ///     prefab.component = boxed_reflect_component_a;
    ///     prefab.component = boxed_reflect_component_b;
    ///     
    ///     // No matter which component is in the resource and without knowing the exact type, you can
    ///     // use the insert_reflected entity command to insert that component into an entity using
    ///     // the data in the provided resource.
    ///     commands
    ///         .spawn_empty()
    ///         .insert_reflected_with_registry::<TypeRegistryResource>(prefab.component.clone_value());
    /// }
    ///
    /// ```
    fn insert_reflected_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component: Box<dyn Reflect>,
    ) -> &mut Self;

    /// Removes the component of the same type as the component type name from the entity using the
    /// reflection data from [`AppTypeRegistry`].
    ///
    /// Does nothing if the entity does not have a component of the same type, if [`AppTypeRegistry`]
    /// does not contain the reflection data for the given component, or if the entity does not exist.
    ///
    /// # Note
    ///
    /// Prefer to use the typed [`EntityCommands::remove`] if possible as it is optimized for removals
    /// compared to reflection which requires more overhead.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::reflect::EntityCommandsReflectExtension;
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
    /// fn remove_reflected_component(
    ///     mut commands: Commands,
    ///     prefab: Res<Prefab>
    ///     ) {
    ///     // Prefab can hold any boxed reflect component. In this case either
    ///     // ComponentA or ComponentB. No matter which component is in the resource though,
    ///     // we can attempt to remove any component of that same type from an entity.
    ///     commands.entity(prefab.entity)
    ///         .remove_reflected(prefab.component.type_name().into());
    /// }
    ///
    /// ```
    fn remove_reflected(&mut self, component_type_name: String) -> &mut Self;

    /// Removes the component of the same type as the given component type name from the entity
    /// using the reflection data in the provided [`TypeRegistry`] [Resource](Resource).
    ///
    /// Does nothing if the entity does not have a component of the same type, if [`AppTypeRegistry`]
    /// does not contain the reflection data for the given component, or if the entity does not exist.
    ///
    /// # Note
    ///
    /// Prefer to use the typed [`EntityCommands::remove`] if possible as it is optimized for removals
    /// compared to reflection which requires more overhead.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::reflect::EntityCommandsReflectExtension;
    /// # use bevy_reflect::{FromReflect, FromType, Reflect, TypeRegistry};
    /// // A custom TypeRegistry Resource
    /// #[derive(Resource)]
    /// struct TypeRegistryResource {
    ///     type_registry: TypeRegistry,
    /// }
    ///
    /// impl AsRef<TypeRegistry> for TypeRegistryResource {
    ///     fn as_ref(&self) -> &TypeRegistry {
    ///         &self.type_registry
    ///     }
    /// }    
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
    /// fn remove_reflected_component(
    ///     mut commands: Commands,
    ///     prefab: Res<Prefab>
    ///     ) {
    ///     // Prefab can hold any boxed reflect component. In this case either
    ///     // ComponentA or ComponentB. No matter which component is in the resource though,
    ///     // we can attempt to remove any component of that same type from an entity.
    ///     commands.entity(prefab.entity)
    ///         .remove_reflected_with_registry::<TypeRegistryResource>(prefab.component.type_name().into());
    /// }
    ///
    /// ```
    fn remove_reflected_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component_type_name: String,
    ) -> &mut Self;
}

impl<'w, 's, 'a> EntityCommandsReflectExtension for EntityCommands<'w, 's, 'a> {
    fn insert_reflected(&mut self, component: Box<dyn Reflect>) -> &mut Self {
        self.commands.add(InsertReflected {
            entity: self.entity,
            component,
        });
        self
    }

    fn insert_reflected_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component: Box<dyn Reflect>,
    ) -> &mut Self {
        self.commands.add(InsertReflectedWithRegistry::<T> {
            entity: self.entity,
            _t: PhantomData,
            component,
        });
        self
    }

    fn remove_reflected(&mut self, component_type_name: String) -> &mut Self {
        self.commands.add(RemoveReflected {
            entity: self.entity,
            component_type_name,
        });
        self
    }

    fn remove_reflected_with_registry<T: Resource + AsRef<TypeRegistry>>(
        &mut self,
        component_type_name: String,
    ) -> &mut Self {
        self.commands.add(RemoveReflectedWithRegistry::<T> {
            entity: self.entity,
            _t: PhantomData,
            component_type_name,
        });
        self
    }
}

/// A [`Command`] that adds the boxed reflect component to an entity using the data in
/// [`AppTypeRegistry`].
///
/// See [`EntityCommandsReflectExtension::insert_reflected`] for details.
pub struct InsertReflected {
    /// The entity on which the component will be inserted.
    pub entity: Entity,
    /// The reflect [Component](crate::component::Component) that will be added to the entity.
    pub component: Box<dyn Reflect>,
}

impl Command for InsertReflected {
    fn apply(self, world: &mut World) {
        let registry = world.get_resource::<AppTypeRegistry>().unwrap().clone();
        if let Some(mut entity) = world.get_entity_mut(self.entity) {
            let type_info = self.component.type_name();
            if let Some(type_registration) = registry.read().get_with_name(type_info) {
                if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
                    reflect_component.insert(&mut entity, &*self.component);
                } else {
                    panic!("Could not get ReflectComponent data (for component type {}) because it doesn't exist in this TypeRegistration.", self.component.type_name());
                }
            } else {
                panic!("Could not get type registration (for component type {}) because it doesn't exist in the TypeRegistry.", self.component.type_name());
            }
        } else {
            panic!("error[B0003]: Could not insert a reflected component (of type {}) for entity {:?} because it doesn't exist in this World.", self.component.type_name(), self.entity);
        }
    }
}

/// A [`Command`] that adds the boxed reflect component to an entity using the data in the provided
/// [`Resource`] that implements [`AsRef<TypeRegistry>`].
///
/// See [`EntityCommandsReflectExtension::insert_reflected_with_registry`] for details.
pub struct InsertReflectedWithRegistry<T: Resource + AsRef<TypeRegistry>> {
    /// The entity on which the component will be inserted.
    pub entity: Entity,
    pub _t: PhantomData<T>,
    /// The reflect [Component](crate::component::Component) that will be added to the entity.
    pub component: Box<dyn Reflect>,
}

impl<T: Resource + AsRef<TypeRegistry>> Command for InsertReflectedWithRegistry<T> {
    fn apply(self, world: &mut World) {
        let registry = world.remove_resource::<T>().unwrap();
        let registry: &TypeRegistry = registry.as_ref();

        if let Some(mut entity) = world.get_entity_mut(self.entity) {
            let type_info = self.component.type_name();
            if let Some(type_registration) = registry.get_with_name(type_info) {
                if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
                    reflect_component.insert(&mut entity, &*self.component);
                } else {
                    panic!("Could not get ReflectComponent data (for component type {}) because it doesn't exist in this TypeRegistration.", self.component.type_name());
                }
            } else {
                panic!("Could not get type registration (for component type {}) because it doesn't exist in the TypeRegistry.", self.component.type_name());
            }
        } else {
            panic!("error[B0003]: Could not insert a reflected component (of type {}) for entity {:?} because it doesn't exist in this World.", self.component.type_name(), self.entity);
        }
    }
}

/// A [`Command`] that removes the component of the same type as the given component type name from
/// the provided entity.
///
/// See [`EntityCommandsReflectExtension::remove_reflected`] for details.
pub struct RemoveReflected {
    /// The entity from which the component will be removed.
    pub entity: Entity,
    /// The [Component](crate::component::Component) type name that will be used to remove a component
    /// of the same type from the entity.
    pub component_type_name: String,
}

impl Command for RemoveReflected {
    fn apply(self, world: &mut World) {
        let registry = world.get_resource::<AppTypeRegistry>().unwrap().clone();
        if let Some(mut entity) = world.get_entity_mut(self.entity) {
            if let Some(type_registration) = registry
                .read()
                .get_with_name(self.component_type_name.as_ref())
            {
                if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
                    reflect_component.remove(&mut entity);
                }
            }
        }
    }
}

/// A [`Command`] that removes the component of the same type as the given component type name from
/// the provided entity using the provided [`Resource`] that implements [`AsRef<TypeRegistry>`].
///
/// See [`EntityCommandsReflectExtension::remove_reflected_with_registry`] for details.
pub struct RemoveReflectedWithRegistry<T: Resource + AsRef<TypeRegistry>> {
    /// The entity from which the component will be removed.
    pub entity: Entity,
    pub _t: PhantomData<T>,
    /// The [Component](crate::component::Component) type name that will be used to remove a component
    /// of the same type from the entity.
    pub component_type_name: String,
}

impl<T: Resource + AsRef<TypeRegistry>> Command for RemoveReflectedWithRegistry<T> {
    fn apply(self, world: &mut World) {
        let registry = world.remove_resource::<T>().unwrap();
        let registry: &TypeRegistry = registry.as_ref();

        if let Some(mut entity) = world.get_entity_mut(self.entity) {
            if let Some(type_registration) =
                registry.get_with_name(self.component_type_name.as_ref())
            {
                if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
                    reflect_component.remove(&mut entity);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::{AppTypeRegistry, ReflectComponent};
    use crate::reflect::EntityCommandsReflectExtension;
    use crate::system::{Commands, SystemState};
    use crate::{self as bevy_ecs, component::Component, world::World};
    use bevy_ecs_macros::Resource;
    use bevy_reflect::{FromType, Reflect, TypeRegistry};

    #[derive(Resource)]
    struct TypeRegistryResource {
        type_registry: TypeRegistry,
    }

    impl AsRef<TypeRegistry> for TypeRegistryResource {
        fn as_ref(&self) -> &TypeRegistry {
            &self.type_registry
        }
    }

    #[derive(Component, Reflect, Default)]
    #[reflect(Component)]
    struct ComponentA(u32);

    #[test]
    fn insert_reflected() {
        let mut world = World::new();

        let type_registry = AppTypeRegistry::default();
        let mut registry = type_registry.write();
        registry.register::<ComponentA>();
        let registration = registry
            .get_mut(std::any::TypeId::of::<ComponentA>())
            .unwrap();
        registration.insert(<ReflectComponent as FromType<ComponentA>>::from_type());
        drop(registry);
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands.spawn_empty().id();

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;

        commands
            .entity(entity)
            .insert_reflected(boxed_reflect_component_a);
        system_state.apply(&mut world);

        assert!(world.entity(entity).get::<ComponentA>().is_some());
        assert_eq!(world.entity(entity).get::<ComponentA>().unwrap().0, 916);
    }

    #[test]
    fn insert_reflected_with_registry() {
        let mut world = World::new();

        let mut type_registry = TypeRegistryResource {
            type_registry: TypeRegistry::new(),
        };

        type_registry.type_registry.register::<ComponentA>();
        let registration = type_registry
            .type_registry
            .get_mut(std::any::TypeId::of::<ComponentA>())
            .unwrap();
        registration.insert(<ReflectComponent as FromType<ComponentA>>::from_type());
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands.spawn_empty().id();

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;

        commands
            .entity(entity)
            .insert_reflected_with_registry::<TypeRegistryResource>(boxed_reflect_component_a);
        system_state.apply(&mut world);

        assert!(world.entity(entity).get::<ComponentA>().is_some());
        assert_eq!(world.entity(entity).get::<ComponentA>().unwrap().0, 916);
    }

    #[test]
    fn remove_reflected() {
        let mut world = World::new();

        let type_registry = AppTypeRegistry::default();
        let mut registry = type_registry.write();
        registry.register::<ComponentA>();
        let registration = registry
            .get_mut(std::any::TypeId::of::<ComponentA>())
            .unwrap();
        registration.insert(<ReflectComponent as FromType<ComponentA>>::from_type());
        drop(registry);
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands.spawn(ComponentA(0)).id();

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;

        commands
            .entity(entity)
            .remove_reflected(boxed_reflect_component_a.type_name().into());
        system_state.apply(&mut world);

        assert!(world.entity(entity).get::<ComponentA>().is_none());
    }

    #[test]
    fn remove_reflected_with_registry() {
        let mut world = World::new();

        let mut type_registry = TypeRegistryResource {
            type_registry: TypeRegistry::new(),
        };

        type_registry.type_registry.register::<ComponentA>();
        let registration = type_registry
            .type_registry
            .get_mut(std::any::TypeId::of::<ComponentA>())
            .unwrap();
        registration.insert(<ReflectComponent as FromType<ComponentA>>::from_type());
        world.insert_resource(type_registry);

        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let mut commands = system_state.get_mut(&mut world);

        let entity = commands.spawn(ComponentA(0)).id();

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;

        commands
            .entity(entity)
            .remove_reflected_with_registry::<TypeRegistryResource>(
                boxed_reflect_component_a.type_name().into(),
            );
        system_state.apply(&mut world);

        assert!(world.entity(entity).get::<ComponentA>().is_none());
    }
}
