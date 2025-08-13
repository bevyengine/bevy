//! Provides additional functionality for [`World`] when the `bevy_reflect` feature is enabled.

use core::any::TypeId;

use thiserror::Error;

use bevy_reflect::{Reflect, ReflectFromPtr};
use bevy_utils::prelude::DebugName;

use crate::{prelude::*, world::ComponentId};

impl World {
    /// Retrieves a reference to the given `entity`'s [`Component`] of the given `type_id` using
    /// reflection.
    ///
    /// Requires implementing [`Reflect`] for the [`Component`] (e.g., using [`#[derive(Reflect)`](derive@bevy_reflect::Reflect))
    /// and `app.register_type::<TheComponent>()` to have been called[^note-reflect-impl].
    ///
    /// If you want to call this with a [`ComponentId`], see [`World::components`] and [`Components::get_id`] to get
    /// the corresponding [`TypeId`].
    ///
    /// Also see the crate documentation for [`bevy_reflect`] for more information on
    /// [`Reflect`] and bevy's reflection capabilities.
    ///
    /// # Errors
    ///
    /// See [`GetComponentReflectError`] for the possible errors and their descriptions.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    /// use bevy_reflect::Reflect;
    /// use std::any::TypeId;
    ///
    /// // define a `Component` and derive `Reflect` for it
    /// #[derive(Component, Reflect)]
    /// struct MyComponent;
    ///
    /// // create a `World` for this example
    /// let mut world = World::new();
    ///
    /// // Note: This is usually handled by `App::register_type()`, but this example cannot use `App`.
    /// world.init_resource::<AppTypeRegistry>();
    /// world.get_resource_mut::<AppTypeRegistry>().unwrap().write().register::<MyComponent>();
    ///
    /// // spawn an entity with a `MyComponent`
    /// let entity = world.spawn(MyComponent).id();
    ///
    /// // retrieve a reflected reference to the entity's `MyComponent`
    /// let comp_reflected: &dyn Reflect = world.get_reflect(entity, TypeId::of::<MyComponent>()).unwrap();
    ///
    /// // make sure we got the expected type
    /// assert!(comp_reflected.is::<MyComponent>());
    /// ```
    ///
    /// # Note
    /// Requires the `bevy_reflect` feature (included in the default features).
    ///
    /// [`Components::get_id`]: crate::component::Components::get_id
    /// [`ReflectFromPtr`]: bevy_reflect::ReflectFromPtr
    /// [`TypeData`]: bevy_reflect::TypeData
    /// [`Reflect`]: bevy_reflect::Reflect
    /// [`App::register_type`]: ../../bevy_app/struct.App.html#method.register_type
    /// [^note-reflect-impl]: More specifically: Requires [`TypeData`] for [`ReflectFromPtr`] to be registered for the given `type_id`,
    ///     which is automatically handled when deriving [`Reflect`] and calling [`App::register_type`].
    #[inline]
    pub fn get_reflect(
        &self,
        entity: Entity,
        type_id: TypeId,
    ) -> Result<&dyn Reflect, GetComponentReflectError> {
        let Some(component_id) = self.components().get_valid_id(type_id) else {
            return Err(GetComponentReflectError::NoCorrespondingComponentId(
                type_id,
            ));
        };

        let Some(comp_ptr) = self.get_by_id(entity, component_id) else {
            let component_name = self.components().get_name(component_id);

            return Err(GetComponentReflectError::EntityDoesNotHaveComponent {
                entity,
                type_id,
                component_id,
                component_name,
            });
        };

        let Some(type_registry) = self.get_resource::<AppTypeRegistry>().map(|atr| atr.read())
        else {
            return Err(GetComponentReflectError::MissingAppTypeRegistry);
        };

        let Some(reflect_from_ptr) = type_registry.get_type_data::<ReflectFromPtr>(type_id) else {
            return Err(GetComponentReflectError::MissingReflectFromPtrTypeData(
                type_id,
            ));
        };

        // SAFETY:
        // - `comp_ptr` is guaranteed to point to an object of type `type_id`
        // - `reflect_from_ptr` was constructed for type `type_id`
        // - Assertion that checks this equality is present
        unsafe {
            assert_eq!(
                reflect_from_ptr.type_id(),
                type_id,
                "Mismatch between Ptr's type_id and ReflectFromPtr's type_id",
            );

            Ok(reflect_from_ptr.as_reflect(comp_ptr))
        }
    }

    /// Retrieves a mutable reference to the given `entity`'s [`Component`] of the given `type_id` using
    /// reflection.
    ///
    /// Requires implementing [`Reflect`] for the [`Component`] (e.g., using [`#[derive(Reflect)`](derive@bevy_reflect::Reflect))
    /// and `app.register_type::<TheComponent>()` to have been called.
    ///
    /// This is the mutable version of [`World::get_reflect`], see its docs for more information
    /// and an example.
    ///
    /// Just calling this method does not trigger [change detection](crate::change_detection).
    ///
    /// # Errors
    ///
    /// See [`GetComponentReflectError`] for the possible errors and their descriptions.
    ///
    /// # Example
    ///
    /// See the documentation for [`World::get_reflect`].
    ///
    /// # Note
    /// Requires the feature `bevy_reflect` (included in the default features).
    ///
    /// [`Reflect`]: bevy_reflect::Reflect
    #[inline]
    pub fn get_reflect_mut(
        &mut self,
        entity: Entity,
        type_id: TypeId,
    ) -> Result<Mut<'_, dyn Reflect>, GetComponentReflectError> {
        // little clone() + read() dance so we a) don't keep a borrow of `self` and b) don't drop a
        // temporary (from read()) too  early.
        let Some(app_type_registry) = self.get_resource::<AppTypeRegistry>().cloned() else {
            return Err(GetComponentReflectError::MissingAppTypeRegistry);
        };
        let type_registry = app_type_registry.read();

        let Some(reflect_from_ptr) = type_registry.get_type_data::<ReflectFromPtr>(type_id) else {
            return Err(GetComponentReflectError::MissingReflectFromPtrTypeData(
                type_id,
            ));
        };

        let Some(component_id) = self.components().get_valid_id(type_id) else {
            return Err(GetComponentReflectError::NoCorrespondingComponentId(
                type_id,
            ));
        };

        // HACK: Only required for the `None`-case/`else`-branch, but it borrows `self`, which will
        // already be mutably borrowed by `self.get_mut_by_id()`, and I didn't find a way around it.
        let component_name = self.components().get_name(component_id).clone();

        let Some(comp_mut_untyped) = self.get_mut_by_id(entity, component_id) else {
            return Err(GetComponentReflectError::EntityDoesNotHaveComponent {
                entity,
                type_id,
                component_id,
                component_name,
            });
        };

        // SAFETY:
        // - `comp_mut_untyped` is guaranteed to point to an object of type `type_id`
        // - `reflect_from_ptr` was constructed for type `type_id`
        // - Assertion that checks this equality is present
        let comp_mut_typed = comp_mut_untyped.map_unchanged(|ptr_mut| unsafe {
            assert_eq!(
                reflect_from_ptr.type_id(),
                type_id,
                "Mismatch between PtrMut's type_id and ReflectFromPtr's type_id",
            );

            reflect_from_ptr.as_reflect_mut(ptr_mut)
        });

        Ok(comp_mut_typed)
    }
}

/// The error type returned by [`World::get_reflect`] and [`World::get_reflect_mut`].
#[derive(Error, Debug)]
pub enum GetComponentReflectError {
    /// There is no [`ComponentId`] corresponding to the given [`TypeId`].
    ///
    /// This is usually handled by calling [`App::register_type`] for the type corresponding to
    /// the given [`TypeId`].
    ///
    /// See the documentation for [`bevy_reflect`] for more information.
    ///
    /// [`App::register_type`]: ../../../bevy_app/struct.App.html#method.register_type
    #[error("No `ComponentId` corresponding to {0:?} found (did you call App::register_type()?)")]
    NoCorrespondingComponentId(TypeId),

    /// The given [`Entity`] does not have a [`Component`] corresponding to the given [`TypeId`].
    #[error("The given `Entity` {entity} does not have a `{component_name:?}` component ({component_id:?}, which corresponds to {type_id:?})")]
    EntityDoesNotHaveComponent {
        /// The given [`Entity`].
        entity: Entity,
        /// The given [`TypeId`].
        type_id: TypeId,
        /// The [`ComponentId`] corresponding to the given [`TypeId`].
        component_id: ComponentId,
        /// The name corresponding to the [`Component`] with the given [`TypeId`], or `None`
        /// if not available.
        component_name: Option<DebugName>,
    },

    /// The [`World`] was missing the [`AppTypeRegistry`] resource.
    #[error("The `World` was missing the `AppTypeRegistry` resource")]
    MissingAppTypeRegistry,

    /// The [`World`]'s [`TypeRegistry`] did not contain [`TypeData`] for [`ReflectFromPtr`] for the given [`TypeId`].
    ///
    /// This is usually handled by calling [`App::register_type`] for the type corresponding to
    /// the given [`TypeId`].
    ///
    /// See the documentation for [`bevy_reflect`] for more information.
    ///
    /// [`TypeData`]: bevy_reflect::TypeData
    /// [`TypeRegistry`]: bevy_reflect::TypeRegistry
    /// [`ReflectFromPtr`]: bevy_reflect::ReflectFromPtr
    /// [`App::register_type`]: ../../../bevy_app/struct.App.html#method.register_type
    #[error("The `World`'s `TypeRegistry` did not contain `TypeData` for `ReflectFromPtr` for the given {0:?} (did you call `App::register_type()`?)")]
    MissingReflectFromPtrTypeData(TypeId),
}

#[cfg(test)]
mod tests {
    use core::any::TypeId;

    use bevy_reflect::Reflect;

    use crate::prelude::{AppTypeRegistry, Component, DetectChanges, World};

    #[derive(Component, Reflect)]
    struct RFoo(i32);

    #[derive(Component)]
    struct Bar;

    #[test]
    fn get_component_as_reflect() {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();

        let app_type_registry = world.get_resource_mut::<AppTypeRegistry>().unwrap();
        app_type_registry.write().register::<RFoo>();

        {
            let entity_with_rfoo = world.spawn(RFoo(42)).id();
            let comp_reflect = world
                .get_reflect(entity_with_rfoo, TypeId::of::<RFoo>())
                .expect("Reflection of RFoo-component failed");

            assert!(comp_reflect.is::<RFoo>());
        }

        {
            let entity_without_rfoo = world.spawn_empty().id();
            let reflect_opt = world.get_reflect(entity_without_rfoo, TypeId::of::<RFoo>());

            assert!(reflect_opt.is_err());
        }

        {
            let entity_with_bar = world.spawn(Bar).id();
            let reflect_opt = world.get_reflect(entity_with_bar, TypeId::of::<Bar>());

            assert!(reflect_opt.is_err());
        }
    }

    #[test]
    fn get_component_as_mut_reflect() {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();

        let app_type_registry = world.get_resource_mut::<AppTypeRegistry>().unwrap();
        app_type_registry.write().register::<RFoo>();

        {
            let entity_with_rfoo = world.spawn(RFoo(42)).id();
            let mut comp_reflect = world
                .get_reflect_mut(entity_with_rfoo, TypeId::of::<RFoo>())
                .expect("Mutable reflection of RFoo-component failed");

            let comp_rfoo_reflected = comp_reflect
                .downcast_mut::<RFoo>()
                .expect("Wrong type reflected (expected RFoo)");
            assert_eq!(comp_rfoo_reflected.0, 42);
            comp_rfoo_reflected.0 = 1337;

            let rfoo_ref = world.entity(entity_with_rfoo).get_ref::<RFoo>().unwrap();
            assert!(rfoo_ref.is_changed());
            assert_eq!(rfoo_ref.0, 1337);
        }

        {
            let entity_without_rfoo = world.spawn_empty().id();
            let reflect_opt = world.get_reflect_mut(entity_without_rfoo, TypeId::of::<RFoo>());

            assert!(reflect_opt.is_err());
        }

        {
            let entity_with_bar = world.spawn(Bar).id();
            let reflect_opt = world.get_reflect_mut(entity_with_bar, TypeId::of::<Bar>());

            assert!(reflect_opt.is_err());
        }
    }
}
