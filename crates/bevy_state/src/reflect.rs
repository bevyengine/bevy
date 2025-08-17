use crate::state::{FreelyMutableState, NextState, State, States};

use bevy_ecs::{reflect::from_reflect_with_fallback, world::World};
use bevy_reflect::{FromType, Reflect, TypePath, TypeRegistry};

/// A struct used to operate on the reflected [`States`] trait of a type.
///
/// A [`ReflectState`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectState(ReflectStateFns);

/// The raw function pointers needed to make up a [`ReflectState`].
#[derive(Clone)]
pub struct ReflectStateFns {
    /// Function pointer implementing [`ReflectState::reflect()`].
    pub reflect: fn(&World) -> Option<&dyn Reflect>,
}

impl ReflectStateFns {
    /// Get the default set of [`ReflectStateFns`] for a specific component type using its
    /// [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: States + Reflect>() -> Self {
        <ReflectState as FromType<T>>::from_type().0
    }
}

impl ReflectState {
    /// Gets the value of this [`States`] type from the world as a reflected reference.
    pub fn reflect<'a>(&self, world: &'a World) -> Option<&'a dyn Reflect> {
        (self.0.reflect)(world)
    }
}

impl<S: States + Reflect> FromType<S> for ReflectState {
    fn from_type() -> Self {
        ReflectState(ReflectStateFns {
            reflect: |world| {
                world
                    .get_resource::<State<S>>()
                    .map(|res| res.get() as &dyn Reflect)
            },
        })
    }
}

/// A struct used to operate on the reflected [`FreelyMutableState`] trait of a type.
///
/// A [`ReflectFreelyMutableState`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub struct ReflectFreelyMutableState(ReflectFreelyMutableStateFns);

/// The raw function pointers needed to make up a [`ReflectFreelyMutableState`].
#[derive(Clone)]
pub struct ReflectFreelyMutableStateFns {
    /// Function pointer implementing [`ReflectFreelyMutableState::set_next_state()`].
    pub set_next_state: fn(&mut World, &dyn Reflect, &TypeRegistry),
}

impl ReflectFreelyMutableStateFns {
    /// Get the default set of [`ReflectFreelyMutableStateFns`] for a specific component type using its
    /// [`FromType`] implementation.
    ///
    /// This is useful if you want to start with the default implementation before overriding some
    /// of the functions to create a custom implementation.
    pub fn new<T: FreelyMutableState + Reflect + TypePath>() -> Self {
        <ReflectFreelyMutableState as FromType<T>>::from_type().0
    }
}

impl ReflectFreelyMutableState {
    /// Tentatively set a pending state transition to a reflected [`ReflectFreelyMutableState`].
    pub fn set_next_state(&self, world: &mut World, state: &dyn Reflect, registry: &TypeRegistry) {
        (self.0.set_next_state)(world, state, registry);
    }
}

impl<S: FreelyMutableState + Reflect + TypePath> FromType<S> for ReflectFreelyMutableState {
    fn from_type() -> Self {
        ReflectFreelyMutableState(ReflectFreelyMutableStateFns {
            set_next_state: |world, reflected_state, registry| {
                let new_state: S = from_reflect_with_fallback(
                    reflected_state.as_partial_reflect(),
                    world,
                    registry,
                );
                if let Some(mut next_state) = world.get_resource_mut::<NextState<S>>() {
                    next_state.set(new_state);
                }
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        app::{AppExtStates, StatesPlugin},
        reflect::{ReflectFreelyMutableState, ReflectState},
        state::State,
    };
    use bevy_app::App;
    use bevy_ecs::prelude::AppTypeRegistry;
    use bevy_reflect::Reflect;
    use bevy_state_macros::States;
    use core::any::TypeId;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, States, Reflect)]
    enum StateTest {
        A,
        B,
    }

    #[test]
    fn test_reflect_state_operations() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin)
            .insert_state(StateTest::A)
            .register_type_mutable_state::<StateTest>();

        let type_registry = app.world_mut().resource::<AppTypeRegistry>().0.clone();
        let type_registry = type_registry.read();

        let (reflect_state, reflect_mutable_state) = (
            type_registry
                .get_type_data::<ReflectState>(TypeId::of::<StateTest>())
                .unwrap()
                .clone(),
            type_registry
                .get_type_data::<ReflectFreelyMutableState>(TypeId::of::<StateTest>())
                .unwrap()
                .clone(),
        );

        let current_value = reflect_state.reflect(app.world()).unwrap();
        assert_eq!(
            current_value.downcast_ref::<StateTest>().unwrap(),
            &StateTest::A
        );

        reflect_mutable_state.set_next_state(app.world_mut(), &StateTest::B, &type_registry);

        assert_ne!(
            app.world().resource::<State<StateTest>>().get(),
            &StateTest::B
        );

        app.update();

        assert_eq!(
            app.world().resource::<State<StateTest>>().get(),
            &StateTest::B
        );

        let current_value = reflect_state.reflect(app.world()).unwrap();
        assert_eq!(
            current_value.downcast_ref::<StateTest>().unwrap(),
            &StateTest::B
        );
    }
}
