//! Tools for controlling behavior in an ECS application.
//!
//! Systems define how an ECS based application behaves.
//! Systems are added to a [`Schedule`](crate::schedule::Schedule), which is then run.
//! A system is usually written as a normal function, which is automatically converted into a system.
//!
//! System functions can have parameters, through which one can query and mutate Bevy ECS state.
//! Only types that implement [`SystemParam`] can be used, automatically fetching data from
//! the [`World`](crate::world::World).
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
//! - by adding them to a [`SystemSet`], and then using `.configure_set(ThisSet.before(ThatSet))` syntax to configure many systems at once
//! - through the use of `.add_systems((system_a, system_b, system_c).chain())`
//!
//! [`SystemSet`]: crate::schedule::SystemSet
//!
//! ## Example
//!
//! ```
//! # use bevy_ecs::prelude::*;
//! # let mut app = Schedule::new();
//! // Prints "Hello, World!" each frame.
//! app
//!     .add_system(print_first.before(print_mid))
//!     .add_system(print_mid)
//!     .add_system(print_last.after(print_mid));
//! # let mut world = World::new();
//! # app.run(&mut world);
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
//! - [`&World`](crate::world::World)
//! - [`RemovedComponents`](crate::removal_detection::RemovedComponents)
//! - [`SystemName`]
//! - [`SystemChangeTick`]
//! - [`Archetypes`](crate::archetype::Archetypes) (Provides Archetype metadata)
//! - [`Bundles`](crate::bundle::Bundles) (Provides Bundles metadata)
//! - [`Components`](crate::component::Components) (Provides Components metadata)
//! - [`Entities`](crate::entity::Entities) (Provides Entities metadata)
//! - All tuples between 1 to 16 elements where each element implements [`SystemParam`]
//! - [`()` (unit primitive type)](https://doc.rust-lang.org/stable/std/primitive.unit.html)

mod commands;
mod exclusive_function_system;
mod exclusive_system_param;
mod function_system;
mod query;
#[allow(clippy::module_inception)]
mod system;
mod system_param;
mod system_piping;

pub use commands::*;
pub use exclusive_function_system::*;
pub use exclusive_system_param::*;
pub use function_system::*;
pub use query::*;
pub use system::*;
pub use system_param::*;
pub use system_piping::*;

/// Ensure that a given function is a [system](System).
///
/// This should be used when writing doc examples,
/// to confirm that systems used in an example are
/// valid systems.
pub fn assert_is_system<In, Out, Params, S: IntoSystem<In, Out, Params>>(sys: S) {
    if false {
        // Check it can be converted into a system
        // TODO: This should ensure that the system has no conflicting system params
        IntoSystem::into_system(sys);
    }
}

/// Ensure that a given function is a [read-only system](ReadOnlySystem).
///
/// This should be used when writing doc examples,
/// to confirm that systems used in an example are
/// valid systems.
pub fn assert_is_read_only_system<In, Out, Params, S: IntoSystem<In, Out, Params>>(sys: S)
where
    S::System: ReadOnlySystem,
{
    if false {
        // Check it can be converted into a system
        // TODO: This should ensure that the system has no conflicting system params
        IntoSystem::into_system(sys);
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use crate::{
        self as bevy_ecs,
        archetype::{ArchetypeComponentId, Archetypes},
        bundle::Bundles,
        change_detection::DetectChanges,
        component::{Component, Components},
        entity::{Entities, Entity},
        prelude::AnyOf,
        query::{Added, Changed, Or, With, Without},
        removal_detection::RemovedComponents,
        schedule::{apply_system_buffers, IntoSystemConfig, Schedule},
        system::{
            Commands, IntoSystem, Local, NonSend, NonSendMut, ParamSet, Query, QueryComponentError,
            Res, ResMut, Resource, System, SystemState,
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

    fn run_system<Param, S: IntoSystem<(), (), Param>>(world: &mut World, system: S) {
        let mut schedule = Schedule::default();
        schedule.add_system(system);
        schedule.run(world);
    }

    #[test]
    fn query_system_gets() {
        fn query_system(
            mut ran: ResMut<SystemRan>,
            entity_query: Query<Entity, With<A>>,
            b_query: Query<&B>,
            a_c_query: Query<(&A, &C)>,
            d_query: Query<&D>,
        ) {
            let entities = entity_query.iter().collect::<Vec<Entity>>();
            assert!(
                b_query.get_component::<B>(entities[0]).is_err(),
                "entity 0 should not have B"
            );
            assert!(
                b_query.get_component::<B>(entities[1]).is_ok(),
                "entity 1 should have B"
            );
            assert!(
                b_query.get_component::<A>(entities[1]).is_err(),
                "entity 1 should have A, but b_query shouldn't have access to it"
            );
            assert!(
                b_query.get_component::<D>(entities[3]).is_err(),
                "entity 3 should have D, but it shouldn't be accessible from b_query"
            );
            assert!(
                b_query.get_component::<C>(entities[2]).is_err(),
                "entity 2 has C, but it shouldn't be accessible from b_query"
            );
            assert!(
                a_c_query.get_component::<C>(entities[2]).is_ok(),
                "entity 2 has C, and it should be accessible from a_c_query"
            );
            assert!(
                a_c_query.get_component::<D>(entities[3]).is_err(),
                "entity 3 should have D, but it shouldn't be accessible from b_query"
            );
            assert!(
                d_query.get_component::<D>(entities[3]).is_ok(),
                "entity 3 should have D"
            );

            *ran = SystemRan::Yes;
        }

        let mut world = World::default();
        world.insert_resource(SystemRan::No);
        world.spawn(A);
        world.spawn((A, B));
        world.spawn((A, C));
        world.spawn((A, D));

        run_system(&mut world, query_system);

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

        schedule.add_system(incr_e_on_flip);
        schedule.add_system(apply_system_buffers.after(incr_e_on_flip));
        schedule.add_system(World::clear_trackers.after(apply_system_buffers));

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

    fn test_for_conflicting_resources<Param, S: IntoSystem<(), (), Param>>(sys: S) {
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
        struct NotSend1(std::rc::Rc<i32>);
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
        struct NotSend1(std::rc::Rc<i32>);
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
                removed_i32.iter().collect::<Vec<_>>(),
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
                removed_i32.iter().collect::<Vec<_>>(),
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
                    .get_id(std::any::TypeId::of::<(W<i32>, W<bool>)>())
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
        system.update_archetype_component_access(&world);
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
        system.update_archetype_component_access(&world);
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
        system.update_archetype_component_access(&world);
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
    fn readonly_query_get_mut_component_fails() {
        let mut world = World::new();
        let entity = world.spawn(W(42u32)).id();
        run_system(&mut world, move |q: Query<&mut W<u32>>| {
            let mut rq = q.to_readonly();
            assert_eq!(
                QueryComponentError::MissingWriteAccess,
                rq.get_component_mut::<W<u32>>(entity).unwrap_err(),
            );
        });
    }

    #[test]
    #[should_panic = "Attempted to use bevy_ecs::query::state::QueryState<()> with a mismatched World."]
    fn query_validates_world_id() {
        let mut world1 = World::new();
        let world2 = World::new();
        let qstate = world1.query::<()>();
        // SAFETY: doesnt access anything
        let query = unsafe { Query::new(&world2, &qstate, 0, 0, false) };
        query.iter();
    }

    #[test]
    #[should_panic]
    fn panic_inside_system() {
        let mut world = World::new();
        run_system(&mut world, || panic!("this system panics"));
    }
}
