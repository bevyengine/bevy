use crate::schedule_v3::{CurrentState, State};
use crate::system::{BoxedSystem, NonSend, Res, Resource};

pub type BoxedRunCondition = BoxedSystem<(), bool>;

/// Implemented for functions and closures that convert into [`System<In=(), Out=bool>`](crate::system::System)
/// types that have [read-only](crate::system::ReadOnlySystemParamFetch) data access.
pub trait IntoRunCondition<Params>: sealed::IntoRunCondition<Params> {}

impl<Params, F> IntoRunCondition<Params> for F where F: sealed::IntoRunCondition<Params> {}

mod sealed {
    use crate::system::{
        IntoSystem, IsFunctionSystem, ReadOnlySystemParamFetch, SystemParam, SystemParamFunction,
    };

    // This trait is private to prevent implementations for systems that aren't read-only.
    pub trait IntoRunCondition<Params>: IntoSystem<(), bool, Params> {}

    impl<Params, Marker, F> IntoRunCondition<(IsFunctionSystem, Params, Marker)> for F
    where
        F: SystemParamFunction<(), bool, Params, Marker> + Send + Sync + 'static,
        Params: SystemParam + 'static,
        Params::Fetch: ReadOnlySystemParamFetch,
        Marker: 'static,
    {
    }
}

/// Convenience function that generates an [`IntoRunCondition`]-compatible closure that returns `true`
/// if the resource exists.
pub fn resource_exists<T>() -> impl FnMut(Option<Res<T>>) -> bool
where
    T: Resource,
{
    move |res: Option<Res<T>>| res.is_some()
}

/// Convenience function that generates an [`IntoRunCondition`]-compatible closure that returns `true`
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

/// Convenience function that generates an [`IntoRunCondition`]-compatible closure that returns `true`
/// if the resource exists and is equal to `value`.
pub fn resource_exists_and_equals<T>(value: T) -> impl FnMut(Option<Res<T>>) -> bool
where
    T: Resource + PartialEq,
{
    move |res: Option<Res<T>>| match res {
        Some(res) => *res == value,
        None => false,
    }
}

/// Convenience function that generates an [`IntoRunCondition`]-compatible closure that returns `true`
/// if the non-[`Send`] resource exists.
pub fn non_send_resource_exists<T>() -> impl FnMut(Option<NonSend<T>>) -> bool
where
    T: Resource,
{
    move |res: Option<NonSend<T>>| res.is_some()
}

/// Convenience function that generates an [`IntoRunCondition`]-compatible closure that returns `true`
/// if the non-`Send` resource is equal to `value`.
///
/// # Panics
///
/// The condition will panic if the resource does not exist.
pub fn non_send_resource_equals<T>(value: T) -> impl FnMut(NonSend<T>) -> bool
where
    T: Resource + PartialEq,
{
    move |res: NonSend<T>| *res == value
}

/// Convenience function that generates an [`IntoRunCondition`]-compatible closure that returns `true`
/// if the non-[`Send`] resource exists and is equal to `value`.
pub fn non_send_resource_exists_and_equals<T>(value: T) -> impl FnMut(Option<NonSend<T>>) -> bool
where
    T: Resource + PartialEq,
{
    move |res: Option<NonSend<T>>| match res {
        Some(res) => *res == value,
        None => false,
    }
}

/// Convenience function that generates an [`IntoRunCondition`]-compatible closure that returns `true`
/// if the state machine exists.
pub fn state_machine_exists<S: State>() -> impl FnMut(Option<Res<CurrentState<S>>>) -> bool {
    move |current_state: Option<Res<CurrentState<S>>>| current_state.is_some()
}

/// Convenience function that generates an [`IntoRunCondition`]-compatible closure that returns `true`
/// if the state machine is currently in `state`.
///
/// # Panics
///
/// The condition will panic if the resource does not exist.
pub fn state_equals<S: State>(state: S) -> impl FnMut(Res<CurrentState<S>>) -> bool {
    move |current_state: Res<CurrentState<S>>| current_state.0 == state
}

/// Convenience function that generates an [`IntoRunCondition`]-compatible closure that returns `true`
/// if the state machine exists and is currently in `state`.
pub fn state_machine_exists_and_equals<S: State>(
    state: S,
) -> impl FnMut(Option<Res<CurrentState<S>>>) -> bool {
    move |current_state: Option<Res<CurrentState<S>>>| match current_state {
        Some(current_state) => current_state.0 == state,
        None => false,
    }
}
