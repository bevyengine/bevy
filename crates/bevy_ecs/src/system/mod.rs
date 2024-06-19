//! Tools for controlling behavior in an ECS application.
//!
//! Systems define how an ECS based application behaves.
//! Systems are added to a [`Schedule`](crate::schedule::Schedule), which is then run.
//! A system is usually written as a normal function, which is automatically converted into a system.
//!
//! System functions can have parameters, through which one can query and mutate Bevy ECS state.
//! Only types that implement [`SystemParam`] can be used, automatically fetching data from
//! the [`World`].
//!
//! System functions often look like this:
//!
//! ```
//! # use bevy_ecs::prelude::*;
//! #
//! # #[derive(Component)]
//! # struct Player { alive: bool }
//! # #[derive(Component)]
//! # struct Score(u32);
//! # #[derive(Resource)]
//! # struct Round(u32);
//! #
//! fn update_score_system(
//!     mut query: Query<(&Player, &mut Score)>,
//!     mut round: ResMut<Round>,
//! ) {
//!     for (player, mut score) in &mut query {
//!         if player.alive {
//!             score.0 += round.0;
//!         }
//!     }
//!     round.0 += 1;
//! }
//! # bevy_ecs::system::assert_is_system(update_score_system);
//! ```
//!
//! # System ordering
//!
//! By default, the execution of systems is parallel and not deterministic.
//! Not all systems can run together: if a system mutably accesses data,
//! no other system that reads or writes that data can be run at the same time.
//! These systems are said to be **incompatible**.
//!
//! The relative order in which incompatible systems are run matters.
//! When this is not specified, a **system order ambiguity** exists in your schedule.
//! You can **explicitly order** systems:
//!
//! - by calling the `.before(this_system)` or `.after(that_system)` methods when adding them to your schedule
//! - by adding them to a [`SystemSet`], and then using `.configure_sets(ThisSet.before(ThatSet))` syntax to configure many systems at once
//! - through the use of `.add_systems((system_a, system_b, system_c).chain())`
//!
//! [`SystemSet`]: crate::schedule::SystemSet
//!
//! ## Example
//!
//! ```
//! # use bevy_ecs::prelude::*;
//! # let mut schedule = Schedule::default();
//! # let mut world = World::new();
//! // Configure these systems to run in order using `chain()`.
//! schedule.add_systems((print_first, print_last).chain());
//! // Prints "HelloWorld!"
//! schedule.run(&mut world);
//!
//! // Configure this system to run in between the other two systems
//! // using explicit dependencies.
//! schedule.add_systems(print_mid.after(print_first).before(print_last));
//! // Prints "Hello, World!"
//! schedule.run(&mut world);
//!
//! fn print_first() {
//!     print!("Hello");
//! }
//! fn print_mid() {
//!     print!(", ");
//! }
//! fn print_last() {
//!     println!("World!");
//! }
//! ```
//!
//! # System parameter list
//! Following is the complete list of accepted types as system parameters:
//!
//! - [`Query`]
//! - [`Res`] and `Option<Res>`
//! - [`ResMut`] and `Option<ResMut>`
//! - [`Commands`]
//! - [`Local`]
//! - [`EventReader`](crate::event::EventReader)
//! - [`EventWriter`](crate::event::EventWriter)
//! - [`NonSend`] and `Option<NonSend>`
//! - [`NonSendMut`] and `Option<NonSendMut>`
//! - [`RemovedComponents`](crate::removal_detection::RemovedComponents)
//! - [`SystemName`]
//! - [`SystemChangeTick`]
//! - [`Archetypes`](crate::archetype::Archetypes) (Provides Archetype metadata)
//! - [`Bundles`](crate::bundle::Bundles) (Provides Bundles metadata)
//! - [`Components`](crate::component::Components) (Provides Components metadata)
//! - [`Entities`](crate::entity::Entities) (Provides Entities metadata)
//! - All tuples between 1 to 16 elements where each element implements [`SystemParam`]
//! - [`()` (unit primitive type)](https://doc.rust-lang.org/stable/std/primitive.unit.html)

mod adapter_system;
mod builder;
mod combinator;
mod commands;
mod exclusive_function_system;
mod exclusive_system_param;
mod function_system;
mod observer_system;
mod query;
#[allow(clippy::module_inception)]
mod system;
mod system_name;
mod system_param;
mod system_registry;

use std::{any::TypeId, borrow::Cow};

pub use adapter_system::*;
pub use builder::*;
pub use combinator::*;
pub use commands::*;
pub use exclusive_function_system::*;
pub use exclusive_system_param::*;
pub use function_system::*;
pub use observer_system::*;
pub use query::*;
pub use system::*;
pub use system_name::*;
pub use system_param::*;
pub use system_registry::*;

use crate::world::World;

/// Conversion trait to turn something into a [`System`].
///
/// Use this to get a system from a function. Also note that every system implements this trait as
/// well.
///
/// # Examples
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// fn my_system_function(a_usize_local: Local<usize>) {}
///
/// let system = IntoSystem::into_system(my_system_function);
/// ```
// This trait has to be generic because we have potentially overlapping impls, in particular
// because Rust thinks a type could impl multiple different `FnMut` combinations
// even though none can currently
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid system with input `{In}` and output `{Out}`",
    label = "invalid system"
)]
pub trait IntoSystem<In, Out, Marker>: Sized {
    /// The type of [`System`] that this instance converts into.
    type System: System<In = In, Out = Out>;

    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;

    /// Pass the output of this system `A` into a second system `B`, creating a new compound system.
    ///
    /// The second system must have [`In<T>`](crate::system::In) as its first parameter,
    /// where `T` is the return type of the first system.
    fn pipe<B, Final, MarkerB>(self, system: B) -> PipeSystem<Self::System, B::System>
    where
        B: IntoSystem<Out, Final, MarkerB>,
    {
        let system_a = IntoSystem::into_system(self);
        let system_b = IntoSystem::into_system(system);
        let name = format!("Pipe({}, {})", system_a.name(), system_b.name());
        PipeSystem::new(system_a, system_b, Cow::Owned(name))
    }

    /// Pass the output of this system into the passed function `f`, creating a new system that
    /// outputs the value returned from the function.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut schedule = Schedule::default();
    /// // Ignores the output of a system that may fail.
    /// schedule.add_systems(my_system.map(drop));
    /// # let mut world = World::new();
    /// # world.insert_resource(T);
    /// # schedule.run(&mut world);
    ///
    /// # #[derive(Resource)] struct T;
    /// # type Err = ();
    /// fn my_system(res: Res<T>) -> Result<(), Err> {
    ///     // ...
    ///     # Err(())
    /// }
    /// ```
    fn map<T, F>(self, f: F) -> AdapterSystem<F, Self::System>
    where
        F: Send + Sync + 'static + FnMut(Out) -> T,
    {
        let system = Self::into_system(self);
        let name = system.name();
        AdapterSystem::new(f, system, name)
    }

    /// Get the [`TypeId`] of the [`System`] produced after calling [`into_system`](`IntoSystem::into_system`).
    #[inline]
    fn system_type_id(&self) -> TypeId {
        TypeId::of::<Self::System>()
    }
}

// All systems implicitly implement IntoSystem.
impl<T: System> IntoSystem<T::In, T::Out, ()> for T {
    type System = T;
    fn into_system(this: Self) -> Self {
        this
    }
}

/// Wrapper type to mark a [`SystemParam`] as an input.
///
/// [`System`]s may take an optional input which they require to be passed to them when they
/// are being [`run`](System::run). For [`FunctionSystems`](FunctionSystem) the input may be marked
/// with this `In` type, but only the first param of a function may be tagged as an input. This also
/// means a system can only have one or zero input parameters.
///
/// # Examples
///
/// Here is a simple example of a system that takes a [`usize`] returning the square of it.
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// fn main() {
///     let mut square_system = IntoSystem::into_system(square);
///
///     let mut world = World::default();
///     square_system.initialize(&mut world);
///     assert_eq!(square_system.run(12, &mut world), 144);
/// }
///
/// fn square(In(input): In<usize>) -> usize {
///     input * input
/// }
/// ```
pub struct In<In>(pub In);

/// Ensure that a given function is a [system](System).
///
/// This should be used when writing doc examples,
/// to confirm that systems used in an example are
/// valid systems.
///
/// # Examples
///
/// The following example will panic when run since the
/// system's parameters mutably access the same component
/// multiple times.
///
/// ```should_panic
/// # use bevy_ecs::{prelude::*, system::assert_is_system};
/// #
/// # #[derive(Component)]
/// # struct Transform;
/// #
/// fn my_system(query1: Query<&mut Transform>, query2: Query<&mut Transform>) {
///     // ...
/// }
///
/// assert_is_system(my_system);
/// ```
pub fn assert_is_system<In: 'static, Out: 'static, Marker>(
    system: impl IntoSystem<In, Out, Marker>,
) {
    let mut system = IntoSystem::into_system(system);

    // Initialize the system, which will panic if the system has access conflicts.
    let mut world = World::new();
    system.initialize(&mut world);
}

/// Ensure that a given function is a [read-only system](ReadOnlySystem).
///
/// This should be used when writing doc examples,
/// to confirm that systems used in an example are
/// valid systems.
///
/// # Examples
///
/// The following example will fail to compile
/// since the system accesses a component mutably.
///
/// ```compile_fail
/// # use bevy_ecs::{prelude::*, system::assert_is_read_only_system};
/// #
/// # #[derive(Component)]
/// # struct Transform;
/// #
/// fn my_system(query: Query<&mut Transform>) {
///     // ...
/// }
///
/// assert_is_read_only_system(my_system);
/// ```
pub fn assert_is_read_only_system<In: 'static, Out: 'static, Marker, S>(system: S)
where
    S: IntoSystem<In, Out, Marker>,
    S::System: ReadOnlySystem,
{
    assert_is_system(system);
}

/// Ensures that the provided system doesn't with itself.
///
/// This function will  panic if the provided system conflict with itself.
///
/// Note: this will run the system on an empty world.
pub fn assert_system_does_not_conflict<Out, Params, S: IntoSystem<(), Out, Params>>(sys: S) {
    let mut world = World::new();
    let mut system = IntoSystem::into_system(sys);
    system.initialize(&mut world);
    system.run((), &mut world);
}

impl<T> std::ops::Deref for In<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for In<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use bevy_utils::default;

    use crate::{
        self as bevy_ecs,
        archetype::{ArchetypeComponentId, Archetypes},
        bundle::Bundles,
        change_detection::DetectChanges,
        component::{Component, Components, Tick},
        entity::{Entities, Entity},
        prelude::AnyOf,
        query::{Added, Changed, Or, With, Without},
        removal_detection::RemovedComponents,
        schedule::{
            apply_deferred, common_conditions::resource_exists, Condition, IntoSystemConfigs,
            Schedule,
        },
        system::{
            Commands, In, IntoSystem, Local, NonSend, NonSendMut, ParamSet, Query, Res, ResMut,
            Resource, StaticSystemParam, System, SystemState,
        },
        world::{FromWorld, World},
    };

    #[derive(Resource, PartialEq, Debug)]
    enum SystemRan {
        Yes,
        No,
    }

    #[derive(Component, Resource, Debug, Eq, PartialEq, Default)]
    struct A;
    #[derive(Component, Resource)]
    struct B;
    #[derive(Component, Resource)]
    struct C;
    #[derive(Component, Resource)]
    struct D;
    #[derive(Component, Resource)]
    struct E;
    #[derive(Component, Resource)]
    struct F;

    #[derive(Component, Debug)]
    struct W<T>(T);

    #[test]
    fn simple_system() {
        fn sys(query: Query<&A>) {
            for a in &query {
                println!("{a:?}");
            }
        }

        let mut system = IntoSystem::into_system(sys);
        let mut world = World::new();
        world.spawn(A);

        system.initialize(&mut world);
        system.run((), &mut world);
    }

    fn run_system<Marker, S: IntoSystem<(), (), Marker>>(world: &mut World, system: S) {
        let mut schedule = Schedule::default();
        schedule.add_systems(system);
        schedule.run(world);
    }

    #[test]
    fn get_many_is_ordered() {
        use crate::system::Resource;
        const ENTITIES_COUNT: usize = 1000;

        #[derive(Resource)]
        struct EntitiesArray(Vec<Entity>);

        fn query_system(
            mut ran: ResMut<SystemRan>,
            entities_array: Res<EntitiesArray>,
            q: Query<&W<usize>>,
        ) {
            let entities_array: [Entity; ENTITIES_COUNT] =
                entities_array.0.clone().try_into().unwrap();

            for (i, w) in (0..ENTITIES_COUNT).zip(q.get_many(entities_array).unwrap()) {
                assert_eq!(i, w.0);
            }

            *ran = SystemRan::Yes;
        }

        fn query_system_mut(
            mut ran: ResMut<SystemRan>,
            entities_array: Res<EntitiesArray>,
            mut q: Query<&mut W<usize>>,
        ) {
            let entities_array: [Entity; ENTITIES_COUNT] =
                entities_array.0.clone().try_into().unwrap();

            #[allow(unused_mut)]
            for (i, mut w) in (0..ENTITIES_COUNT).zip(q.get_many_mut(entities_array).unwrap()) {
                assert_eq!(i, w.0);
            }

            *ran = SystemRan::Yes;
        }

        let mut world = World::default();
        world.insert_resource(SystemRan::No);
        let entity_ids = (0..ENTITIES_COUNT)
            .map(|i| world.spawn(W(i)).id())
            .collect();
        world.insert_resource(EntitiesArray(entity_ids));

        run_system(&mut world, query_system);
        assert_eq!(*world.resource::<SystemRan>(), SystemRan::Yes);

        world.insert_resource(SystemRan::No);
        run_system(&mut world, query_system_mut);
        assert_eq!(*world.resource::<SystemRan>(), SystemRan::Yes);
    }

    #[test]
    fn or_param_set_system() {
        // Regression test for issue #762
        fn query_system(
            mut ran: ResMut<SystemRan>,
            mut set: ParamSet<(
                Query<(), Or<(Changed<A>, Changed<B>)>>,
                Query<(), Or<(Added<A>, Added<B>)>>,
            )>,
        ) {
            let changed = set.p0().iter().count();
            let added = set.p1().iter().count();

            assert_eq!(changed, 1);
            assert_eq!(added, 1);

            *ran = SystemRan::Yes;
        }

        let mut world = World::default();
        world.insert_resource(SystemRan::No);
        world.spawn((A, B));

        run_system(&mut world, query_system);

        assert_eq!(*world.resource::<SystemRan>(), SystemRan::Yes);
    }

    #[test]
    fn changed_resource_system() {
        use crate::system::Resource;

        #[derive(Resource)]
        struct Flipper(bool);

        #[derive(Resource)]
        struct Added(usize);

        #[derive(Resource)]
        struct Changed(usize);

        fn incr_e_on_flip(
            value: Res<Flipper>,
            mut changed: ResMut<Changed>,
            mut added: ResMut<Added>,
        ) {
            if value.is_added() {
                added.0 += 1;
            }

            if value.is_changed() {
                changed.0 += 1;
            }
        }

        let mut world = World::default();
        world.insert_resource(Flipper(false));
        world.insert_resource(Added(0));
        world.insert_resource(Changed(0));

        let mut schedule = Schedule::default();

        schedule.add_systems((incr_e_on_flip, apply_deferred, World::clear_trackers).chain());

        schedule.run(&mut world);
        assert_eq!(world.resource::<Added>().0, 1);
        assert_eq!(world.resource::<Changed>().0, 1);

        schedule.run(&mut world);
        assert_eq!(world.resource::<Added>().0, 1);
        assert_eq!(world.resource::<Changed>().0, 1);

        world.resource_mut::<Flipper>().0 = true;
        schedule.run(&mut world);
        assert_eq!(world.resource::<Added>().0, 1);
        assert_eq!(world.resource::<Changed>().0, 2);
    }

    #[test]
    #[should_panic = "error[B0001]"]
    fn option_has_no_filter_with() {
        fn sys(_: Query<(Option<&A>, &mut B)>, _: Query<&mut B, Without<A>>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn option_doesnt_remove_unrelated_filter_with() {
        fn sys(_: Query<(Option<&A>, &mut B, &A)>, _: Query<&mut B, Without<A>>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic = "error[B0001]"]
    fn any_of_has_no_filter_with() {
        fn sys(_: Query<(AnyOf<(&A, ())>, &mut B)>, _: Query<&mut B, Without<A>>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn any_of_has_filter_with_when_both_have_it() {
        fn sys(_: Query<(AnyOf<(&A, &A)>, &mut B)>, _: Query<&mut B, Without<A>>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn any_of_doesnt_remove_unrelated_filter_with() {
        fn sys(_: Query<(AnyOf<(&A, ())>, &mut B, &A)>, _: Query<&mut B, Without<A>>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn any_of_and_without() {
        fn sys(_: Query<(AnyOf<(&A, &B)>, &mut C)>, _: Query<&mut C, (Without<A>, Without<B>)>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic = "error[B0001]"]
    fn or_has_no_filter_with() {
        fn sys(_: Query<&mut B, Or<(With<A>, With<B>)>>, _: Query<&mut B, Without<A>>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn or_has_filter_with_when_both_have_it() {
        fn sys(_: Query<&mut B, Or<(With<A>, With<A>)>>, _: Query<&mut B, Without<A>>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn or_has_filter_with() {
        fn sys(
            _: Query<&mut C, Or<(With<A>, With<B>)>>,
            _: Query<&mut C, (Without<A>, Without<B>)>,
        ) {
        }
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn or_expanded_with_and_without_common() {
        fn sys(_: Query<&mut D, (With<A>, Or<(With<B>, With<C>)>)>, _: Query<&mut D, Without<A>>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn or_expanded_nested_with_and_without_common() {
        fn sys(
            _: Query<&mut E, (Or<((With<B>, With<C>), (With<C>, With<D>))>, With<A>)>,
            _: Query<&mut E, (Without<B>, Without<D>)>,
        ) {
        }
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic = "error[B0001]"]
    fn or_expanded_nested_with_and_disjoint_without() {
        fn sys(
            _: Query<&mut E, (Or<((With<B>, With<C>), (With<C>, With<D>))>, With<A>)>,
            _: Query<&mut E, Without<D>>,
        ) {
        }
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic = "error[B0001]"]
    fn or_expanded_nested_or_with_and_disjoint_without() {
        fn sys(
            _: Query<&mut D, Or<(Or<(With<A>, With<B>)>, Or<(With<A>, With<C>)>)>>,
            _: Query<&mut D, Without<A>>,
        ) {
        }
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn or_expanded_nested_with_and_common_nested_without() {
        fn sys(
            _: Query<&mut D, Or<((With<A>, With<B>), (With<B>, With<C>))>>,
            _: Query<&mut D, Or<(Without<D>, Without<B>)>>,
        ) {
        }
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn or_with_without_and_compatible_with_without() {
        fn sys(
            _: Query<&mut C, Or<(With<A>, Without<B>)>>,
            _: Query<&mut C, (With<B>, Without<A>)>,
        ) {
        }
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic = "error[B0001]"]
    fn with_and_disjoint_or_empty_without() {
        fn sys(_: Query<&mut B, With<A>>, _: Query<&mut B, Or<((), Without<A>)>>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic = "error[B0001]"]
    fn or_expanded_with_and_disjoint_nested_without() {
        fn sys(
            _: Query<&mut D, Or<(With<A>, With<B>)>>,
            _: Query<&mut D, Or<(Without<A>, Without<B>)>>,
        ) {
        }
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic = "error[B0001]"]
    fn or_expanded_nested_with_and_disjoint_nested_without() {
        fn sys(
            _: Query<&mut D, Or<((With<A>, With<B>), (With<B>, With<C>))>>,
            _: Query<&mut D, Or<(Without<A>, Without<B>)>>,
        ) {
        }
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn or_doesnt_remove_unrelated_filter_with() {
        fn sys(_: Query<&mut B, (Or<(With<A>, With<B>)>, With<A>)>, _: Query<&mut B, Without<A>>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_mut_system() {
        fn sys(_q1: Query<&mut A>, _q2: Query<&mut A>) {}

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn disjoint_query_mut_system() {
        fn sys(_q1: Query<&mut A, With<B>>, _q2: Query<&mut A, Without<B>>) {}

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn disjoint_query_mut_read_component_system() {
        fn sys(_q1: Query<(&mut A, &B)>, _q2: Query<&mut A, Without<B>>) {}

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_immut_system() {
        fn sys(_q1: Query<&A>, _q2: Query<&mut A>) {}

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    fn query_set_system() {
        fn sys(mut _set: ParamSet<(Query<&mut A>, Query<&A>)>) {}
        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_with_query_set_system() {
        fn sys(_query: Query<&mut A>, _set: ParamSet<(Query<&mut A>, Query<&B>)>) {}

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_query_sets_system() {
        fn sys(_set_1: ParamSet<(Query<&mut A>,)>, _set_2: ParamSet<(Query<&mut A>, Query<&B>)>) {}

        let mut world = World::default();
        run_system(&mut world, sys);
    }

    #[derive(Default, Resource)]
    struct BufferRes {
        _buffer: Vec<u8>,
    }

    fn test_for_conflicting_resources<Marker, S: IntoSystem<(), (), Marker>>(sys: S) {
        let mut world = World::default();
        world.insert_resource(BufferRes::default());
        world.insert_resource(A);
        world.insert_resource(B);
        run_system(&mut world, sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_system_resources() {
        fn sys(_: ResMut<BufferRes>, _: Res<BufferRes>) {}
        test_for_conflicting_resources(sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_system_resources_reverse_order() {
        fn sys(_: Res<BufferRes>, _: ResMut<BufferRes>) {}
        test_for_conflicting_resources(sys);
    }

    #[test]
    #[should_panic]
    fn conflicting_system_resources_multiple_mutable() {
        fn sys(_: ResMut<BufferRes>, _: ResMut<BufferRes>) {}
        test_for_conflicting_resources(sys);
    }

    #[test]
    fn nonconflicting_system_resources() {
        fn sys(_: Local<BufferRes>, _: ResMut<BufferRes>, _: Local<A>, _: ResMut<A>) {}
        test_for_conflicting_resources(sys);
    }

    #[test]
    fn local_system() {
        let mut world = World::default();
        world.insert_resource(ProtoFoo { value: 1 });
        world.insert_resource(SystemRan::No);

        struct Foo {
            value: u32,
        }

        #[derive(Resource)]
        struct ProtoFoo {
            value: u32,
        }

        impl FromWorld for Foo {
            fn from_world(world: &mut World) -> Self {
                Foo {
                    value: world.resource::<ProtoFoo>().value + 1,
                }
            }
        }

        fn sys(local: Local<Foo>, mut system_ran: ResMut<SystemRan>) {
            assert_eq!(local.value, 2);
            *system_ran = SystemRan::Yes;
        }

        run_system(&mut world, sys);

        // ensure the system actually ran
        assert_eq!(*world.resource::<SystemRan>(), SystemRan::Yes);
    }

    #[test]
    fn non_send_option_system() {
        let mut world = World::default();

        world.insert_resource(SystemRan::No);
        #[allow(dead_code)]
        struct NotSend1(std::rc::Rc<i32>);
        #[allow(dead_code)]
        struct NotSend2(std::rc::Rc<i32>);
        world.insert_non_send_resource(NotSend1(std::rc::Rc::new(0)));

        fn sys(
            op: Option<NonSend<NotSend1>>,
            mut _op2: Option<NonSendMut<NotSend2>>,
            mut system_ran: ResMut<SystemRan>,
        ) {
            op.expect("NonSend should exist");
            *system_ran = SystemRan::Yes;
        }

        run_system(&mut world, sys);
        // ensure the system actually ran
        assert_eq!(*world.resource::<SystemRan>(), SystemRan::Yes);
    }

    #[test]
    fn non_send_system() {
        let mut world = World::default();

        world.insert_resource(SystemRan::No);
        #[allow(dead_code)]
        struct NotSend1(std::rc::Rc<i32>);
        #[allow(dead_code)]
        struct NotSend2(std::rc::Rc<i32>);

        world.insert_non_send_resource(NotSend1(std::rc::Rc::new(1)));
        world.insert_non_send_resource(NotSend2(std::rc::Rc::new(2)));

        fn sys(
            _op: NonSend<NotSend1>,
            mut _op2: NonSendMut<NotSend2>,
            mut system_ran: ResMut<SystemRan>,
        ) {
            *system_ran = SystemRan::Yes;
        }

        run_system(&mut world, sys);
        assert_eq!(*world.resource::<SystemRan>(), SystemRan::Yes);
    }

    #[test]
    fn removal_tracking() {
        let mut world = World::new();

        let entity_to_despawn = world.spawn(W(1)).id();
        let entity_to_remove_w_from = world.spawn(W(2)).id();
        let spurious_entity = world.spawn_empty().id();

        // Track which entities we want to operate on
        #[derive(Resource)]
        struct Despawned(Entity);
        world.insert_resource(Despawned(entity_to_despawn));

        #[derive(Resource)]
        struct Removed(Entity);
        world.insert_resource(Removed(entity_to_remove_w_from));

        // Verify that all the systems actually ran
        #[derive(Default, Resource)]
        struct NSystems(usize);
        world.insert_resource(NSystems::default());

        // First, check that removal detection is triggered if and only if we despawn an entity with the correct component
        world.entity_mut(entity_to_despawn).despawn();
        world.entity_mut(spurious_entity).despawn();

        fn validate_despawn(
            mut removed_i32: RemovedComponents<W<i32>>,
            despawned: Res<Despawned>,
            mut n_systems: ResMut<NSystems>,
        ) {
            assert_eq!(
                removed_i32.read().collect::<Vec<_>>(),
                &[despawned.0],
                "despawning causes the correct entity to show up in the 'RemovedComponent' system parameter."
            );

            n_systems.0 += 1;
        }

        run_system(&mut world, validate_despawn);

        // Reset the trackers to clear the buffer of removed components
        // Ordinarily, this is done in a system added by MinimalPlugins
        world.clear_trackers();

        // Then, try removing a component
        world.spawn(W(3));
        world.spawn(W(4));
        world.entity_mut(entity_to_remove_w_from).remove::<W<i32>>();

        fn validate_remove(
            mut removed_i32: RemovedComponents<W<i32>>,
            despawned: Res<Despawned>,
            removed: Res<Removed>,
            mut n_systems: ResMut<NSystems>,
        ) {
            // The despawned entity from the previous frame was
            // double buffered so we now have it in this system as well.
            assert_eq!(
                removed_i32.read().collect::<Vec<_>>(),
                &[despawned.0, removed.0],
                "removing a component causes the correct entity to show up in the 'RemovedComponent' system parameter."
            );

            n_systems.0 += 1;
        }

        run_system(&mut world, validate_remove);

        // Verify that both systems actually ran
        assert_eq!(world.resource::<NSystems>().0, 2);
    }

    #[test]
    fn world_collections_system() {
        let mut world = World::default();
        world.insert_resource(SystemRan::No);
        world.spawn((W(42), W(true)));
        fn sys(
            archetypes: &Archetypes,
            components: &Components,
            entities: &Entities,
            bundles: &Bundles,
            query: Query<Entity, With<W<i32>>>,
            mut system_ran: ResMut<SystemRan>,
        ) {
            assert_eq!(query.iter().count(), 1, "entity exists");
            for entity in &query {
                let location = entities.get(entity).unwrap();
                let archetype = archetypes.get(location.archetype_id).unwrap();
                let archetype_components = archetype.components().collect::<Vec<_>>();
                let bundle_id = bundles
                    .get_id(TypeId::of::<(W<i32>, W<bool>)>())
                    .expect("Bundle used to spawn entity should exist");
                let bundle_info = bundles.get(bundle_id).unwrap();
                let mut bundle_components = bundle_info.components().to_vec();
                bundle_components.sort();
                for component_id in &bundle_components {
                    assert!(
                        components.get_info(*component_id).is_some(),
                        "every bundle component exists in Components"
                    );
                }
                assert_eq!(
                    bundle_components, archetype_components,
                    "entity's bundle components exactly match entity's archetype components"
                );
            }
            *system_ran = SystemRan::Yes;
        }

        run_system(&mut world, sys);

        // ensure the system actually ran
        assert_eq!(*world.resource::<SystemRan>(), SystemRan::Yes);
    }

    #[test]
    fn get_system_conflicts() {
        fn sys_x(_: Res<A>, _: Res<B>, _: Query<(&C, &D)>) {}

        fn sys_y(_: Res<A>, _: ResMut<B>, _: Query<(&C, &mut D)>) {}

        let mut world = World::default();
        let mut x = IntoSystem::into_system(sys_x);
        let mut y = IntoSystem::into_system(sys_y);
        x.initialize(&mut world);
        y.initialize(&mut world);

        let conflicts = x.component_access().get_conflicts(y.component_access());
        let b_id = world
            .components()
            .get_resource_id(TypeId::of::<B>())
            .unwrap();
        let d_id = world.components().get_id(TypeId::of::<D>()).unwrap();
        assert_eq!(conflicts, vec![b_id, d_id]);
    }

    #[test]
    fn query_is_empty() {
        fn without_filter(not_empty: Query<&A>, empty: Query<&B>) {
            assert!(!not_empty.is_empty());
            assert!(empty.is_empty());
        }

        fn with_filter(not_empty: Query<&A, With<C>>, empty: Query<&A, With<D>>) {
            assert!(!not_empty.is_empty());
            assert!(empty.is_empty());
        }

        let mut world = World::default();
        world.spawn(A).insert(C);

        let mut without_filter = IntoSystem::into_system(without_filter);
        without_filter.initialize(&mut world);
        without_filter.run((), &mut world);

        let mut with_filter = IntoSystem::into_system(with_filter);
        with_filter.initialize(&mut world);
        with_filter.run((), &mut world);
    }

    #[test]
    #[allow(clippy::too_many_arguments)]
    fn can_have_16_parameters() {
        fn sys_x(
            _: Res<A>,
            _: Res<B>,
            _: Res<C>,
            _: Res<D>,
            _: Res<E>,
            _: Res<F>,
            _: Query<&A>,
            _: Query<&B>,
            _: Query<&C>,
            _: Query<&D>,
            _: Query<&E>,
            _: Query<&F>,
            _: Query<(&A, &B)>,
            _: Query<(&C, &D)>,
            _: Query<(&E, &F)>,
        ) {
        }
        fn sys_y(
            _: (
                Res<A>,
                Res<B>,
                Res<C>,
                Res<D>,
                Res<E>,
                Res<F>,
                Query<&A>,
                Query<&B>,
                Query<&C>,
                Query<&D>,
                Query<&E>,
                Query<&F>,
                Query<(&A, &B)>,
                Query<(&C, &D)>,
                Query<(&E, &F)>,
            ),
        ) {
        }
        let mut world = World::default();
        let mut x = IntoSystem::into_system(sys_x);
        let mut y = IntoSystem::into_system(sys_y);
        x.initialize(&mut world);
        y.initialize(&mut world);
    }

    #[test]
    fn read_system_state() {
        #[derive(Eq, PartialEq, Debug, Resource)]
        struct A(usize);

        #[derive(Component, Eq, PartialEq, Debug)]
        struct B(usize);

        let mut world = World::default();
        world.insert_resource(A(42));
        world.spawn(B(7));

        let mut system_state: SystemState<(Res<A>, Query<&B>, ParamSet<(Query<&C>, Query<&D>)>)> =
            SystemState::new(&mut world);
        let (a, query, _) = system_state.get(&world);
        assert_eq!(*a, A(42), "returned resource matches initial value");
        assert_eq!(
            *query.single(),
            B(7),
            "returned component matches initial value"
        );
    }

    #[test]
    fn write_system_state() {
        #[derive(Resource, Eq, PartialEq, Debug)]
        struct A(usize);

        #[derive(Component, Eq, PartialEq, Debug)]
        struct B(usize);

        let mut world = World::default();
        world.insert_resource(A(42));
        world.spawn(B(7));

        let mut system_state: SystemState<(ResMut<A>, Query<&mut B>)> =
            SystemState::new(&mut world);

        // The following line shouldn't compile because the parameters used are not ReadOnlySystemParam
        // let (a, query) = system_state.get(&world);

        let (a, mut query) = system_state.get_mut(&mut world);
        assert_eq!(*a, A(42), "returned resource matches initial value");
        assert_eq!(
            *query.single_mut(),
            B(7),
            "returned component matches initial value"
        );
    }

    #[test]
    fn system_state_change_detection() {
        #[derive(Component, Eq, PartialEq, Debug)]
        struct A(usize);

        let mut world = World::default();
        let entity = world.spawn(A(1)).id();

        let mut system_state: SystemState<Query<&A, Changed<A>>> = SystemState::new(&mut world);
        {
            let query = system_state.get(&world);
            assert_eq!(*query.single(), A(1));
        }

        {
            let query = system_state.get(&world);
            assert!(query.get_single().is_err());
        }

        world.entity_mut(entity).get_mut::<A>().unwrap().0 = 2;
        {
            let query = system_state.get(&world);
            assert_eq!(*query.single(), A(2));
        }
    }

    #[test]
    #[should_panic]
    fn system_state_invalid_world() {
        let mut world = World::default();
        let mut system_state = SystemState::<Query<&A>>::new(&mut world);
        let mismatched_world = World::default();
        system_state.get(&mismatched_world);
    }

    #[test]
    fn system_state_archetype_update() {
        #[derive(Component, Eq, PartialEq, Debug)]
        struct A(usize);

        #[derive(Component, Eq, PartialEq, Debug)]
        struct B(usize);

        let mut world = World::default();
        world.spawn(A(1));

        let mut system_state = SystemState::<Query<&A>>::new(&mut world);
        {
            let query = system_state.get(&world);
            assert_eq!(
                query.iter().collect::<Vec<_>>(),
                vec![&A(1)],
                "exactly one component returned"
            );
        }

        world.spawn((A(2), B(2)));
        {
            let query = system_state.get(&world);
            assert_eq!(
                query.iter().collect::<Vec<_>>(),
                vec![&A(1), &A(2)],
                "components from both archetypes returned"
            );
        }
    }

    /// this test exists to show that read-only world-only queries can return data that lives as long as 'world
    #[test]
    #[allow(unused)]
    fn long_life_test() {
        struct Holder<'w> {
            value: &'w A,
        }

        struct State {
            state: SystemState<Res<'static, A>>,
            state_q: SystemState<Query<'static, 'static, &'static A>>,
        }

        impl State {
            fn hold_res<'w>(&mut self, world: &'w World) -> Holder<'w> {
                let a = self.state.get(world);
                Holder {
                    value: a.into_inner(),
                }
            }
            fn hold_component<'w>(&mut self, world: &'w World, entity: Entity) -> Holder<'w> {
                let q = self.state_q.get(world);
                let a = q.get_inner(entity).unwrap();
                Holder { value: a }
            }
            fn hold_components<'w>(&mut self, world: &'w World) -> Vec<Holder<'w>> {
                let mut components = Vec::new();
                let q = self.state_q.get(world);
                for a in q.iter_inner() {
                    components.push(Holder { value: a });
                }
                components
            }
        }
    }

    #[test]
    fn immutable_mut_test() {
        #[derive(Component, Eq, PartialEq, Debug, Clone, Copy)]
        struct A(usize);

        let mut world = World::default();
        world.spawn(A(1));
        world.spawn(A(2));

        let mut system_state = SystemState::<Query<&mut A>>::new(&mut world);
        {
            let mut query = system_state.get_mut(&mut world);
            assert_eq!(
                query.iter_mut().map(|m| *m).collect::<Vec<A>>(),
                vec![A(1), A(2)],
                "both components returned by iter_mut of &mut"
            );
            assert_eq!(
                query.iter().collect::<Vec<&A>>(),
                vec![&A(1), &A(2)],
                "both components returned by iter of &mut"
            );
        }
    }

    #[test]
    fn convert_mut_to_immut() {
        {
            let mut world = World::new();

            fn mutable_query(mut query: Query<&mut A>) {
                for _ in &mut query {}

                immutable_query(query.to_readonly());
            }

            fn immutable_query(_: Query<&A>) {}

            let mut sys = IntoSystem::into_system(mutable_query);
            sys.initialize(&mut world);
        }

        {
            let mut world = World::new();

            fn mutable_query(mut query: Query<Option<&mut A>>) {
                for _ in &mut query {}

                immutable_query(query.to_readonly());
            }

            fn immutable_query(_: Query<Option<&A>>) {}

            let mut sys = IntoSystem::into_system(mutable_query);
            sys.initialize(&mut world);
        }

        {
            let mut world = World::new();

            fn mutable_query(mut query: Query<(&mut A, &B)>) {
                for _ in &mut query {}

                immutable_query(query.to_readonly());
            }

            fn immutable_query(_: Query<(&A, &B)>) {}

            let mut sys = IntoSystem::into_system(mutable_query);
            sys.initialize(&mut world);
        }

        {
            let mut world = World::new();

            fn mutable_query(mut query: Query<(&mut A, &mut B)>) {
                for _ in &mut query {}

                immutable_query(query.to_readonly());
            }

            fn immutable_query(_: Query<(&A, &B)>) {}

            let mut sys = IntoSystem::into_system(mutable_query);
            sys.initialize(&mut world);
        }

        {
            let mut world = World::new();

            fn mutable_query(mut query: Query<(&mut A, &mut B), With<C>>) {
                for _ in &mut query {}

                immutable_query(query.to_readonly());
            }

            fn immutable_query(_: Query<(&A, &B), With<C>>) {}

            let mut sys = IntoSystem::into_system(mutable_query);
            sys.initialize(&mut world);
        }

        {
            let mut world = World::new();

            fn mutable_query(mut query: Query<(&mut A, &mut B), Without<C>>) {
                for _ in &mut query {}

                immutable_query(query.to_readonly());
            }

            fn immutable_query(_: Query<(&A, &B), Without<C>>) {}

            let mut sys = IntoSystem::into_system(mutable_query);
            sys.initialize(&mut world);
        }

        {
            let mut world = World::new();

            fn mutable_query(mut query: Query<(&mut A, &mut B), Added<C>>) {
                for _ in &mut query {}

                immutable_query(query.to_readonly());
            }

            fn immutable_query(_: Query<(&A, &B), Added<C>>) {}

            let mut sys = IntoSystem::into_system(mutable_query);
            sys.initialize(&mut world);
        }

        {
            let mut world = World::new();

            fn mutable_query(mut query: Query<(&mut A, &mut B), Changed<C>>) {
                for _ in &mut query {}

                immutable_query(query.to_readonly());
            }

            fn immutable_query(_: Query<(&A, &B), Changed<C>>) {}

            let mut sys = IntoSystem::into_system(mutable_query);
            sys.initialize(&mut world);
        }
    }

    #[test]
    fn update_archetype_component_access_works() {
        use std::collections::HashSet;

        fn a_not_b_system(_query: Query<&A, Without<B>>) {}

        let mut world = World::default();
        let mut system = IntoSystem::into_system(a_not_b_system);
        let mut expected_ids = HashSet::<ArchetypeComponentId>::new();
        let a_id = world.init_component::<A>();

        // set up system and verify its access is empty
        system.initialize(&mut world);
        system.update_archetype_component_access(world.as_unsafe_world_cell());
        assert_eq!(
            system
                .archetype_component_access()
                .reads()
                .collect::<HashSet<_>>(),
            expected_ids
        );

        // add some entities with archetypes that should match and save their ids
        expected_ids.insert(
            world
                .spawn(A)
                .archetype()
                .get_archetype_component_id(a_id)
                .unwrap(),
        );
        expected_ids.insert(
            world
                .spawn((A, C))
                .archetype()
                .get_archetype_component_id(a_id)
                .unwrap(),
        );

        // add some entities with archetypes that should not match
        world.spawn((A, B));
        world.spawn((B, C));

        // update system and verify its accesses are correct
        system.update_archetype_component_access(world.as_unsafe_world_cell());
        assert_eq!(
            system
                .archetype_component_access()
                .reads()
                .collect::<HashSet<_>>(),
            expected_ids
        );

        // one more round
        expected_ids.insert(
            world
                .spawn((A, D))
                .archetype()
                .get_archetype_component_id(a_id)
                .unwrap(),
        );
        world.spawn((A, B, D));
        system.update_archetype_component_access(world.as_unsafe_world_cell());
        assert_eq!(
            system
                .archetype_component_access()
                .reads()
                .collect::<HashSet<_>>(),
            expected_ids
        );
    }

    #[test]
    fn commands_param_set() {
        // Regression test for #4676
        let mut world = World::new();
        let entity = world.spawn_empty().id();

        run_system(
            &mut world,
            move |mut commands_set: ParamSet<(Commands, Commands)>| {
                commands_set.p0().entity(entity).insert(A);
                commands_set.p1().entity(entity).insert(B);
            },
        );

        let entity = world.entity(entity);
        assert!(entity.contains::<A>());
        assert!(entity.contains::<B>());
    }

    #[test]
    fn into_iter_impl() {
        let mut world = World::new();
        world.spawn(W(42u32));
        run_system(&mut world, |mut q: Query<&mut W<u32>>| {
            for mut a in &mut q {
                assert_eq!(a.0, 42);
                a.0 = 0;
            }
            for a in &q {
                assert_eq!(a.0, 0);
            }
        });
    }

    #[test]
    #[should_panic = "Encountered a mismatched World."]
    fn query_validates_world_id() {
        let mut world1 = World::new();
        let world2 = World::new();
        let qstate = world1.query::<()>();
        // SAFETY: doesnt access anything
        let query = unsafe {
            Query::new(
                world2.as_unsafe_world_cell_readonly(),
                &qstate,
                Tick::new(0),
                Tick::new(0),
            )
        };
        query.iter();
    }

    #[test]
    #[should_panic]
    fn assert_system_does_not_conflict() {
        fn system(_query: Query<(&mut W<u32>, &mut W<u32>)>) {}
        super::assert_system_does_not_conflict(system);
    }

    #[test]
    #[should_panic]
    fn panic_inside_system() {
        let mut world = World::new();
        run_system(&mut world, || panic!("this system panics"));
    }

    #[test]
    fn assert_systems() {
        use std::str::FromStr;

        use crate::{prelude::*, system::assert_is_system};

        /// Mocks a system that returns a value of type `T`.
        fn returning<T>() -> T {
            unimplemented!()
        }

        /// Mocks an exclusive system that takes an input and returns an output.
        fn exclusive_in_out<A, B>(_: In<A>, _: &mut World) -> B {
            unimplemented!()
        }

        fn static_system_param(_: StaticSystemParam<Query<'static, 'static, &W<u32>>>) {
            unimplemented!()
        }

        fn exclusive_with_state(
            _: &mut World,
            _: Local<bool>,
            _: (&mut QueryState<&W<i32>>, &mut SystemState<Query<&W<u32>>>),
            _: (),
        ) {
            unimplemented!()
        }

        fn not(In(val): In<bool>) -> bool {
            !val
        }

        assert_is_system(returning::<Result<u32, std::io::Error>>.map(Result::unwrap));
        assert_is_system(returning::<Option<()>>.map(drop));
        assert_is_system(returning::<&str>.map(u64::from_str).map(Result::unwrap));
        assert_is_system(static_system_param);
        assert_is_system(exclusive_in_out::<(), Result<(), std::io::Error>>.map(bevy_utils::error));
        assert_is_system(exclusive_with_state);
        assert_is_system(returning::<bool>.pipe(exclusive_in_out::<bool, ()>));

        returning::<()>.run_if(returning::<bool>.pipe(not));
    }

    #[test]
    fn pipe_change_detection() {
        #[derive(Resource, Default)]
        struct Flag;

        #[derive(Default)]
        struct Info {
            // If true, the respective system will mutate `Flag`.
            do_first: bool,
            do_second: bool,

            // Will be set to true if the respective system saw that `Flag` changed.
            first_flag: bool,
            second_flag: bool,
        }

        fn first(In(mut info): In<Info>, mut flag: ResMut<Flag>) -> Info {
            if flag.is_changed() {
                info.first_flag = true;
            }
            if info.do_first {
                *flag = Flag;
            }

            info
        }

        fn second(In(mut info): In<Info>, mut flag: ResMut<Flag>) -> Info {
            if flag.is_changed() {
                info.second_flag = true;
            }
            if info.do_second {
                *flag = Flag;
            }

            info
        }

        let mut world = World::new();
        world.init_resource::<Flag>();
        let mut sys = first.pipe(second);
        sys.initialize(&mut world);

        sys.run(default(), &mut world);

        // The second system should observe a change made in the first system.
        let info = sys.run(
            Info {
                do_first: true,
                ..default()
            },
            &mut world,
        );
        assert!(!info.first_flag);
        assert!(info.second_flag);

        // When a change is made in the second system, the first system
        // should observe it the next time they are run.
        let info1 = sys.run(
            Info {
                do_second: true,
                ..default()
            },
            &mut world,
        );
        let info2 = sys.run(default(), &mut world);
        assert!(!info1.first_flag);
        assert!(!info1.second_flag);
        assert!(info2.first_flag);
        assert!(!info2.second_flag);
    }

    #[test]
    fn test_combinator_clone() {
        let mut world = World::new();
        #[derive(Resource)]
        struct A;
        #[derive(Resource)]
        struct B;
        #[derive(Resource, PartialEq, Eq, Debug)]
        struct C(i32);

        world.insert_resource(A);
        world.insert_resource(C(0));
        let mut sched = Schedule::default();
        sched.add_systems(
            (
                |mut res: ResMut<C>| {
                    res.0 += 1;
                },
                |mut res: ResMut<C>| {
                    res.0 += 2;
                },
            )
                .distributive_run_if(resource_exists::<A>.or(resource_exists::<B>)),
        );
        sched.initialize(&mut world).unwrap();
        sched.run(&mut world);
        assert_eq!(world.get_resource(), Some(&C(3)));
    }
}
