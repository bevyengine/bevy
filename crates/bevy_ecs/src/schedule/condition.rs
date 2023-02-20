use crate::system::BoxedSystem;

pub type BoxedCondition = BoxedSystem<(), bool>;

/// A system that determines if one or more scheduled systems should run.
///
/// Implemented for functions and closures that convert into [`System<In=(), Out=bool>`](crate::system::System)
/// with [read-only](crate::system::ReadOnlySystemParam) parameters.
pub trait Condition<Params>: sealed::Condition<Params> {}

impl<Params, F> Condition<Params> for F where F: sealed::Condition<Params> {}

mod sealed {
    use crate::system::{IntoSystem, ReadOnlySystem};

    pub trait Condition<Params>:
        IntoSystem<(), bool, Params, System = Self::ReadOnlySystem>
    {
        // This associated type is necessary to let the compiler
        // know that `Self::System` is `ReadOnlySystem`.
        type ReadOnlySystem: ReadOnlySystem<In = (), Out = bool>;
    }

    impl<Params, F> Condition<Params> for F
    where
        F: IntoSystem<(), bool, Params>,
        F::System: ReadOnlySystem,
    {
        type ReadOnlySystem = F::System;
    }
}

pub mod common_conditions {
    use super::Condition;
    use crate::{
        prelude::{Added, Changed, Component, Query},
        removal_detection::RemovedComponents,
        schedule::{State, States},
        system::{In, IntoPipeSystem, ReadOnlySystem, Res, Resource},
    };

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the first time the condition is run and false every time after
    pub fn run_once() -> impl FnMut() -> bool {
        let mut has_run = false;
        move || {
            if !has_run {
                has_run = true;
                true
            } else {
                false
            }
        }
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the resource exists.
    pub fn resource_exists<T>() -> impl FnMut(Option<Res<T>>) -> bool
    where
        T: Resource,
    {
        move |res: Option<Res<T>>| res.is_some()
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the resource is equal to `value`.
    ///
    /// # Panics
    ///
    /// The condition will panic if the resource does not exist.
    pub fn resource_equals<T>(value: T) -> impl FnMut(Res<T>) -> bool
    where
        T: Resource + PartialEq,
    {
        move |res: Res<T>| *res == value
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the resource exists and is equal to `value`.
    ///
    /// The condition will return `false` if the resource does not exist.
    pub fn resource_exists_and_equals<T>(value: T) -> impl FnMut(Option<Res<T>>) -> bool
    where
        T: Resource + PartialEq,
    {
        move |res: Option<Res<T>>| match res {
            Some(res) => *res == value,
            None => false,
        }
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the state machine exists.
    pub fn state_exists<S: States>() -> impl FnMut(Option<Res<State<S>>>) -> bool {
        move |current_state: Option<Res<State<S>>>| current_state.is_some()
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the state machine is currently in `state`.
    ///
    /// # Panics
    ///
    /// The condition will panic if the resource does not exist.
    pub fn in_state<S: States>(state: S) -> impl FnMut(Res<State<S>>) -> bool {
        move |current_state: Res<State<S>>| current_state.0 == state
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the state machine exists and is currently in `state`.
    ///
    /// The condition will return `false` if the state does not exist.
    pub fn state_exists_and_equals<S: States>(
        state: S,
    ) -> impl FnMut(Option<Res<State<S>>>) -> bool {
        move |current_state: Option<Res<State<S>>>| match current_state {
            Some(current_state) => current_state.0 == state,
            None => false,
        }
    }

    /// Generates a  [`Condition`](super::Condition) that inverses the result of passed one.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    /// // Building a new schedule/app...
    /// let mut sched = Schedule::default();
    /// sched.add_system(
    ///         // This system will never run.
    ///         my_system.run_if(not(always_true))
    ///     )
    ///     // ...
    /// #   ;
    /// # let mut world = World::new();
    /// # sched.run(&mut world);
    ///
    /// // A condition that always returns true.
    /// fn always_true() -> bool {
    ///    true
    /// }
    /// #
    /// # fn my_system() { unreachable!() }
    /// ```
    pub fn not<Params>(
        condition: impl Condition<Params>,
    ) -> impl ReadOnlySystem<In = (), Out = bool> {
        condition.pipe(|In(val): In<bool>| !val)
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the given component type was added to any entities.
    ///
    /// Run conditions are evaluated on the main thread, blocking any other systems from running.
    /// This run condition is relatively expensive, as it iterates over every entity with this component.
    /// As a result, you likely only want to use this run condition when the number of entities with the component `T` is small.
    pub fn any_component_added<T: Component>() -> impl FnMut(Query<(), Added<T>>) -> bool {
        move |query: Query<(), Added<T>>| !query.is_empty()
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the value of the given component type has been changed on any entities.
    ///
    /// Run conditions are evaluated on the main thread, blocking any other systems from running.
    /// This run condition is relatively expensive, as it iterates over every entity with this component.
    /// As a result, you likely only want to use this run condition when the number of entities with the component `T` is small.
    pub fn any_component_changed<T: Component>() -> impl FnMut(Query<(), Changed<T>>) -> bool {
        move |query: Query<(), Changed<T>>| !query.is_empty()
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if there are any entities with the given component type removed.
    pub fn any_component_removed<T: Component>() -> impl FnMut(RemovedComponents<T>) -> bool {
        move |removals: RemovedComponents<T>| !removals.is_empty()
    }
}
