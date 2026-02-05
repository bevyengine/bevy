use alloc::{boxed::Box, format};
use bevy_utils::prelude::DebugName;
use core::ops::Not;

use crate::system::{
    Adapt, AdapterSystem, CombinatorSystem, Combine, IntoSystem, ReadOnlySystem, RunSystemError,
    System, SystemIn, SystemInput,
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
/// `SystemCondition` trait has `Marker` type parameter, which has no special meaning,
/// but exists to work around the limitation of Rust's trait system.
///
/// Type parameter in return type can be set to `<()>` by calling [`IntoSystem::into_system`],
/// but usually have to be specified when passing a condition to a function.
///
/// ```
/// # use bevy_ecs::schedule::SystemCondition;
/// # use bevy_ecs::system::IntoSystem;
/// fn not_condition<Marker>(a: impl SystemCondition<Marker>) -> impl SystemCondition<()> {
///    IntoSystem::into_system(a.map(|x| !x))
/// }
/// ```
///
/// # Examples
/// A condition that returns true every other time it's called.
/// ```
/// # use bevy_ecs::prelude::*;
/// fn every_other_time() -> impl SystemCondition<()> {
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
/// fn identity() -> impl SystemCondition<(), In<bool>> {
///     IntoSystem::into_system(|In(x): In<bool>| x)
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
pub trait SystemCondition<Marker, In: SystemInput = ()>:
    IntoSystem<In, bool, Marker, System: ReadOnlySystem>
{
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
    fn and<M, C: SystemCondition<M, In>>(self, and: C) -> And<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(and);
        let name = format!("{} && {}", a.name(), b.name());
        CombinatorSystem::new(a, b, DebugName::owned(name))
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
    fn nand<M, C: SystemCondition<M, In>>(self, nand: C) -> Nand<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(nand);
        let name = format!("!({} && {})", a.name(), b.name());
        CombinatorSystem::new(a, b, DebugName::owned(name))
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
    fn nor<M, C: SystemCondition<M, In>>(self, nor: C) -> Nor<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(nor);
        let name = format!("!({} || {})", a.name(), b.name());
        CombinatorSystem::new(a, b, DebugName::owned(name))
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
    fn or<M, C: SystemCondition<M, In>>(self, or: C) -> Or<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(or);
        let name = format!("{} || {}", a.name(), b.name());
        CombinatorSystem::new(a, b, DebugName::owned(name))
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
    fn xnor<M, C: SystemCondition<M, In>>(self, xnor: C) -> Xnor<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(xnor);
        let name = format!("!({} ^ {})", a.name(), b.name());
        CombinatorSystem::new(a, b, DebugName::owned(name))
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
    fn xor<M, C: SystemCondition<M, In>>(self, xor: C) -> Xor<Self::System, C::System> {
        let a = IntoSystem::into_system(self);
        let b = IntoSystem::into_system(xor);
        let name = format!("({} ^ {})", a.name(), b.name());
        CombinatorSystem::new(a, b, DebugName::owned(name))
    }
}

impl<Marker, In: SystemInput, F> SystemCondition<Marker, In> for F where
    F: IntoSystem<In, bool, Marker, System: ReadOnlySystem>
{
}

/// A collection of [run conditions](SystemCondition) that may be useful in any bevy app.
pub mod common_conditions {
    use super::{NotSystem, SystemCondition};
    use crate::{
        change_detection::DetectChanges,
        lifecycle::RemovedComponents,
        message::{Message, MessageReader},
        prelude::{Component, Query, With},
        query::QueryFilter,
        resource::Resource,
        system::{In, IntoSystem, Local, Res, System, SystemInput},
    };
    use alloc::format;

    /// A [`SystemCondition`]-satisfying system that returns `true`
    /// on the first time the condition is run and false every time after.
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
    ///     my_system.run_if(run_once),
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
    pub fn run_once(mut has_run: Local<bool>) -> bool {
        if !*has_run {
            *has_run = true;
            true
        } else {
            false
        }
    }

    /// A [`SystemCondition`]-satisfying system that returns `true`
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

    /// Generates a [`SystemCondition`]-satisfying closure that returns `true`
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

    /// Generates a [`SystemCondition`]-satisfying closure that returns `true`
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

    /// A [`SystemCondition`]-satisfying system that returns `true`
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

    /// A [`SystemCondition`]-satisfying system that returns `true`
    /// if the resource of the given type has been added or mutably dereferenced
    /// since the condition was last checked.
    ///
    /// **Note** that simply *mutably dereferencing* a resource is considered a change ([`DerefMut`](std::ops::DerefMut)).
    /// Bevy does not compare resources to their previous values.
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

    /// A [`SystemCondition`]-satisfying system that returns `true`
    /// if the resource of the given type has been added or mutably dereferenced since the condition
    /// was last checked.
    ///
    /// **Note** that simply *mutably dereferencing* a resource is considered a change ([`DerefMut`](std::ops::DerefMut)).
    /// Bevy does not compare resources to their previous values.
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

    /// A [`SystemCondition`]-satisfying system that returns `true`
    /// if the resource of the given type has been added, removed or mutably dereferenced since the condition
    /// was last checked.
    ///
    /// **Note** that simply *mutably dereferencing* a resource is considered a change ([`DerefMut`](std::ops::DerefMut)).
    /// Bevy does not compare resources to their previous values.
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
    ///         resource_changed_or_removed::<Counter>
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
    pub fn resource_changed_or_removed<T>(res: Option<Res<T>>, mut existed: Local<bool>) -> bool
    where
        T: Resource,
    {
        if let Some(value) = res {
            *existed = true;
            value.is_changed()
        } else if *existed {
            *existed = false;
            true
        } else {
            false
        }
    }

    /// A [`SystemCondition`]-satisfying system that returns `true`
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
    ///     my_system.run_if(resource_removed::<MyResource>),
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
    pub fn resource_removed<T>(res: Option<Res<T>>, mut existed: Local<bool>) -> bool
    where
        T: Resource,
    {
        if res.is_some() {
            *existed = true;
            false
        } else if *existed {
            *existed = false;
            true
        } else {
            false
        }
    }

    /// A [`SystemCondition`]-satisfying system that returns `true`
    /// if there are any new messages of the given type since it was last called.
    ///
    /// To skip a system based on messages that it reads, use [`PopulatedMessageReader`](crate::prelude::PopulatedMessageReader) instead.
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
    /// # world.init_resource::<Messages<MyMessage>>();
    /// # app.add_systems(bevy_ecs::message::message_update_system.before(my_system));
    ///
    /// app.add_systems(
    ///     my_system.run_if(on_message::<MyMessage>),
    /// );
    ///
    /// #[derive(Message)]
    /// struct MyMessage;
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // No new `MyMessage` messages have been pushed so `my_system` won't run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 0);
    ///
    /// world.resource_mut::<Messages<MyMessage>>().write(MyMessage);
    ///
    /// // A `MyMessage` message has been pushed so `my_system` will run
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// ```
    pub fn on_message<M: Message>(mut reader: MessageReader<M>) -> bool {
        // The messages need to be consumed, so that there are no false positives on subsequent
        // calls of the run condition. Simply checking `is_empty` would not be enough.
        // PERF: note that `count` is efficient (not actually looping/iterating),
        // due to Bevy having a specialized implementation for messages.
        reader.read().count() > 0
    }

    /// A [`SystemCondition`]-satisfying system that returns `true`
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

    /// A [`SystemCondition`]-satisfying system that returns `true`
    /// if there are any entity with a component of the given type removed.
    pub fn any_component_removed<T: Component>(mut removals: RemovedComponents<T>) -> bool {
        // `RemovedComponents` based on events and therefore events need to be consumed,
        // so that there are no false positives on subsequent calls of the run condition.
        // Simply checking `is_empty` would not be enough.
        // PERF: note that `count` is efficient (not actually looping/iterating),
        // due to Bevy having a specialized implementation for events.
        removals.read().count() > 0
    }

    /// A [`SystemCondition`]-satisfying system that returns `true`
    /// if there are any entities that match the given [`QueryFilter`].
    pub fn any_match_filter<F: QueryFilter>(query: Query<(), F>) -> bool {
        !query.is_empty()
    }

    /// Generates a [`SystemCondition`] that inverses the result of passed one.
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
        TOut: core::ops::Not,
        T: IntoSystem<(), TOut, Marker>,
    {
        let condition = IntoSystem::into_system(condition);
        let name = format!("!{}", condition.name());
        NotSystem::new(super::NotMarker, condition, name.into())
    }

    /// Generates a [`SystemCondition`] that returns true when the passed one changes.
    ///
    /// The first time this is called, the passed condition is assumed to have been previously false.
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
    ///     my_system.run_if(condition_changed(resource_exists::<MyResource>)),
    /// );
    ///
    /// #[derive(Resource)]
    /// struct MyResource;
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // `MyResource` is initially there, the inner condition is true, the system runs once
    /// world.insert_resource(MyResource);
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    ///
    /// // We remove `MyResource`, the inner condition is now false, the system runs one more time.
    /// world.remove_resource::<MyResource>();
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 2);
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 2);
    /// ```
    pub fn condition_changed<Marker, CIn, C>(condition: C) -> impl SystemCondition<(), CIn>
    where
        CIn: SystemInput,
        C: SystemCondition<Marker, CIn>,
    {
        IntoSystem::into_system(condition.pipe(|In(new): In<bool>, mut prev: Local<bool>| {
            let changed = *prev != new;
            *prev = new;
            changed
        }))
    }

    /// Generates a [`SystemCondition`] that returns true when the result of
    /// the passed one went from false to true since the last time this was called.
    ///
    /// The first time this is called, the passed condition is assumed to have been previously false.
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
    ///     my_system.run_if(condition_changed_to(true, resource_exists::<MyResource>)),
    /// );
    ///
    /// #[derive(Resource)]
    /// struct MyResource;
    ///
    /// fn my_system(mut counter: ResMut<Counter>) {
    ///     counter.0 += 1;
    /// }
    ///
    /// // `MyResource` is initially there, the inner condition is true, the system runs once
    /// world.insert_resource(MyResource);
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    ///
    /// // We remove `MyResource`, the inner condition is now false, the system doesn't run.
    /// world.remove_resource::<MyResource>();
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 1);
    ///
    /// // We reinsert `MyResource` again, so the system will run one more time
    /// world.insert_resource(MyResource);
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 2);
    /// app.run(&mut world);
    /// assert_eq!(world.resource::<Counter>().0, 2);
    /// ```
    pub fn condition_changed_to<Marker, CIn, C>(
        to: bool,
        condition: C,
    ) -> impl SystemCondition<(), CIn>
    where
        CIn: SystemInput,
        C: SystemCondition<Marker, CIn>,
    {
        IntoSystem::into_system(condition.pipe(
            move |In(new): In<bool>, mut prev: Local<bool>| -> bool {
                let now_true = *prev != new && new == to;
                *prev = new;
                now_true
            },
        ))
    }
}

/// Invokes [`Not`] with the output of another system.
///
/// See [`common_conditions::not`] for examples.
pub type NotSystem<S> = AdapterSystem<NotMarker, S>;

/// Used with [`AdapterSystem`] to negate the output of a system via the [`Not`] operator.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct NotMarker;

impl<S: System<Out: Not>> Adapt<S> for NotMarker {
    type In = S::In;
    type Out = <S::Out as Not>::Output;

    fn adapt(
        &mut self,
        input: <Self::In as SystemInput>::Inner<'_>,
        run_system: impl FnOnce(SystemIn<'_, S>) -> Result<S::Out, RunSystemError>,
    ) -> Result<Self::Out, RunSystemError> {
        run_system(input).map(Not::not)
    }
}

/// Combines the outputs of two systems using the `&&` operator.
pub type And<A, B> = CombinatorSystem<AndMarker, A, B>;

/// Combines and inverts the outputs of two systems using the `&&` and `!` operators.
pub type Nand<A, B> = CombinatorSystem<NandMarker, A, B>;

/// Combines and inverts the outputs of two systems using the `||` and `!` operators.
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
    for<'a> In: SystemInput<Inner<'a>: Copy>,
    A: System<In = In, Out = bool>,
    B: System<In = In, Out = bool>,
{
    type In = In;
    type Out = bool;

    fn combine<T>(
        input: <Self::In as SystemInput>::Inner<'_>,
        data: &mut T,
        a: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<A::Out, RunSystemError>,
        b: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<B::Out, RunSystemError>,
    ) -> Result<Self::Out, RunSystemError> {
        Ok(a(input, data).unwrap_or(false) && b(input, data).unwrap_or(false))
    }
}

#[doc(hidden)]
pub struct NandMarker;

impl<In, A, B> Combine<A, B> for NandMarker
where
    for<'a> In: SystemInput<Inner<'a>: Copy>,
    A: System<In = In, Out = bool>,
    B: System<In = In, Out = bool>,
{
    type In = In;
    type Out = bool;

    fn combine<T>(
        input: <Self::In as SystemInput>::Inner<'_>,
        data: &mut T,
        a: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<A::Out, RunSystemError>,
        b: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<B::Out, RunSystemError>,
    ) -> Result<Self::Out, RunSystemError> {
        Ok(!(a(input, data).unwrap_or(false) && b(input, data).unwrap_or(false)))
    }
}

#[doc(hidden)]
pub struct NorMarker;

impl<In, A, B> Combine<A, B> for NorMarker
where
    for<'a> In: SystemInput<Inner<'a>: Copy>,
    A: System<In = In, Out = bool>,
    B: System<In = In, Out = bool>,
{
    type In = In;
    type Out = bool;

    fn combine<T>(
        input: <Self::In as SystemInput>::Inner<'_>,
        data: &mut T,
        a: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<A::Out, RunSystemError>,
        b: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<B::Out, RunSystemError>,
    ) -> Result<Self::Out, RunSystemError> {
        Ok(!(a(input, data).unwrap_or(false) || b(input, data).unwrap_or(false)))
    }
}

#[doc(hidden)]
pub struct OrMarker;

impl<In, A, B> Combine<A, B> for OrMarker
where
    for<'a> In: SystemInput<Inner<'a>: Copy>,
    A: System<In = In, Out = bool>,
    B: System<In = In, Out = bool>,
{
    type In = In;
    type Out = bool;

    fn combine<T>(
        input: <Self::In as SystemInput>::Inner<'_>,
        data: &mut T,
        a: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<A::Out, RunSystemError>,
        b: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<B::Out, RunSystemError>,
    ) -> Result<Self::Out, RunSystemError> {
        Ok(a(input, data).unwrap_or(false) || b(input, data).unwrap_or(false))
    }
}

#[doc(hidden)]
pub struct XnorMarker;

impl<In, A, B> Combine<A, B> for XnorMarker
where
    for<'a> In: SystemInput<Inner<'a>: Copy>,
    A: System<In = In, Out = bool>,
    B: System<In = In, Out = bool>,
{
    type In = In;
    type Out = bool;

    fn combine<T>(
        input: <Self::In as SystemInput>::Inner<'_>,
        data: &mut T,
        a: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<A::Out, RunSystemError>,
        b: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<B::Out, RunSystemError>,
    ) -> Result<Self::Out, RunSystemError> {
        Ok(!(a(input, data).unwrap_or(false) ^ b(input, data).unwrap_or(false)))
    }
}

#[doc(hidden)]
pub struct XorMarker;

impl<In, A, B> Combine<A, B> for XorMarker
where
    for<'a> In: SystemInput<Inner<'a>: Copy>,
    A: System<In = In, Out = bool>,
    B: System<In = In, Out = bool>,
{
    type In = In;
    type Out = bool;

    fn combine<T>(
        input: <Self::In as SystemInput>::Inner<'_>,
        data: &mut T,
        a: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<A::Out, RunSystemError>,
        b: impl FnOnce(SystemIn<'_, A>, &mut T) -> Result<B::Out, RunSystemError>,
    ) -> Result<Self::Out, RunSystemError> {
        Ok(a(input, data).unwrap_or(false) ^ b(input, data).unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::{common_conditions::*, SystemCondition};
    use crate::error::{BevyError, DefaultErrorHandler, ErrorContext};
    use crate::{
        change_detection::{Res, ResMut},
        component::Component,
        message::Message,
        query::With,
        schedule::{IntoScheduleConfigs, Schedule},
        system::{IntoSystem, Local, System},
        world::World,
    };
    use bevy_ecs_macros::{Resource, SystemSet};

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
    fn combinators_with_maybe_failing_condition() {
        #![allow(
            clippy::nonminimal_bool,
            clippy::overly_complex_bool_expr,
            reason = "Trailing `|| false` and `&& true` are used in this test to visually remain consistent with the combinators"
        )]

        use crate::system::RunSystemOnce;
        use alloc::sync::Arc;
        use std::sync::Mutex;

        // Things that should be tested:
        // - the final result of the combinator is correct
        // - the systems that are expected to run do run
        // - the systems that are expected to not run do not run

        #[derive(Component)]
        struct Vacant;

        // SystemConditions don't have mutable access to the world, so we use a
        // `Res<AtomicCounter>` to count invocations.
        #[derive(Resource, Default)]
        struct Counter(Arc<Mutex<usize>>);

        // The following constants are used to represent a system having run.
        // both are prime so that when multiplied they give a unique value for any TRUE^n*FALSE^m
        const FALSE: usize = 2;
        const TRUE: usize = 3;

        // this is a system, but has the same side effect as `test_true`
        fn is_true_inc(counter: Res<Counter>) -> bool {
            test_true(&counter)
        }

        // this is a system, but has the same side effect as `test_false`
        fn is_false_inc(counter: Res<Counter>) -> bool {
            test_false(&counter)
        }

        // This condition will always yield `false`, because `Vacant` is never present.
        fn vacant(_: crate::system::Single<&Vacant>) -> bool {
            true
        }

        fn test_true(counter: &Counter) -> bool {
            *counter.0.lock().unwrap() *= TRUE;
            true
        }

        fn test_false(counter: &Counter) -> bool {
            *counter.0.lock().unwrap() *= FALSE;
            false
        }

        // Helper function that runs a logic call and returns the result, as
        // well as the composite number of the calls.
        fn logic_call_result(f: impl FnOnce(&Counter) -> bool) -> (usize, bool) {
            let counter = Counter(Arc::new(Mutex::new(1)));
            let result = f(&counter);
            (*counter.0.lock().unwrap(), result)
        }

        // `test_true` and `test_false` can't fail like the systems can, and so
        // we use them to model the short circuiting behavior of rust's logical
        // operators. The goal is to end up with a composite number that
        // describes rust's behavior and compare that to the result of the
        // combinators.

        // we expect `true() || false()` to yield `true`, and short circuit
        // after `true()`
        assert_eq!(
            logic_call_result(|c| test_true(c) || test_false(c)),
            (TRUE.pow(1) * FALSE.pow(0), true)
        );

        let mut world = World::new();
        world.init_resource::<Counter>();

        // ensure there are no `Vacant` entities
        assert!(world.query::<&Vacant>().iter(&world).next().is_none());
        assert!(matches!(
            world.run_system_once((|| true).or(vacant)),
            Ok(true)
        ));

        // This system should fail
        assert!(RunSystemOnce::run_system_once(&mut world, vacant).is_err());

        #[track_caller]
        fn assert_system<Marker>(
            world: &mut World,
            system: impl IntoSystem<(), bool, Marker>,
            equivalent_to: impl FnOnce(&Counter) -> bool,
        ) {
            use crate::system::System;

            *world.resource::<Counter>().0.lock().unwrap() = 1;

            let system = IntoSystem::into_system(system);
            let name = system.name();

            let out = RunSystemOnce::run_system_once(&mut *world, system).unwrap_or(false);

            let (expected_counter, expected) = logic_call_result(equivalent_to);
            let caller = core::panic::Location::caller();
            let counter = *world.resource::<Counter>().0.lock().unwrap();

            assert_eq!(
                out,
                expected,
                "At {}:{} System `{name}` yielded unexpected value `{out}`, expected `{expected}`",
                caller.file(),
                caller.line(),
            );

            assert_eq!(
                counter, expected_counter,
                "At {}:{} System `{name}` did not increment counter as expected: expected `{expected_counter}`, got `{counter}`",
                caller.file(),
                caller.line(),
            );
        }

        assert_system(&mut world, is_true_inc.or(vacant), |c| {
            test_true(c) || false
        });
        assert_system(&mut world, is_true_inc.nor(vacant), |c| {
            !(test_true(c) || false)
        });
        assert_system(&mut world, is_true_inc.xor(vacant), |c| {
            test_true(c) ^ false
        });
        assert_system(&mut world, is_true_inc.xnor(vacant), |c| {
            !(test_true(c) ^ false)
        });
        assert_system(&mut world, is_true_inc.and(vacant), |c| {
            test_true(c) && false
        });
        assert_system(&mut world, is_true_inc.nand(vacant), |c| {
            !(test_true(c) && false)
        });

        // even if `vacant` fails as the first condition, where applicable (or,
        // xor), `is_true_inc` should still be called. `and` and `nand` short
        // circuit on an initial `false`.
        assert_system(&mut world, vacant.or(is_true_inc), |c| {
            false || test_true(c)
        });
        assert_system(&mut world, vacant.nor(is_true_inc), |c| {
            !(false || test_true(c))
        });
        assert_system(&mut world, vacant.xor(is_true_inc), |c| {
            false ^ test_true(c)
        });
        assert_system(&mut world, vacant.xnor(is_true_inc), |c| {
            !(false ^ test_true(c))
        });
        assert_system(&mut world, vacant.and(is_true_inc), |c| {
            false && test_true(c)
        });
        assert_system(&mut world, vacant.nand(is_true_inc), |c| {
            !(false && test_true(c))
        });

        // the same logic ought to be the case with a condition that runs, but yields `false`:
        assert_system(&mut world, is_true_inc.or(is_false_inc), |c| {
            test_true(c) || test_false(c)
        });
        assert_system(&mut world, is_true_inc.nor(is_false_inc), |c| {
            !(test_true(c) || test_false(c))
        });
        assert_system(&mut world, is_true_inc.xor(is_false_inc), |c| {
            test_true(c) ^ test_false(c)
        });
        assert_system(&mut world, is_true_inc.xnor(is_false_inc), |c| {
            !(test_true(c) ^ test_false(c))
        });
        assert_system(&mut world, is_true_inc.and(is_false_inc), |c| {
            test_true(c) && test_false(c)
        });
        assert_system(&mut world, is_true_inc.nand(is_false_inc), |c| {
            !(test_true(c) && test_false(c))
        });

        // and where one condition yields `false` and the other fails:
        assert_system(&mut world, is_false_inc.or(vacant), |c| {
            test_false(c) || false
        });
        assert_system(&mut world, is_false_inc.nor(vacant), |c| {
            !(test_false(c) || false)
        });
        assert_system(&mut world, is_false_inc.xor(vacant), |c| {
            test_false(c) ^ false
        });
        assert_system(&mut world, is_false_inc.xnor(vacant), |c| {
            !(test_false(c) ^ false)
        });
        assert_system(&mut world, is_false_inc.and(vacant), |c| {
            test_false(c) && false
        });
        assert_system(&mut world, is_false_inc.nand(vacant), |c| {
            !(test_false(c) && false)
        });

        // and where both conditions yield `true`:
        assert_system(&mut world, is_true_inc.or(is_true_inc), |c| {
            test_true(c) || test_true(c)
        });
        assert_system(&mut world, is_true_inc.nor(is_true_inc), |c| {
            !(test_true(c) || test_true(c))
        });
        assert_system(&mut world, is_true_inc.xor(is_true_inc), |c| {
            test_true(c) ^ test_true(c)
        });
        assert_system(&mut world, is_true_inc.xnor(is_true_inc), |c| {
            !(test_true(c) ^ test_true(c))
        });
        assert_system(&mut world, is_true_inc.and(is_true_inc), |c| {
            test_true(c) && test_true(c)
        });
        assert_system(&mut world, is_true_inc.nand(is_true_inc), |c| {
            !(test_true(c) && test_true(c))
        });

        // and where both conditions yield `false`:
        assert_system(&mut world, is_false_inc.or(is_false_inc), |c| {
            test_false(c) || test_false(c)
        });
        assert_system(&mut world, is_false_inc.nor(is_false_inc), |c| {
            !(test_false(c) || test_false(c))
        });
        assert_system(&mut world, is_false_inc.xor(is_false_inc), |c| {
            test_false(c) ^ test_false(c)
        });
        assert_system(&mut world, is_false_inc.xnor(is_false_inc), |c| {
            !(test_false(c) ^ test_false(c))
        });
        assert_system(&mut world, is_false_inc.and(is_false_inc), |c| {
            test_false(c) && test_false(c)
        });
        assert_system(&mut world, is_false_inc.nand(is_false_inc), |c| {
            !(test_false(c) && test_false(c))
        });
    }

    #[test]
    fn run_condition_combinators() {
        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::default();

        schedule.add_systems(
            (
                increment_counter.run_if(every_other_time.and(|| true)), // Run every odd cycle.
                increment_counter.run_if(every_other_time.nand(|| false)), // Always run.
                double_counter.run_if(every_other_time.nor(|| false)),   // Run every even cycle.
                increment_counter.run_if(every_other_time.or(|| true)),  // Always run.
                increment_counter.run_if(every_other_time.xnor(|| true)), // Run every odd cycle.
                double_counter.run_if(every_other_time.xnor(|| false)),  // Run every even cycle.
                increment_counter.run_if(every_other_time.xor(|| false)), // Run every odd cycle.
                double_counter.run_if(every_other_time.xor(|| true)),    // Run every even cycle.
            )
                .chain(),
        );

        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 5);
        schedule.run(&mut world);
        assert_eq!(world.resource::<Counter>().0, 52);
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

    #[derive(Message)]
    struct TestMessage;

    #[derive(Resource)]
    struct TestResource(());

    fn test_system() {}

    // Ensure distributive_run_if compiles with the common conditions.
    #[test]
    fn distributive_run_if_compiles() {
        Schedule::default().add_systems(
            (test_system, test_system)
                .distributive_run_if(run_once)
                .distributive_run_if(resource_exists::<TestResource>)
                .distributive_run_if(resource_added::<TestResource>)
                .distributive_run_if(resource_changed::<TestResource>)
                .distributive_run_if(resource_exists_and_changed::<TestResource>)
                .distributive_run_if(resource_changed_or_removed::<TestResource>)
                .distributive_run_if(resource_removed::<TestResource>)
                .distributive_run_if(on_message::<TestMessage>)
                .distributive_run_if(any_with_component::<TestComponent>)
                .distributive_run_if(any_match_filter::<With<TestComponent>>)
                .distributive_run_if(not(run_once)),
        );
    }

    #[test]
    fn run_if_error_contains_system() {
        let mut world = World::new();
        world.insert_resource(DefaultErrorHandler(my_error_handler));

        #[derive(Resource)]
        struct MyResource;

        fn condition(_res: Res<MyResource>) -> bool {
            true
        }

        fn my_error_handler(_: BevyError, ctx: ErrorContext) {
            let a = IntoSystem::into_system(system_a);
            let b = IntoSystem::into_system(system_b);
            assert!(
                matches!(ctx, ErrorContext::RunCondition { system, on_set, .. } if (on_set && system == b.name()) || (!on_set && system == a.name()))
            );
        }

        fn system_a() {}
        fn system_b() {}

        let mut schedule = Schedule::default();
        schedule.add_systems(system_a.run_if(condition));
        schedule.run(&mut world);

        #[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
        struct Set;

        let mut schedule = Schedule::default();
        schedule
            .add_systems((system_b,).in_set(Set))
            .configure_sets(Set.run_if(condition));
        schedule.run(&mut world);
    }
}
