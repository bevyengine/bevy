use std::borrow::Cow;
use std::ops::Not;

use crate::system::{
    Adapt, AdapterSystem, CombinatorSystem, Combine, IntoSystem, ReadOnlySystem, System,
};

/// A type-erased run condition stored in a [`Box`].
pub type BoxedCondition<In = ()> = Box<dyn ReadOnlySystem<In = In, Out = bool>>;

/// A system that determines if one or more scheduled systems should run.
///
/// Implemented for functions and closures that convert into [`System<Out=bool>`](System)
/// with [read-only](crate::system::ReadOnlySystemParam) parameters.
///
/// # Marker type parameter
///
/// `Condition` trait has `Marker` type parameter, which has no special meaning,
/// but exists to work around the limitation of Rust's trait system.
///
/// Type parameter in return type can be set to `<()>` by calling [`IntoSystem::into_system`],
/// but usually have to be specified when passing a condition to a function.
///
/// ```
/// # use bevy_ecs::schedule::Condition;
/// # use bevy_ecs::system::IntoSystem;
/// fn not_condition<Marker>(a: impl Condition<Marker>) -> impl Condition<()> {
///    IntoSystem::into_system(a.map(|x| !x))
/// }
/// ```
///
/// # Examples
/// A condition that returns true every other time it's called.
/// ```
/// # use bevy_ecs::prelude::*;
/// fn every_other_time() -> impl Condition<()> {
///     IntoSystem::into_system(|mut flag: Local<bool>| {
///         *flag = !*flag;
///         *flag
///     })
/// }
///
/// # #[derive(Resource)] struct DidRun(bool);
/// # fn my_system(mut did_run: ResMut<DidRun>) { did_run.0 = true; }
/// # let mut schedule = Schedule::default();
/// schedule.add_systems(my_system.run_if(every_other_time()));
/// # let mut world = World::new();
/// # world.insert_resource(DidRun(false));
/// # schedule.run(&mut world);
/// # assert!(world.resource::<DidRun>().0);
/// # world.insert_resource(DidRun(false));
/// # schedule.run(&mut world);
/// # assert!(!world.resource::<DidRun>().0);
/// ```
///
/// A condition that takes a bool as an input and returns it unchanged.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// fn identity() -> impl Condition<(), bool> {
///     IntoSystem::into_system(|In(x)| x)
/// }
///
/// # fn always_true() -> bool { true }
/// # let mut app = Schedule::default();
/// # #[derive(Resource)] struct DidRun(bool);
/// # fn my_system(mut did_run: ResMut<DidRun>) { did_run.0 = true; }
/// app.add_systems(my_system.run_if(always_true.pipe(identity())));
/// # let mut world = World::new();
/// # world.insert_resource(DidRun(false));
/// # app.run(&mut world);
/// # assert!(world.resource::<DidRun>().0);
pub trait Condition<Marker, In = ()>: sealed::Condition<Marker, In> {
    /// Returns a new run condition that only returns `true`
    /// if both this one and the passed `and` return `true`.
    ///
    /// The returned run condition is short-circuiting, meaning
    /// `and` will only be invoked if `self` returns `true`.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Resource, PartialEq)]
    /// struct R(u32);
    ///
    /// # let mut app = Schedule::default();
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
    /// Use `.and()` to avoid checking the condition.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, PartialEq)]
    /// # struct R(u32);
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # fn my_system() {}
    /// app.add_systems(
    ///     // `resource_equals` will only get run if the resource `R` exists.
    ///     my_system.run_if(resource_exists::<R>.and(resource_equals(R(0)))),
    /// );
    /// # app.run(&mut world);
    /// ```
    ///
    /// Note that in this case, it's better to just use the run condition [`resource_exists_and_equals`].
    ///
    /// [`resource_exists_and_equals`]: common_conditions::resource_exists_and_equals
    fn and<M, C: Condition<M, In>>(self, and: C) -> And<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(and);
        let name = format!("{} && {}", a.name(), b.name());
        CombinatorSystem::new(a, b, Cow::Owned(name))
    }

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
    /// # let mut app = Schedule::default();
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
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # fn my_system() {}
    /// app.add_systems(
    ///     // `resource_equals` will only get run if the resource `R` exists.
    ///     my_system.run_if(resource_exists::<R>.and_then(resource_equals(R(0)))),
    /// );
    /// # app.run(&mut world);
    /// ```
    ///
    /// Note that in this case, it's better to just use the run condition [`resource_exists_and_equals`].
    ///
    /// [`resource_exists_and_equals`]: common_conditions::resource_exists_and_equals
    #[deprecated(
        note = "Users should use the `.and(condition)` method in lieu of `.and_then(condition)`"
    )]
    fn and_then<M, C: Condition<M, In>>(self, and_then: C) -> And<Self::System, C::System> {
        self.and(and_then)
    }

    /// Returns a new run condition that only returns `false`
    /// if both this one and the passed `nand` return `true`.
    ///
    /// The returned run condition is short-circuiting, meaning
    /// `nand` will only be invoked if `self` returns `true`.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// use bevy::prelude::*;
    ///
    /// #[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
    /// pub enum PlayerState {
    ///     Alive,
    ///     Dead,
    /// }
    ///
    /// #[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
    /// pub enum EnemyState {
    ///     Alive,
    ///     Dead,
    /// }
    ///
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # fn game_over_credits() {}
    /// app.add_systems(
    ///     // The game_over_credits system will only execute if either the `in_state(PlayerState::Alive)`
    ///     // run condition or `in_state(EnemyState::Alive)` run condition evaluates to `false`.
    ///     game_over_credits.run_if(
    ///         in_state(PlayerState::Alive).nand(in_state(EnemyState::Alive))
    ///     ),
    /// );
    /// # app.run(&mut world);
    /// ```
    ///
    /// Equivalent logic can be achieved by using `not` in concert with `and`:
    ///
    /// ```compile_fail
    /// app.add_systems(
    ///     game_over_credits.run_if(
    ///         not(in_state(PlayerState::Alive).and(in_state(EnemyState::Alive)))
    ///     ),
    /// );
    /// ```
    fn nand<M, C: Condition<M, In>>(self, nand: C) -> Nand<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(nand);
        let name = format!("!({} && {})", a.name(), b.name());
        CombinatorSystem::new(a, b, Cow::Owned(name))
    }

    /// Returns a new run condition that only returns `true`
    /// if both this one and the passed `nor` return `false`.
    ///
    /// The returned run condition is short-circuiting, meaning
    /// `nor` will only be invoked if `self` returns `false`.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// use bevy::prelude::*;
    ///
    /// #[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
    /// pub enum WeatherState {
    ///     Sunny,
    ///     Cloudy,
    /// }
    ///
    /// #[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
    /// pub enum SoilState {
    ///     Fertilized,
    ///     NotFertilized,
    /// }
    ///
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # fn slow_plant_growth() {}
    /// app.add_systems(
    ///     // The slow_plant_growth system will only execute if both the `in_state(WeatherState::Sunny)`
    ///     // run condition and `in_state(SoilState::Fertilized)` run condition evaluate to `false`.
    ///     slow_plant_growth.run_if(
    ///         in_state(WeatherState::Sunny).nor(in_state(SoilState::Fertilized))
    ///     ),
    /// );
    /// # app.run(&mut world);
    /// ```
    ///
    /// Equivalent logic can be achieved by using `not` in concert with `or`:
    ///
    /// ```compile_fail
    /// app.add_systems(
    ///     slow_plant_growth.run_if(
    ///         not(in_state(WeatherState::Sunny).or(in_state(SoilState::Fertilized)))
    ///     ),
    /// );
    /// ```
    fn nor<M, C: Condition<M, In>>(self, nor: C) -> Nor<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(nor);
        let name = format!("!({} || {})", a.name(), b.name());
        CombinatorSystem::new(a, b, Cow::Owned(name))
    }

    /// Returns a new run condition that returns `true`
    /// if either this one or the passed `or` return `true`.
    ///
    /// The returned run condition is short-circuiting, meaning
    /// `or` will only be invoked if `self` returns `false`.
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
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # #[derive(Resource)] struct C(bool);
    /// # fn my_system(mut c: ResMut<C>) { c.0 = true; }
    /// app.add_systems(
    ///     // Only run the system if either `A` or `B` exist.
    ///     my_system.run_if(resource_exists::<A>.or(resource_exists::<B>)),
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
    fn or<M, C: Condition<M, In>>(self, or: C) -> Or<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(or);
        let name = format!("{} || {}", a.name(), b.name());
        CombinatorSystem::new(a, b, Cow::Owned(name))
    }

    /// Returns a new run condition that returns `true`
    /// if either this one or the passed `or` return `true`.
    ///
    /// The returned run condition is short-circuiting, meaning
    /// `or` will only be invoked if `self` returns `false`.
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
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # #[derive(Resource)] struct C(bool);
    /// # fn my_system(mut c: ResMut<C>) { c.0 = true; }
    /// app.add_systems(
    ///     // Only run the system if either `A` or `B` exist.
    ///     my_system.run_if(resource_exists::<A>.or(resource_exists::<B>)),
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
    #[deprecated(
        note = "Users should use the `.or(condition)` method in lieu of `.or_else(condition)`"
    )]
    fn or_else<M, C: Condition<M, In>>(self, or_else: C) -> Or<Self::System, C::System> {
        self.or(or_else)
    }

    /// Returns a new run condition that only returns `true`
    /// if `self` and `xnor` **both** return `false` or **both** return `true`.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// use bevy::prelude::*;
    ///
    /// #[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
    /// pub enum CoffeeMachineState {
    ///     Heating,
    ///     Brewing,
    ///     Inactive,
    /// }
    ///
    /// #[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
    /// pub enum TeaKettleState {
    ///     Heating,
    ///     Steeping,
    ///     Inactive,
    /// }
    ///
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # fn take_drink_orders() {}
    /// app.add_systems(
    ///     // The take_drink_orders system will only execute if the `in_state(CoffeeMachineState::Inactive)`
    ///     // run condition and `in_state(TeaKettleState::Inactive)` run conditions both evaluate to `false`,
    ///     // or both evaluate to `true`.
    ///     take_drink_orders.run_if(
    ///         in_state(CoffeeMachineState::Inactive).xnor(in_state(TeaKettleState::Inactive))
    ///     ),
    /// );
    /// # app.run(&mut world);
    /// ```
    ///
    /// Equivalent logic can be achieved by using `not` in concert with `xor`:
    ///
    /// ```compile_fail
    /// app.add_systems(
    ///     take_drink_orders.run_if(
    ///         not(in_state(CoffeeMachineState::Inactive).xor(in_state(TeaKettleState::Inactive)))
    ///     ),
    /// );
    /// ```
    fn xnor<M, C: Condition<M, In>>(self, xnor: C) -> Xnor<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(xnor);
        let name = format!("!({} ^ {})", a.name(), b.name());
        CombinatorSystem::new(a, b, Cow::Owned(name))
    }

    /// Returns a new run condition that only returns `true`
    /// if either `self` or `xor` return `true`, but not both.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// use bevy::prelude::*;
    ///
    /// #[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
    /// pub enum CoffeeMachineState {
    ///     Heating,
    ///     Brewing,
    ///     Inactive,
    /// }
    ///
    /// #[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
    /// pub enum TeaKettleState {
    ///     Heating,
    ///     Steeping,
    ///     Inactive,
    /// }
    ///
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # fn prepare_beverage() {}
    /// app.add_systems(
    ///     // The prepare_beverage system will only execute if either the `in_state(CoffeeMachineState::Inactive)`
    ///     // run condition or `in_state(TeaKettleState::Inactive)` run condition evaluates to `true`,
    ///     // but not both.
    ///     prepare_beverage.run_if(
    ///         in_state(CoffeeMachineState::Inactive).xor(in_state(TeaKettleState::Inactive))
    ///     ),
    /// );
    /// # app.run(&mut world);
    /// ```
    fn xor<M, C: Condition<M, In>>(self, xor: C) -> Xor<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(xor);
        let name = format!("({} ^ {})", a.name(), b.name());
        CombinatorSystem::new(a, b, Cow::Owned(name))
    }
}

impl<Marker, In, F> Condition<Marker, In> for F where F: sealed::Condition<Marker, In> {}

mod sealed {
    use crate::system::{IntoSystem, ReadOnlySystem};

    pub trait Condition<Marker, In>:
        IntoSystem<In, bool, Marker, System = Self::ReadOnlySystem>
    {
        // This associated type is necessary to let the compiler
        // know that `Self::System` is `ReadOnlySystem`.
        type ReadOnlySystem: ReadOnlySystem<In = In, Out = bool>;
    }

    impl<Marker, In, F> Condition<Marker, In> for F
    where
        F: IntoSystem<In, bool, Marker>,
        F::System: ReadOnlySystem,
    {
        type ReadOnlySystem = F::System;
    }
}

/// A collection of [run conditions](Condition) that may be useful in any bevy app.
pub mod common_conditions {
    use super::NotSystem;
    use crate::{
        change_detection::DetectChanges,
        event::{Event, EventReader},
        prelude::{Component, Query, With},
        removal_detection::RemovedComponents,
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
    /// # let mut app = Schedule::default();
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

    /// A [`Condition`](super::Condition)-satisfying system that returns `true`
    /// if the resource exists.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// app.add_systems(
    ///     // `resource_exists` will only return true if the given resource exists in the world
    ///     my_system.run_if(resource_exists::<Counter>),
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
    pub fn resource_exists<T>(res: Option<Res<T>>) -> bool
    where
        T: Resource,
    {
        res.is_some()
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
    /// # let mut app = Schedule::default();
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
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// app.add_systems(
    ///     // `resource_exists_and_equals` will only return true
    ///     // if the given resource exists and equals the given value
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

    /// A [`Condition`](super::Condition)-satisfying system that returns `true`
    /// if the resource of the given type has been added since the condition was last checked.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// app.add_systems(
    ///     // `resource_added` will only return true if the
    ///     // given resource was just added
    ///     my_system.run_if(resource_added::<Counter>),
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
    pub fn resource_added<T>(res: Option<Res<T>>) -> bool
    where
        T: Resource,
    {
        match res {
            Some(res) => res.is_added(),
            None => false,
        }
    }

    /// A [`Condition`](super::Condition)-satisfying system that returns `true`
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
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// app.add_systems(
    ///     // `resource_changed` will only return true if the
    ///     // given resource was just changed (or added)
    ///     my_system.run_if(
    ///         resource_changed::<Counter>
    ///         // By default detecting changes will also trigger if the resource was
    ///         // just added, this won't work with my example so I will add a second
    ///         // condition to make sure the resource wasn't just added
    ///         .and(not(resource_added::<Counter>))
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
    pub fn resource_changed<T>(res: Res<T>) -> bool
    where
        T: Resource,
    {
        res.is_changed()
    }

    /// A [`Condition`](super::Condition)-satisfying system that returns `true`
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
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// app.add_systems(
    ///     // `resource_exists_and_changed` will only return true if the
    ///     // given resource exists and was just changed (or added)
    ///     my_system.run_if(
    ///         resource_exists_and_changed::<Counter>
    ///         // By default detecting changes will also trigger if the resource was
    ///         // just added, this won't work with my example so I will add a second
    ///         // condition to make sure the resource wasn't just added
    ///         .and(not(resource_added::<Counter>))
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
    pub fn resource_exists_and_changed<T>(res: Option<Res<T>>) -> bool
    where
        T: Resource,
    {
        match res {
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
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// app.add_systems(
    ///     // `resource_changed_or_removed` will only return true if the
    ///     // given resource was just changed or removed (or added)
    ///     my_system.run_if(
    ///         resource_changed_or_removed::<Counter>()
    ///         // By default detecting changes will also trigger if the resource was
    ///         // just added, this won't work with my example so I will add a second
    ///         // condition to make sure the resource wasn't just added
    ///         .and(not(resource_added::<Counter>))
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
    /// # let mut app = Schedule::default();
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
    /// if there are any new events of the given type since it was last called.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// # world.init_resource::<Events<MyEvent>>();
    /// # app.add_systems(bevy_ecs::event::event_update_system.before(my_system));
    ///
    /// app.add_systems(
    ///     my_system.run_if(on_event::<MyEvent>()),
    /// );
    ///
    /// #[derive(Event)]
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
        move |mut reader: EventReader<T>| reader.read().count() > 0
    }

    /// A [`Condition`](super::Condition)-satisfying system that returns `true`
    /// if there are any entities with the given component type.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::default();
    /// # let mut world = World::new();
    /// # world.init_resource::<Counter>();
    /// app.add_systems(
    ///     my_system.run_if(any_with_component::<MyComponent>),
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
    pub fn any_with_component<T: Component>(query: Query<(), With<T>>) -> bool {
        !query.is_empty()
    }

    /// Generates a [`Condition`](super::Condition)-satisfying closure that returns `true`
    /// if there are any entity with a component of the given type removed.
    pub fn any_component_removed<T: Component>() -> impl FnMut(RemovedComponents<T>) -> bool {
        // `RemovedComponents` based on events and therefore events need to be consumed,
        // so that there are no false positives on subsequent calls of the run condition.
        // Simply checking `is_empty` would not be enough.
        // PERF: note that `count` is efficient (not actually looping/iterating),
        // due to Bevy having a specialized implementation for events.
        move |mut removals: RemovedComponents<T>| removals.read().count() != 0
    }

    /// Generates a [`Condition`](super::Condition) that inverses the result of passed one.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource, Default)]
    /// # struct Counter(u8);
    /// # let mut app = Schedule::default();
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
        NotSystem::new(super::NotMarker, condition, name.into())
    }
}

/// Invokes [`Not`] with the output of another system.
///
/// See [`common_conditions::not`] for examples.
pub type NotSystem<T> = AdapterSystem<NotMarker, T>;

/// Used with [`AdapterSystem`] to negate the output of a system via the [`Not`] operator.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct NotMarker;

impl<T: System> Adapt<T> for NotMarker
where
    T::Out: Not,
{
    type In = T::In;
    type Out = <T::Out as Not>::Output;

    fn adapt(&mut self, input: Self::In, run_system: impl FnOnce(T::In) -> T::Out) -> Self::Out {
        !run_system(input)
    }
}

/// Combines the outputs of two systems using the `&&` operator.
pub type And<A, B> = CombinatorSystem<AndMarker, A, B>;

/// Combines and inverts the outputs of two systems using the `&&` and `!` operators.
pub type Nand<A, B> = CombinatorSystem<NandMarker, A, B>;

/// Combines and inverts the outputs of two systems using the `&&` and `!` operators.
pub type Nor<A, B> = CombinatorSystem<NorMarker, A, B>;

/// Combines the outputs of two systems using the `||` operator.
pub type Or<A, B> = CombinatorSystem<OrMarker, A, B>;

/// Combines and inverts the outputs of two systems using the `^` and `!` operators.
pub type Xnor<A, B> = CombinatorSystem<XnorMarker, A, B>;

/// Combines the outputs of two systems using the `^` operator.
pub type Xor<A, B> = CombinatorSystem<XorMarker, A, B>;

#[doc(hidden)]
pub struct AndMarker;

impl<In, A, B> Combine<A, B> for AndMarker
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
pub struct NandMarker;

impl<In, A, B> Combine<A, B> for NandMarker
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
        !(a(input) && b(input))
    }
}

#[doc(hidden)]
pub struct NorMarker;

impl<In, A, B> Combine<A, B> for NorMarker
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
        !(a(input) || b(input))
    }
}

#[doc(hidden)]
pub struct OrMarker;

impl<In, A, B> Combine<A, B> for OrMarker
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

#[doc(hidden)]
pub struct XnorMarker;

impl<In, A, B> Combine<A, B> for XnorMarker
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
        !(a(input) ^ b(input))
    }
}

#[doc(hidden)]
pub struct XorMarker;

impl<In, A, B> Combine<A, B> for XorMarker
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
        a(input) ^ b(input)
    }
}

#[cfg(test)]
mod tests {
    use super::{common_conditions::*, Condition};
    use crate as bevy_ecs;
    use crate::component::Component;
    use crate::schedule::IntoSystemConfigs;
    use crate::system::Local;
    use crate::{change_detection::ResMut, schedule::Schedule, world::World};
    use bevy_ecs_macros::Event;
    use bevy_ecs_macros::Resource;

    #[derive(Resource, Default)]
    struct Counter(usize);

    fn increment_counter(mut counter: ResMut<Counter>) {
        counter.0 += 1;
    }

    fn double_counter(mut counter: ResMut<Counter>) {
        counter.0 *= 2;
    }

    fn every_other_time(mut has_ran: Local<bool>) -> bool {
        *has_ran = !*has_ran;
        *has_ran
    }

    #[test]
    fn run_condition() {
        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::default();

        // Run every other cycle
        schedule.add_systems(increment_counter.run_if(every_other_time));

        schedule.run(&mut world);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 1);
        schedule.run(&mut world);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 2);

        // Run every other cycle opposite to the last one
        schedule.add_systems(increment_counter.run_if(not(every_other_time)));

        schedule.run(&mut world);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 4);
        schedule.run(&mut world);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 6);
    }

    #[test]
    #[allow(deprecated)]
    fn run_condition_combinators() {
        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::default();

        schedule.add_systems(
            (
                increment_counter.run_if(every_other_time.and(|| true)), // Run every odd cycle.
                increment_counter.run_if(every_other_time.and_then(|| true)), // Run every odd cycle.
                increment_counter.run_if(every_other_time.nand(|| false)),    // Always run.
                double_counter.run_if(every_other_time.nor(|| false)), // Run every even cycle.
                increment_counter.run_if(every_other_time.or(|| true)), // Always run.
                increment_counter.run_if(every_other_time.or_else(|| true)), // Always run.
                increment_counter.run_if(every_other_time.xnor(|| true)), // Run every odd cycle.
                double_counter.run_if(every_other_time.xnor(|| false)), // Run every even cycle.
                increment_counter.run_if(every_other_time.xor(|| false)), // Run every odd cycle.
                double_counter.run_if(every_other_time.xor(|| true)),  // Run every even cycle.
            )
                .chain(),
        );

        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 7);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 72);
    }

    #[test]
    fn multiple_run_conditions() {
        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::default();

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

        let mut schedule = Schedule::default();

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
    #[derive(Component)]
    struct TestComponent;

    #[derive(Event)]
    struct TestEvent;

    #[derive(Resource)]
    struct TestResource(());

    fn test_system() {}

    // Ensure distributive_run_if compiles with the common conditions.
    #[test]
    fn distributive_run_if_compiles() {
        Schedule::default().add_systems(
            (test_system, test_system)
                .distributive_run_if(run_once())
                .distributive_run_if(resource_exists::<TestResource>)
                .distributive_run_if(resource_added::<TestResource>)
                .distributive_run_if(resource_changed::<TestResource>)
                .distributive_run_if(resource_exists_and_changed::<TestResource>)
                .distributive_run_if(resource_changed_or_removed::<TestResource>())
                .distributive_run_if(resource_removed::<TestResource>())
                .distributive_run_if(on_event::<TestEvent>())
                .distributive_run_if(any_with_component::<TestComponent>)
                .distributive_run_if(not(run_once())),
        );
    }
}
