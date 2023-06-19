use crate::{
    prelude::Entity,
    reflect::ReflectComponent,
    system::{Command, EntityCommands},
    world::World,
};
use bevy_reflect::{Reflect, TypeRegistry};

/// An extension trait for [`EntityCommands`] for reflection related functions
pub trait EntityCommandsReflectExtension {
    /// Inserts the given boxed reflect component to the entity using the reflection data in the supplied
    /// [`TypeRegistry`]. This will overwrite any previous component of the same type.
    /// Panics if the entity doesn't exist or if the [`TypeRegistry`] doesn't have the reflection data
    /// for the given [`Component`].
    ///
    /// # Note
    /// Prefer to use the typed [`EntityCommands::insert`] unless you have good reason to use reflection.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::reflect::EntityCommandsReflectExtension;
    /// # use bevy_reflect::{FromReflect, FromType, Reflect, TypeRegistry};
    ///
    /// # #[derive(Resource)]
    /// # struct TypeRegistryResource{
    /// #     type_registry: TypeRegistry,
    /// # }
    /// // A resource that can hold any component that implements reflect as a boxed reflect component
    /// #[derive(Resource)]
    /// struct SpecialComponentHolder{
    ///     component: Box<dyn Reflect>,
    /// }
    /// #[derive(Component, Reflect, FromReflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Reflect, FromReflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentB(String);
    ///
    /// fn insert_reflected_component(
    ///     mut commands: Commands,
    ///     mut type_registry: ResMut<TypeRegistryResource>,
    ///     mut special_component_holder: ResMut<SpecialComponentHolder>
    ///     ) {
    ///     #
    ///     # type_registry.type_registry.register::<ComponentA>();
    ///     # let mut registration = type_registry
    ///     #     .type_registry
    ///     #     .get_mut(std::any::TypeId::of::<ComponentA>())
    ///     #     .unwrap();
    ///     # registration.insert(<ReflectComponent as FromType<ComponentA>>::from_type());
    ///     #
    ///     # type_registry.type_registry.register::<ComponentB>();
    ///     # let mut registration = type_registry
    ///     #     .type_registry
    ///     #     .get_mut(std::any::TypeId::of::<ComponentB>())
    ///     #     .unwrap();
    ///     # registration.insert(<ReflectComponent as FromType<ComponentB>>::from_type());
    ///     // Create a set of new boxed reflect components to use
    ///     let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;
    ///     let boxed_reflect_component_b = Box::new(ComponentB("NineSixteen".to_string())) as Box<dyn Reflect>;
    ///
    ///     // You can overwrite the component in the resource with either ComponentA or ComponentB
    ///     special_component_holder.component = boxed_reflect_component_a;
    ///     special_component_holder.component = boxed_reflect_component_b;
    ///     
    ///     // No matter which component is in the resource and without knowing the exact type, you can
    ///     // use the insert_reflected entity command to insert that component into an entity.
    ///     commands
    ///         .spawn_empty()
    ///         .insert_reflected(special_component_holder.component.clone_value(), type_registry.type_registry.clone());
    /// }
    ///
    /// ```
    fn insert_reflected(
        &mut self,
        component: Box<dyn Reflect>,
        type_registry: TypeRegistry,
    ) -> &mut Self;

    /// Removes the component of the same type as the supplied boxed reflect component from the entity
    /// using the reflection data in the supplied [`TypeRegistry`]. Does nothing if the entity does not
    /// have a component of the same type, if the [`TypeRegistry`] does not contain the reflection data
    /// for the given component, or if the entity does not exist.
    ///
    /// # Note
    /// Prefer to use the typed [`EntityCommands::remove`] unless you have good reason to use reflection.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::reflect::EntityCommandsReflectExtension;
    /// # use bevy_reflect::{FromReflect, FromType, Reflect, TypeRegistry};
    ///
    /// # #[derive(Resource)]
    /// # struct TypeRegistryResource{
    /// #     type_registry: TypeRegistry,
    /// # }
    /// // A resource that can hold any component that implements reflect as a boxed reflect component
    /// #[derive(Resource)]
    /// struct SpecialComponentHolder{
    ///     entity: Entity,
    ///     component: Box<dyn Reflect>,
    /// }
    /// #[derive(Component, Reflect, FromReflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentA(u32);
    /// #[derive(Component, Reflect, FromReflect, Default)]
    /// #[reflect(Component)]
    /// struct ComponentB(String);
    ///
    /// fn remove_reflected_component(
    ///     mut commands: Commands,
    ///     mut type_registry: ResMut<TypeRegistryResource>,
    ///     special_component_holder: Res<SpecialComponentHolder>
    ///     ) {
    ///     #
    ///     # type_registry.type_registry.register::<ComponentA>();
    ///     # let mut registration = type_registry
    ///     #     .type_registry
    ///     #     .get_mut(std::any::TypeId::of::<ComponentA>())
    ///     #     .unwrap();
    ///     # registration.insert(<ReflectComponent as FromType<ComponentA>>::from_type());
    ///     #
    ///     # type_registry.type_registry.register::<ComponentB>();
    ///     # let mut registration = type_registry
    ///     #     .type_registry
    ///     #     .get_mut(std::any::TypeId::of::<ComponentB>())
    ///     #     .unwrap();
    ///     # registration.insert(<ReflectComponent as FromType<ComponentB>>::from_type());
    ///     // SpecialComponentHolder can hold any boxed reflect component. In this case either
    ///     // ComponentA or ComponentB. No matter which component is in the resource though,
    ///     // we can attempt to remove any component of that same type from an entity.
    ///     commands.entity(special_component_holder.entity)
    ///         .remove_reflected(special_component_holder.component.clone_value(), type_registry.type_registry.clone());
    /// }
    ///
    /// ```
    fn remove_reflected(
        &mut self,
        component: Box<dyn Reflect>,
        type_registry: TypeRegistry,
    ) -> &mut Self;
}

impl<'w, 's, 'a> EntityCommandsReflectExtension for EntityCommands<'w, 's, 'a> {
    fn insert_reflected(
        &mut self,
        component: Box<dyn Reflect>,
        type_registry: TypeRegistry,
    ) -> &mut Self {
        self.commands.add(InsertReflected {
            entity: self.entity,
            type_registry,
            component,
        });
        self
    }

    fn remove_reflected(
        &mut self,
        component: Box<dyn Reflect>,
        type_registry: TypeRegistry,
    ) -> &mut Self {
        self.commands.add(RemoveReflected {
            entity: self.entity,
            type_registry,
            component,
        });
        self
    }
}

/// A [`Command`] that adds the boxed reflect component to an entity using the data in the provided
/// [`TypeRegistry`].
pub struct InsertReflected {
    /// The entity on which the component will be inserted.
    pub entity: Entity,
    /// The [`TypeRegistry`] that the component is registered in and that will be used to get reflection
    /// data in order to insert the component.
    pub type_registry: TypeRegistry,
    /// The reflect [`Component`] that will be added to the entity.
    pub component: Box<dyn Reflect>,
}

impl Command for InsertReflected {
    fn apply(self, world: &mut World) {
        if let Some(mut entity) = world.get_entity_mut(self.entity) {
            let type_info = self.component.type_name();
            if let Some(type_registration) = self.type_registry.get_with_name(type_info) {
                if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
                    reflect_component.insert(&mut entity, &*self.component);
                } else {
                    panic!("Could not get ReflectComponent data (for component type {}) because it doesn't exist in this TypeRegistration.", self.component.type_name());
                }
            } else {
                panic!("Could not get type registration for component (for component {}) because it doesn't exist in the TypeRegistry.", self.component.type_name());
            }
        } else {
            panic!("error[B0003]: Could not insert a reflected component (of type {}) for entity {:?} because it doesn't exist in this World.", self.component.type_name(), self.entity);
        }
    }
}

/// A [`Command`] that removes the component of the same type as the given boxed reflect component from
/// the provided entity. Does nothing if the entity does not have a component of the same
/// type, if the [`TypeRegistry`] does not contain the data for the given component, or if the entity
/// does not exist.
pub struct RemoveReflected {
    /// The entity from which the component will be removed.
    pub entity: Entity,
    /// The [`TypeRegistry`] that the component is registered in and that will be used to get reflection
    /// data in order to remove the component.
    pub type_registry: TypeRegistry,
    /// The boxed reflect [`Component`] that will be used to remove a component of the same type
    /// from the entity.
    pub component: Box<dyn Reflect>,
}

impl Command for RemoveReflected {
    fn apply(self, world: &mut World) {
        if let Some(mut entity) = world.get_entity_mut(self.entity) {
            let type_info = self.component.type_name();
            if let Some(type_registration) = self.type_registry.get_with_name(type_info) {
                if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
                    reflect_component.remove(&mut entity);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        component::Component,
        prelude::ReflectComponent,
        reflect::entity_commands::EntityCommandsReflectExtension,
        system::{Commands, Res, SystemState},
        world::World,
    };
    use bevy_ecs_macros::Resource;
    use bevy_reflect::{FromReflect, FromType, Reflect, TypeRegistry};

    #[derive(Resource)]
    struct TypeRegistryResource {
        type_registry: TypeRegistry,
    }

    #[derive(Component, Reflect, FromReflect, Default)]
    #[reflect(Component)]
    struct ComponentA(u32);

    #[test]
    fn insert_reflected() {
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

        let mut system_state: SystemState<(Commands, Res<TypeRegistryResource>)> =
            SystemState::new(&mut world);
        let (mut commands, type_registry) = system_state.get_mut(&mut world);

        let entity = commands.spawn_empty().id();

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;

        commands.entity(entity).insert_reflected(
            boxed_reflect_component_a,
            type_registry.type_registry.clone(),
        );
        system_state.apply(&mut world);

        assert!(world.entity(entity).get::<ComponentA>().is_some());
        assert_eq!(world.entity(entity).get::<ComponentA>().unwrap().0, 916);
    }

    #[test]
    fn remove_reflected() {
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

        let mut system_state: SystemState<(Commands, Res<TypeRegistryResource>)> =
            SystemState::new(&mut world);
        let (mut commands, type_registry) = system_state.get_mut(&mut world);

        let entity = commands.spawn(ComponentA(0)).id();

        let boxed_reflect_component_a = Box::new(ComponentA(916)) as Box<dyn Reflect>;

        commands.entity(entity).remove_reflected(
            boxed_reflect_component_a,
            type_registry.type_registry.clone(),
        );
        system_state.apply(&mut world);

        assert!(world.entity(entity).get::<ComponentA>().is_none());
    }
}
