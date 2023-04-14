use std::any::TypeId;
use std::borrow::Cow;
use std::ops::Not;

use crate::component::{self, ComponentId};
use crate::query::Access;
use crate::system::{CombinatorSystem, Combine, IntoSystem, ReadOnlySystem, System};
use crate::world::World;

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
    /// app.add_systems(
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
    /// app.add_systems(
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
    /// app.add_systems(
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
    use std::borrow::Cow;

    use super::NotSystem;
    use crate::{
        change_detection::DetectChanges,
        event::{Event, EventReader},
        prelude::{Component, Query, With},
        schedule::{State, States},
        system::{IntoSystem, Res, Resource, System},
    };

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the first time the condition is run and false every time after
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// app.add_systems(
    ///     // `run_once` will only return true the first time it's evaluated
    ///     my_system.run_if(run_once()),
    /// );
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // This is the first time the condition will be evaluated so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    ///
    /// // This is the seconds time the condition will be evaluated so `my_system` won't run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// ```
    pub fn run_once() -> impl FnMut() -> bool + Clone {
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
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// app.add_systems(
    ///     // `resource_exsists` will only return true if the given resource exsists in the world
    ///     my_system.run_if(resource_exists::<Counter>()),
    /// );
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // `Counter` hasn't been added so `my_system` won't run
    /// app.run(&mut world);
    /// world.init_resource::<Counter>();
    ///
    /// // `Counter` has now been added so `my_system` can run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// ```
    pub fn resource_exists<T>() -> impl FnMut(Option<Res<T>>) -> bool + Clone
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
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default, PartialEq)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// app.add_systems(
    ///     // `resource_equals` will only return true if the given resource equals the given value
    ///     my_system.run_if(resource_equals(Counter(0))),
    /// );
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // `Counter` is `0` so `my_system` can run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    ///
    /// // `Counter` is no longer `0` so `my_system` won't run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default, PartialEq)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// app.add_systems(
    ///     // `resource_exists_and_equals` will only return true
    ///     // if the given resource exsists and equals the given value
    ///     my_system.run_if(resource_exists_and_equals(Counter(0))),
    /// );
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // `Counter` hasn't been added so `my_system` can't run
    /// app.run(&mut world);
    /// world.init_resource::<Counter>();
    ///
    /// // `Counter` is `0` so `my_system` can run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    ///
    /// // `Counter` is no longer `0` so `my_system` won't run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// app.add_systems(
    ///     // `resource_added` will only return true if the
    ///     // given resource was just added
    ///     my_system.run_if(resource_added::<Counter>()),
    /// );
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// world.init_resource::<Counter>();
    ///
    /// // `Counter` was just added so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    ///
    /// // `Counter` was not just added so `my_system` will not run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// ```
    pub fn resource_added<T>() -> impl FnMut(Option<Res<T>>) -> bool + Clone
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
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// app.add_systems(
    ///     // `resource_changed` will only return true if the
    ///     // given resource was just changed (or added)
    ///     my_system.run_if(
    ///         resource_changed::<Counter>()
    ///         // By default detecting changes will also trigger if the resource was
    ///         // just added, this won't work with my example so I will addd a second
    ///         // condition to make sure the resource wasn't just added
    ///         .and_then(not(resource_added::<Counter>()))
    ///     ),
    /// );
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // `Counter` hasn't been changed so `my_system` won't run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    ///
    /// world.resource_mut::<Counter>().0 = 50;
    ///
    /// // `Counter` was just changed so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 51);
    /// ```
    pub fn resource_changed<T>() -> impl FnMut(Res<T>) -> bool + Clone
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
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// app.add_systems(
    ///     // `resource_exists_and_changed` will only return true if the
    ///     // given resource exsists and was just changed (or added)
    ///     my_system.run_if(
    ///         resource_exists_and_changed::<Counter>()
    ///         // By default detecting changes will also trigger if the resource was
    ///         // just added, this won't work with my example so I will addd a second
    ///         // condition to make sure the resource wasn't just added
    ///         .and_then(not(resource_added::<Counter>()))
    ///     ),
    /// );
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // `Counter` doesn't exist so `my_system` won't run
    /// app.run(&mut world);
    /// world.init_resource::<Counter>();
    ///
    /// // `Counter` hasn't been changed so `my_system` won't run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    ///
    /// world.resource_mut::<Counter>().0 = 50;
    ///
    /// // `Counter` was just changed so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 51);
    /// ```
    pub fn resource_exists_and_changed<T>() -> impl FnMut(Option<Res<T>>) -> bool + Clone
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
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// app.add_systems(
    ///     // `resource_changed_or_removed` will only return true if the
    ///     // given resource was just changed or removed (or added)
    ///     my_system.run_if(
    ///         resource_changed_or_removed::<Counter>()
    ///         // By default detecting changes will also trigger if the resource was
    ///         // just added, this won't work with my example so I will addd a second
    ///         // condition to make sure the resource wasn't just added
    ///         .and_then(not(resource_added::<Counter>()))
    ///     ),
    /// );
    ///
    /// #[derive(Resource, Default)]
    /// struct MyResource;
    ///
    /// // If `Counter` exists, increment it, otherwise insert `MyResource`
    /// fn my_system(mut commands: Commands, mut counter: Option<ResMut<Counter>>) {
    ///     if let Some(mut counter) = counter {
    ///         counter.0 += 1;
    ///     } else {
    ///         commands.init_resource::<MyResource>();
    ///     }
    /// }
    ///
    /// // `Counter` hasn't been changed so `my_system` won't run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    ///
    /// world.resource_mut::<Counter>().0 = 50;
    ///
    /// // `Counter` was just changed so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 51);
    ///
    /// world.remove_resource::<Counter>();
    ///
    /// // `Counter` was just removed so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.contains_resource::<MyResource>(), true);
    /// ```
    pub fn resource_changed_or_removed<T>() -> impl FnMut(Option<Res<T>>) -> bool + Clone
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
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// app.add_systems(
    ///     // `resource_removed` will only return true if the
    ///     // given resource was just removed
    ///     my_system.run_if(resource_removed::<MyResource>()),
    /// );
    ///
    /// #[derive(Resource, Default)]
    /// struct MyResource;
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// world.init_resource::<MyResource>();
    ///
    /// // `MyResource` hasn't just been removed so `my_system` won't run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    ///
    /// world.remove_resource::<MyResource>();
    ///
    /// // `MyResource` was just removed so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// ```
    pub fn resource_removed<T>() -> impl FnMut(Option<Res<T>>) -> bool + Clone
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
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// #[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
    /// enum GameState {
    ///     #[default]
    ///     Playing,
    ///     Paused,
    /// }
    ///
    /// app.add_systems(
    ///     // `state_exists` will only return true if the
    ///     // given state exsists
    ///     my_system.run_if(state_exists::<GameState>()),
    /// );
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // `GameState` does not yet exist `my_system` won't run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    ///
    /// world.init_resource::<State<GameState>>();
    ///
    /// // `GameState` now exists so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// ```
    pub fn state_exists<S: States>() -> impl FnMut(Option<Res<State<S>>>) -> bool + Clone {
        move |current_state: Option<Res<State<S>>>| current_state.is_some()
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the state machine is currently in `state`.
    ///
    /// # Panics
    ///
    /// The condition will panic if the resource does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// #[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
    /// enum GameState {
    ///     #[default]
    ///     Playing,
    ///     Paused,
    /// }
    ///
    /// world.init_resource::<State<GameState>>();
    ///
    /// app.add_systems((
    ///     // `in_state` will only return true if the
    ///     // given state equals the given value
    ///     play_system.run_if(in_state(GameState::Playing)),
    ///     pause_system.run_if(in_state(GameState::Paused)),
    /// ));
    ///
    /// fn play_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// fn pause_system(mut counter: ResMut<Counter>) {
    ///     counter.0 -= 1;
    /// }
    ///
    /// // We default to `GameState::Playing` so `play_system` runs
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    ///
    /// *world.resource_mut::<State<GameState>>() = State::new(GameState::Paused);
    ///
    /// // Now that we are in `GameState::Pause`, `pause_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    /// ```
    pub fn in_state<S: States>(state: S) -> impl FnMut(Res<State<S>>) -> bool + Clone {
        move |current_state: Res<State<S>>| *current_state == state
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if the state machine exists and is currently in `state`.
    ///
    /// The condition will return `false` if the state does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// #[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
    /// enum GameState {
    ///     #[default]
    ///     Playing,
    ///     Paused,
    /// }
    ///
    /// app.add_systems((
    ///     // `state_exists_and_equals` will only return true if the
    ///     // given state exsists and equals the given value
    ///     play_system.run_if(state_exists_and_equals(GameState::Playing)),
    ///     pause_system.run_if(state_exists_and_equals(GameState::Paused)),
    /// ));
    ///
    /// fn play_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// fn pause_system(mut counter: ResMut<Counter>) {
    ///     counter.0 -= 1;
    /// }
    ///
    /// // `GameState` does not yet exists so neither system will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    ///
    /// world.init_resource::<State<GameState>>();
    ///
    /// // We default to `GameState::Playing` so `play_system` runs
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    ///
    /// *world.resource_mut::<State<GameState>>() = State::new(GameState::Paused);
    ///
    /// // Now that we are in `GameState::Pause`, `pause_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    /// ```
    pub fn state_exists_and_equals<S: States>(
        state: S,
    ) -> impl FnMut(Option<Res<State<S>>>) -> bool + Clone {
        move |current_state: Option<Res<State<S>>>| match current_state {
            Some(current_state) => *current_state == state,
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
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// #[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
    /// enum GameState {
    ///     #[default]
    ///     Playing,
    ///     Paused,
    /// }
    ///
    /// world.init_resource::<State<GameState>>();
    ///
    /// app.add_systems(
    ///     // `state_changed` will only return true if the
    ///     // given states value has just been updated or
    ///     // the state has just been added
    ///     my_system.run_if(state_changed::<GameState>()),
    /// );
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // `GameState` has just been added so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    ///
    /// // `GameState` has not been updated so `my_system` will not run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    ///
    /// *world.resource_mut::<State<GameState>>() = State::new(GameState::Paused);
    ///
    /// // Now that `GameState` has been updated `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 2);
    /// ```
    pub fn state_changed<S: States>() -> impl FnMut(Res<State<S>>) -> bool + Clone {
        move |current_state: Res<State<S>>| current_state.is_changed()
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if there are any new events of the given type since it was last called.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// # world.init_resource::<Events<MyEvent>>();
    /// # app.add_systems(Events::<MyEvent>::update_system.before(my_system));
    ///
    /// app.add_systems(
    ///     my_system.run_if(on_event::<MyEvent>()),
    /// );
    ///
    /// struct MyEvent;
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // No new `MyEvent` events have been push so `my_system` won't run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    ///
    /// world.resource_mut::<Events<MyEvent>>().send(MyEvent);
    ///
    /// // A `MyEvent` event has been pushed so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// ```
    pub fn on_event<T: Event>() -> impl FnMut(EventReader<T>) -> bool + Clone {
        // The events need to be consumed, so that there are no false positives on subsequent
        // calls of the run condition. Simply checking `is_empty` would not be enough.
        // PERF: note that `count` is efficient (not actually looping/iterating),
        // due to Bevy having a specialized implementation for events.
        move |mut reader: EventReader<T>| reader.iter().count() > 0
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if there are any entities with the given component type.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// app.add_systems(
    ///     my_system.run_if(any_with_component::<MyComponent>()),
    /// );
    ///
    /// #[derive(Component)]
    /// struct MyComponent;
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // No entities exist yet with a `MyComponent` component so `my_system` won't run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    ///
    /// world.spawn(MyComponent);
    ///
    /// // An entities with `MyComponent` now exists so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// ```
    pub fn any_with_component<T: Component>() -> impl FnMut(Query<(), With<T>>) -> bool + Clone {
        move |query: Query<(), With<T>>| !query.is_empty()
    }

    /// Generates a [`Condition`](super::Condition) that inverses the result of passed one.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::new();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// app.add_systems(
    ///     // `not` will inverse any condition you pass in.
    ///     // Since the condition we choose always returns true
    ///     // this system will never run
    ///     my_system.run_if(not(always)),
    /// );
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// fn always() -> bool {
    ///     true
    /// }
    ///
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    /// ```
    pub fn not<Marker, TOut, T>(condition: T) -> NotSystem<T::System>
    where
        TOut: std::ops::Not,
        T: IntoSystem<(), TOut, Marker>,
    {
        let condition = IntoSystem::into_system(condition);
        let name = format!("!{}", condition.name());
        NotSystem::<T::System> {
            condition,
            name: Cow::Owned(name),
        }
    }
}

/// Invokes [`Not`] with the output of another system.
///
/// See [`common_conditions::not`] for examples.
#[derive(Clone)]
pub struct NotSystem<T: System>
where
    T::Out: Not,
{
    condition: T,
    name: Cow<'static, str>,
}
impl<T: System> System for NotSystem<T>
where
    T::Out: Not,
{
    type In = T::In;
    type Out = <T::Out as Not>::Output;

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        self.condition.component_access()
    }

    fn archetype_component_access(&self) -> &Access<crate::archetype::ArchetypeComponentId> {
        self.condition.archetype_component_access()
    }

    fn is_send(&self) -> bool {
        self.condition.is_send()
    }

    fn is_exclusive(&self) -> bool {
        self.condition.is_exclusive()
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out {
        // SAFETY: The inner condition system asserts its own safety.
        !self.condition.run_unsafe(input, world)
    }

    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        !self.condition.run(input, world)
    }

    fn apply_buffers(&mut self, world: &mut World) {
        self.condition.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut World) {
        self.condition.initialize(world);
    }

    fn update_archetype_component_access(&mut self, world: &World) {
        self.condition.update_archetype_component_access(world);
    }

    fn check_change_tick(&mut self, change_tick: component::Tick) {
        self.condition.check_change_tick(change_tick);
    }

    fn get_last_run(&self) -> component::Tick {
        self.condition.get_last_run()
    }

    fn set_last_run(&mut self, last_run: component::Tick) {
        self.condition.set_last_run(last_run);
    }
}

// SAFETY: This trait is only implemented when the inner system is read-only.
// The `Not` condition does not modify access, and thus cannot transform a read-only system into one that is not.
unsafe impl<T> ReadOnlySystem for NotSystem<T>
where
    T: ReadOnlySystem,
    T::Out: Not,
{
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

#[cfg(test)]
mod tests {
    use super::{common_conditions::*, Condition};
    use crate as bevy_ecs;
    use crate::component::Component;
    use crate::schedule::IntoSystemConfigs;
    use crate::schedule::{common_conditions::not, State, States};
    use crate::system::Local;
    use crate::{change_detection::ResMut, schedule::Schedule, world::World};
    use bevy_ecs_macros::Resource;

    #[derive(Resource, Default)]
    struct Counter(usize);

    fn increment_counter(mut counter: ResMut<Counter>) {
        counter.0 += 1;
    }

    fn every_other_time(mut has_ran: Local<bool>) -> bool {
        *has_ran = !*has_ran;
        *has_ran
    }

    #[test]
    fn run_condition() {
        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::new();

        // Run every other cycle
        schedule.add_systems(increment_counter.run_if(every_other_time));

        schedule.run(&mut world);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 1);
        schedule.run(&mut world);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 2);

        // Run every other cycle oppsite to the last one
        schedule.add_systems(increment_counter.run_if(not(every_other_time)));

        schedule.run(&mut world);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 4);
        schedule.run(&mut world);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 6);
    }

    #[test]
    fn run_condition_combinators() {
        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::new();

        // Always run
        schedule.add_systems(increment_counter.run_if(every_other_time.or_else(|| true)));
        // Run every other cycle
        schedule.add_systems(increment_counter.run_if(every_other_time.and_then(|| true)));

        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 2);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 3);
    }

    #[test]
    fn multiple_run_conditions() {
        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::new();

        // Run every other cycle
        schedule.add_systems(increment_counter.run_if(every_other_time).run_if(|| true));
        // Never run
        schedule.add_systems(increment_counter.run_if(every_other_time).run_if(|| false));

        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 1);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 1);
    }

    #[test]
    fn multiple_run_conditions_is_and_operation() {
        let mut world = World::new();
        world.init_resource::<Counter>();

        let mut schedule = Schedule::new();

        // This should never run, if multiple run conditions worked
        // like an OR condition then it would always run
        schedule.add_systems(
            increment_counter
                .run_if(every_other_time)
                .run_if(not(every_other_time)),
        );

        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 0);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 0);
    }

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum TestState {
        #[default]
        A,
        B,
    }

    #[derive(Component)]
    struct TestComponent;

    fn test_system() {}

    // Ensure distributive_run_if compiles with the common conditions.
    #[test]
    fn distributive_run_if_compiles() {
        Schedule::default().add_systems(
            (test_system, test_system)
                .distributive_run_if(run_once())
                .distributive_run_if(resource_exists::<State<TestState>>())
                .distributive_run_if(resource_added::<State<TestState>>())
                .distributive_run_if(resource_changed::<State<TestState>>())
                .distributive_run_if(resource_exists_and_changed::<State<TestState>>())
                .distributive_run_if(resource_changed_or_removed::<State<TestState>>())
                .distributive_run_if(resource_removed::<State<TestState>>())
                .distributive_run_if(state_exists::<TestState>())
                .distributive_run_if(in_state(TestState::A))
                .distributive_run_if(state_changed::<TestState>())
                .distributive_run_if(on_event::<u8>())
                .distributive_run_if(any_with_component::<TestComponent>())
                .distributive_run_if(not(run_once())),
        );
    }
}
