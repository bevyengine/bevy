use std::borrow::Cow;

use crate::system::{CombinatorSystem, Combine, IntoSystem, ReadOnlySystem, System};

pub type BoxedCondition = Box<dyn ReadOnlySystem<In = (), Out = bool>>;

/// A system that determines if one or more scheduled systems should run.
///
/// Implemented for functions and closures that convert into [`System<In=(), Out=bool>`](crate::system::System)
/// with [read-only](crate::system::ReadOnlySystemParam) parameters.
pub trait Condition<Marker>: sealed::Condition<Marker> {
    /// Returns a new run condition that only returns `true`
    /// if both this one and the passed `and_then` return `true`.
    ///
    /// The returned run condition is short-circuiting, meaning
    /// `and_then` will only be invoked if `self` returns `true`.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Resource, PartialEq)]
    /// struct R(u32);
    ///
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # fn my_system() {}
    /// app.add_system(
    ///     // The `resource_equals` run condition will panic since we don't initialize `R`,
    ///     // just like if we used `Res<R>` in a system.
    ///     my_system.run_if(resource_equals(R(0))),
    /// );
    /// # app.run(&mut world);
    /// ```
    ///
    /// Use `.and_then()` to avoid checking the condition.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, PartialEq)]
    /// # struct R(u32);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # fn my_system() {}
    /// app.add_system(
    ///     // `resource_equals` will only get run if the resource `R` exists.
    ///     my_system.run_if(resource_exists::<R>().and_then(resource_equals(R(0)))),
    /// );
    /// # app.run(&mut world);
    /// ```
    ///
    /// Note that in this case, it's better to just use the run condition [`resource_exists_and_equals`].
    ///
    /// [`resource_exists_and_equals`]: common_conditions::resource_exists_and_equals
    fn and_then<M, C: Condition<M>>(self, and_then: C) -> AndThen<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(and_then);
        let name = format!("{} && {}", a.name(), b.name());
        CombinatorSystem::new(a, b, Cow::Owned(name))
    }

    /// Returns a new run condition that returns `true`
    /// if either this one or the passed `or_else` return `true`.
    ///
    /// The returned run condition is short-circuiting, meaning
    /// `or_else` will only be invoked if `self` returns `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Resource, PartialEq)]
    /// struct A(u32);
    ///
    /// #[derive(Resource, PartialEq)]
    /// struct B(u32);
    ///
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # #[derive(Resource)] struct C(bool);
    /// # fn my_system(mut c: ResMut<C>) { c.0 = true; }
    /// app.add_system(
    ///     // Only run the system if either `A` or `B` exist.
    ///     my_system.run_if(resource_exists::<A>().or_else(resource_exists::<B>())),
    /// );
    /// #
    /// # world.insert_resource(C(false));
    /// # app.run(&mut world);
    /// # assert!(!world.resource::<C>().0);
    /// #
    /// # world.insert_resource(A(0));
    /// # app.run(&mut world);
    /// # assert!(world.resource::<C>().0);
    /// #
    /// # world.remove_resource::<A>();
    /// # world.insert_resource(B(0));
    /// # world.insert_resource(C(false));
    /// # app.run(&mut world);
    /// # assert!(world.resource::<C>().0);
    /// ```
    fn or_else<M, C: Condition<M>>(self, or_else: C) -> OrElse<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(or_else);
        let name = format!("{} || {}", a.name(), b.name());
        CombinatorSystem::new(a, b, Cow::Owned(name))
    }
}

impl<Marker, F> Condition<Marker> for F where F: sealed::Condition<Marker> {}

mod sealed {
    use crate::system::{IntoSystem, ReadOnlySystem};

    pub trait Condition<Marker>:
        IntoSystem<(), bool, Marker, System = Self::ReadOnlySystem>
    {
        // This associated type is necessary to let the compiler
        // know that `Self::System` is `ReadOnlySystem`.
        type ReadOnlySystem: ReadOnlySystem<In = (), Out = bool>;
    }

    impl<Marker, F> Condition<Marker> for F
    where
        F: IntoSystem<(), bool, Marker>,
        F::System: ReadOnlySystem,
    {
        type ReadOnlySystem = F::System;
    }
}

pub mod common_conditions {
    use super::Condition;
    use crate::{
        change_detection::DetectChanges,
        event::{Event, EventReader},
        prelude::{Component, Query, With},
        schedule::{State, States},
        system::{In, IntoPipeSystem, Res, Resource},
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
    /// if the resource of the given type has been added since the condition was last checked.
    pub fn resource_added<T>() -> impl FnMut(Option<Res<T>>) -> bool
    where
        T: Resource,
    {
        move |res: Option<Res<T>>| match res {
            Some(res) => res.is_added(),
            None => false,
        }
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the resource of the given type has had its value changed since the condition
    /// was last checked.
    ///
    /// The value is considered changed when it is added. The first time this condition
    /// is checked after the resource was added, it will return `true`.
    /// Change detection behaves like this everywhere in Bevy.
    ///
    /// # Panics
    ///
    /// The condition will panic if the resource does not exist.
    pub fn resource_changed<T>() -> impl FnMut(Res<T>) -> bool
    where
        T: Resource,
    {
        move |res: Res<T>| res.is_changed()
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the resource of the given type has had its value changed since the condition
    /// was last checked.
    ///
    /// The value is considered changed when it is added. The first time this condition
    /// is checked after the resource was added, it will return `true`.
    /// Change detection behaves like this everywhere in Bevy.
    ///
    /// This run condition does not detect when the resource is removed.
    ///
    /// The condition will return `false` if the resource does not exist.
    pub fn resource_exists_and_changed<T>() -> impl FnMut(Option<Res<T>>) -> bool
    where
        T: Resource,
    {
        move |res: Option<Res<T>>| match res {
            Some(res) => res.is_changed(),
            None => false,
        }
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the resource of the given type has had its value changed since the condition
    /// was last checked.
    ///
    /// The value is considered changed when it is added. The first time this condition
    /// is checked after the resource was added, it will return `true`.
    /// Change detection behaves like this everywhere in Bevy.
    ///
    /// This run condition also detects removal. It will return `true` if the resource
    /// has been removed since the run condition was last checked.
    ///
    /// The condition will return `false` if the resource does not exist.
    pub fn resource_changed_or_removed<T>() -> impl FnMut(Option<Res<T>>) -> bool
    where
        T: Resource,
    {
        let mut existed = false;
        move |res: Option<Res<T>>| {
            if let Some(value) = res {
                existed = true;
                value.is_changed()
            } else if existed {
                existed = false;
                true
            } else {
                false
            }
        }
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the resource of the given type has been removed since the condition was last checked.
    pub fn resource_removed<T>() -> impl FnMut(Option<Res<T>>) -> bool
    where
        T: Resource,
    {
        let mut existed = false;
        move |res: Option<Res<T>>| {
            if res.is_some() {
                existed = true;
                false
            } else if existed {
                existed = false;
                true
            } else {
                false
            }
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

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the state machine changed state.
    ///
    /// To do things on transitions to/from specific states, use their respective OnEnter/OnExit
    /// schedules. Use this run condition if you want to detect any change, regardless of the value.
    ///
    /// # Panics
    ///
    /// The condition will panic if the resource does not exist.
    pub fn state_changed<S: States>() -> impl FnMut(Res<State<S>>) -> bool {
        move |current_state: Res<State<S>>| current_state.is_changed()
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if there are any new events of the given type since it was last called.
    pub fn on_event<T: Event>() -> impl FnMut(EventReader<T>) -> bool {
        // The events need to be consumed, so that there are no false positives on subsequent
        // calls of the run condition. Simply checking `is_empty` would not be enough.
        // PERF: note that `count` is efficient (not actually looping/iterating),
        // due to Bevy having a specialized implementation for events.
        move |mut reader: EventReader<T>| reader.iter().count() > 0
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if there are any entities with the given component type.
    pub fn any_with_component<T: Component>() -> impl FnMut(Query<(), With<T>>) -> bool {
        move |query: Query<(), With<T>>| !query.is_empty()
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
    pub fn not<Marker>(condition: impl Condition<Marker>) -> impl Condition<()> {
        condition.pipe(|In(val): In<bool>| !val)
    }
}

/// Combines the outputs of two systems using the `&&` operator.
pub type AndThen<A, B> = CombinatorSystem<AndThenMarker, A, B>;

/// Combines the outputs of two systems using the `||` operator.
pub type OrElse<A, B> = CombinatorSystem<OrElseMarker, A, B>;

#[doc(hidden)]
pub struct AndThenMarker;

impl<In, A, B> Combine<A, B> for AndThenMarker
where
    In: Copy,
    A: System<In = In, Out = bool>,
    B: System<In = In, Out = bool>,
{
    type In = In;
    type Out = bool;

    fn combine(
        input: Self::In,
        a: impl FnOnce(<A as System>::In) -> <A as System>::Out,
        b: impl FnOnce(<B as System>::In) -> <B as System>::Out,
    ) -> Self::Out {
        a(input) && b(input)
    }
}

#[doc(hidden)]
pub struct OrElseMarker;

impl<In, A, B> Combine<A, B> for OrElseMarker
where
    In: Copy,
    A: System<In = In, Out = bool>,
    B: System<In = In, Out = bool>,
{
    type In = In;
    type Out = bool;

    fn combine(
        input: Self::In,
        a: impl FnOnce(<A as System>::In) -> <A as System>::Out,
        b: impl FnOnce(<B as System>::In) -> <B as System>::Out,
    ) -> Self::Out {
        a(input) || b(input)
    }
}
