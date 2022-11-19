pub use helper::*;

use crate::system::BoxedSystem;

pub type BoxedCondition = BoxedSystem<(), bool>;

/// A system that determines if one or more scheduled systems should run.
///
/// Implemented for functions and closures that convert into [`System<In=(), Out=bool>`](crate::system::System)
/// with [read-only](crate::system::ReadOnlySystemParamFetch) parameters.
pub trait Condition<Params>: sealed::Condition<Params> {}

impl<Params, F> Condition<Params> for F where F: sealed::Condition<Params> {}

mod sealed {
    use crate::system::{
        IntoSystem, IsFunctionSystem, ReadOnlySystemParamFetch, SystemParam, SystemParamFunction,
    };

    pub trait Condition<Params>: IntoSystem<(), bool, Params> {}

    impl<Params, Marker, F> Condition<(IsFunctionSystem, Params, Marker)> for F
    where
        F: SystemParamFunction<(), bool, Params, Marker> + Send + Sync + 'static,
        Params: SystemParam + 'static,
        Params::Fetch: ReadOnlySystemParamFetch,
        Marker: 'static,
    {
    }
}

pub mod helper {
    use crate::schedule_v3::{State, Statelike};
    use crate::system::{Res, Resource};

    /// Generates a [`Condition`]-satisfying closure that returns `true`
    /// if the resource exists.
    pub fn resource_exists<T>() -> impl FnMut(Option<Res<T>>) -> bool
    where
        T: Resource,
    {
        move |res: Option<Res<T>>| res.is_some()
    }

    /// Generates a [`Condition`]-satisfying closure that returns `true`
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

    /// Generates a [`Condition`]-satisfying closure that returns `true`
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

    /// Generates a [`Condition`]-satisfying closure that returns `true`
    /// if the state machine exists.
    pub fn state_exists<S: Statelike>() -> impl FnMut(Option<Res<State<S>>>) -> bool {
        move |current_state: Option<Res<State<S>>>| current_state.is_some()
    }

    /// Generates a [`Condition`]-satisfying closure that returns `true`
    /// if the state machine is currently in `state`.
    ///
    /// # Panics
    ///
    /// The condition will panic if the resource does not exist.
    pub fn state_equals<S: Statelike>(state: S) -> impl FnMut(Res<State<S>>) -> bool {
        move |current_state: Res<State<S>>| current_state.0 == state
    }

    /// Generates a [`Condition`]-satisfying closure that returns `true`
    /// if the state machine exists and is currently in `state`.
    ///
    /// The condition will return `false` if the state does not exist.
    pub fn state_exists_and_equals<S: Statelike>(
        state: S,
    ) -> impl FnMut(Option<Res<State<S>>>) -> bool {
        move |current_state: Option<Res<State<S>>>| match current_state {
            Some(current_state) => current_state.0 == state,
            None => false,
        }
    }
}
