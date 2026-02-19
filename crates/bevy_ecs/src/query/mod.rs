//! Contains APIs for retrieving component data from the world.

mod access;
mod builder;
mod error;
mod fetch;
mod filter;
mod iter;
mod par_iter;
mod state;
mod world_query;

pub use access::*;
pub use bevy_ecs_macros::{QueryData, QueryFilter};
pub use builder::*;
pub use error::*;
pub use fetch::*;
pub use filter::*;
pub use iter::*;
pub use par_iter::*;
pub use state::*;
pub use world_query::*;

/// A debug checked version of [`Option::unwrap_unchecked`]. Will panic in
/// debug modes if unwrapping a `None` or `Err` value in debug mode, but is
/// equivalent to `Option::unwrap_unchecked` or `Result::unwrap_unchecked`
/// in release mode.
pub(crate) trait DebugCheckedUnwrap {
    type Item;
    /// # Panics
    /// Panics if the value is `None` or `Err`, only in debug mode.
    ///
    /// # Safety
    /// This must never be called on a `None` or `Err` value. This can
    /// only be called on `Some` or `Ok` values.
    unsafe fn debug_checked_unwrap(self) -> Self::Item;
}

// These two impls are explicitly split to ensure that the unreachable! macro
// does not cause inlining to fail when compiling in release mode.
#[cfg(debug_assertions)]
impl<T> DebugCheckedUnwrap for Option<T> {
    type Item = T;

    #[inline(always)]
    #[track_caller]
    unsafe fn debug_checked_unwrap(self) -> Self::Item {
        if let Some(inner) = self {
            inner
        } else {
            unreachable!()
        }
    }
}

// These two impls are explicitly split to ensure that the unreachable! macro
// does not cause inlining to fail when compiling in release mode.
#[cfg(debug_assertions)]
impl<T, U> DebugCheckedUnwrap for Result<T, U> {
    type Item = T;

    #[inline(always)]
    #[track_caller]
    unsafe fn debug_checked_unwrap(self) -> Self::Item {
        if let Ok(inner) = self {
            inner
        } else {
            unreachable!()
        }
    }
}

// These two impls are explicitly split to ensure that the unreachable! macro
// does not cause inlining to fail when compiling in release mode.
#[cfg(not(debug_assertions))]
impl<T, U> DebugCheckedUnwrap for Result<T, U> {
    type Item = T;

    #[inline(always)]
    #[track_caller]
    unsafe fn debug_checked_unwrap(self) -> Self::Item {
        if let Ok(inner) = self {
            inner
        } else {
            core::hint::unreachable_unchecked()
        }
    }
}

#[cfg(not(debug_assertions))]
impl<T> DebugCheckedUnwrap for Option<T> {
    type Item = T;

    #[inline(always)]
    unsafe fn debug_checked_unwrap(self) -> Self::Item {
        if let Some(inner) = self {
            inner
        } else {
            core::hint::unreachable_unchecked()
        }
    }
}

#[cfg(test)]
#[expect(clippy::print_stdout, reason = "Allowed in tests.")]
mod tests {
    use crate::{
        archetype::Archetype,
        component::{Component, ComponentId, Components, Tick},
        prelude::{AnyOf, Changed, Entity, Or, QueryState, Resource, With, Without},
        query::{
            ArchetypeFilter, FilteredAccess, Has, QueryCombinationIter, QueryData,
            ReadOnlyQueryData, WorldQuery,
        },
        schedule::{IntoScheduleConfigs, Schedule},
        storage::{Table, TableRow},
        system::{assert_is_system, IntoSystem, Query, System, SystemState},
        world::{unsafe_world_cell::UnsafeWorldCell, World},
    };
    use alloc::{vec, vec::Vec};
    use bevy_ecs_macros::QueryFilter;
    use core::{any::type_name, fmt::Debug, hash::Hash};
    use std::{collections::HashSet, println};

    #[derive(Component, Debug, Hash, Eq, PartialEq, Clone, Copy, PartialOrd, Ord)]
    struct A(usize);
    #[derive(Component, Debug, Hash, Eq, PartialEq, Clone, Copy)]
    struct B(usize);
    #[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
    struct C(usize);
    #[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
    struct D(usize);

    #[derive(Component, Debug, Hash, Eq, PartialEq, Clone, Copy, PartialOrd, Ord)]
    #[component(storage = "SparseSet")]
    struct Sparse(usize);

    #[test]
    fn query() {
        let mut world = World::new();
        world.spawn((A(1), B(1)));
        world.spawn(A(2));
        let values = world.query::<&A>().iter(&world).collect::<HashSet<&A>>();
        assert!(values.contains(&A(1)));
        assert!(values.contains(&A(2)));

        for (_a, mut b) in world.query::<(&A, &mut B)>().iter_mut(&mut world) {
            b.0 = 3;
        }
        let values = world.query::<&B>().iter(&world).collect::<Vec<&B>>();
        assert_eq!(values, vec![&B(3)]);
    }

    #[test]
    fn query_filtered_exactsizeiterator_len() {
        fn choose(n: usize, k: usize) -> usize {
            if n == 0 || k == 0 || n < k {
                return 0;
            }
            let ks = 1..=k;
            let ns = (n - k + 1..=n).rev();
            ks.zip(ns).fold(1, |acc, (k, n)| acc * n / k)
        }
        fn assert_combination<D, F, const K: usize>(world: &mut World, expected_size: usize)
        where
            D: ReadOnlyQueryData,
            F: ArchetypeFilter,
        {
            let mut query = world.query_filtered::<D, F>();
            let query_type = type_name::<QueryCombinationIter<D, F, K>>();
            let iter = query.iter_combinations::<K>(world);
            assert_all_sizes_iterator_equal(iter, expected_size, 0, query_type);
            let iter = query.iter_combinations::<K>(world);
            assert_all_sizes_iterator_equal(iter, expected_size, 1, query_type);
            let iter = query.iter_combinations::<K>(world);
            assert_all_sizes_iterator_equal(iter, expected_size, 5, query_type);
        }
        fn assert_all_sizes_equal<D, F>(world: &mut World, expected_size: usize)
        where
            D: ReadOnlyQueryData,
            F: ArchetypeFilter,
        {
            let mut query = world.query_filtered::<D, F>();
            let query_type = type_name::<QueryState<D, F>>();
            assert_all_exact_sizes_iterator_equal(query.iter(world), expected_size, 0, query_type);
            assert_all_exact_sizes_iterator_equal(query.iter(world), expected_size, 1, query_type);
            assert_all_exact_sizes_iterator_equal(query.iter(world), expected_size, 5, query_type);

            let expected = expected_size;
            assert_combination::<D, F, 1>(world, choose(expected, 1));
            assert_combination::<D, F, 2>(world, choose(expected, 2));
            assert_combination::<D, F, 5>(world, choose(expected, 5));
            assert_combination::<D, F, 43>(world, choose(expected, 43));
            assert_combination::<D, F, 64>(world, choose(expected, 64));
        }
        fn assert_all_exact_sizes_iterator_equal(
            iterator: impl ExactSizeIterator,
            expected_size: usize,
            skip: usize,
            query_type: &'static str,
        ) {
            let len = iterator.len();
            println!("len:           {len}");
            assert_all_sizes_iterator_equal(iterator, expected_size, skip, query_type);
            assert_eq!(len, expected_size);
        }
        fn assert_all_sizes_iterator_equal(
            mut iterator: impl Iterator,
            expected_size: usize,
            skip: usize,
            query_type: &'static str,
        ) {
            let expected_size = expected_size.saturating_sub(skip);
            for _ in 0..skip {
                iterator.next();
            }
            let size_hint_0 = iterator.size_hint().0;
            let size_hint_1 = iterator.size_hint().1;
            // `count` tests that not only it is the expected value, but also
            // the value is accurate to what the query returns.
            let count = iterator.count();
            // This will show up when one of the asserts in this function fails
            println!(
                "query declared sizes: \n\
                for query:     {query_type} \n\
                expected:      {expected_size} \n\
                size_hint().0: {size_hint_0} \n\
                size_hint().1: {size_hint_1:?} \n\
                count():       {count}"
            );
            assert_eq!(size_hint_0, expected_size);
            assert_eq!(size_hint_1, Some(expected_size));
            assert_eq!(count, expected_size);
        }

        let mut world = World::new();
        world.spawn((A(1), B(1)));
        world.spawn(A(2));
        world.spawn(A(3));

        assert_all_sizes_equal::<&A, With<B>>(&mut world, 1);
        assert_all_sizes_equal::<&A, Without<B>>(&mut world, 2);

        let mut world = World::new();
        world.spawn((A(1), B(1), C(1)));
        world.spawn((A(2), B(2)));
        world.spawn((A(3), B(3)));
        world.spawn((A(4), C(4)));
        world.spawn((A(5), C(5)));
        world.spawn((A(6), C(6)));
        world.spawn(A(7));
        world.spawn(A(8));
        world.spawn(A(9));
        world.spawn(A(10));

        // With/Without for B and C
        assert_all_sizes_equal::<&A, With<B>>(&mut world, 3);
        assert_all_sizes_equal::<&A, With<C>>(&mut world, 4);
        assert_all_sizes_equal::<&A, Without<B>>(&mut world, 7);
        assert_all_sizes_equal::<&A, Without<C>>(&mut world, 6);

        // With/Without (And) combinations
        assert_all_sizes_equal::<&A, (With<B>, With<C>)>(&mut world, 1);
        assert_all_sizes_equal::<&A, (With<B>, Without<C>)>(&mut world, 2);
        assert_all_sizes_equal::<&A, (Without<B>, With<C>)>(&mut world, 3);
        assert_all_sizes_equal::<&A, (Without<B>, Without<C>)>(&mut world, 4);

        // With/Without Or<()> combinations
        assert_all_sizes_equal::<&A, Or<(With<B>, With<C>)>>(&mut world, 6);
        assert_all_sizes_equal::<&A, Or<(With<B>, Without<C>)>>(&mut world, 7);
        assert_all_sizes_equal::<&A, Or<(Without<B>, With<C>)>>(&mut world, 8);
        assert_all_sizes_equal::<&A, Or<(Without<B>, Without<C>)>>(&mut world, 9);
        assert_all_sizes_equal::<&A, (Or<(With<B>,)>, Or<(With<C>,)>)>(&mut world, 1);
        assert_all_sizes_equal::<&A, Or<(Or<(With<B>, With<C>)>, With<D>)>>(&mut world, 6);

        for i in 11..14 {
            world.spawn((A(i), D(i)));
        }

        assert_all_sizes_equal::<&A, Or<(Or<(With<B>, With<C>)>, With<D>)>>(&mut world, 9);
        assert_all_sizes_equal::<&A, Or<(Or<(With<B>, With<C>)>, Without<D>)>>(&mut world, 10);

        // a fair amount of entities
        for i in 14..20 {
            world.spawn((C(i), D(i)));
        }
        assert_all_sizes_equal::<Entity, (With<C>, With<D>)>(&mut world, 6);
    }

    // the order of the combinations is not guaranteed, but each unique combination is present
    fn check_combinations<T: Ord + Hash + Debug, const K: usize>(
        values: HashSet<[&T; K]>,
        expected: HashSet<[&T; K]>,
    ) {
        values.iter().for_each(|pair| {
            let mut sorted = *pair;
            sorted.sort();
            assert!(expected.contains(&sorted),
                    "the results of iter_combinations should contain this combination {:?}. Expected: {:?}, got: {:?}",
                    &sorted, &expected, &values);
        });
    }

    #[test]
    fn query_iter_combinations() {
        let mut world = World::new();

        world.spawn((A(1), B(1)));
        world.spawn(A(2));
        world.spawn(A(3));
        world.spawn(A(4));

        let values: HashSet<[&A; 2]> = world.query::<&A>().iter_combinations(&world).collect();
        check_combinations(
            values,
            HashSet::from([
                [&A(1), &A(2)],
                [&A(1), &A(3)],
                [&A(1), &A(4)],
                [&A(2), &A(3)],
                [&A(2), &A(4)],
                [&A(3), &A(4)],
            ]),
        );
        let mut a_query = world.query::<&A>();

        let values: HashSet<[&A; 3]> = a_query.iter_combinations(&world).collect();
        check_combinations(
            values,
            HashSet::from([
                [&A(1), &A(2), &A(3)],
                [&A(1), &A(2), &A(4)],
                [&A(1), &A(3), &A(4)],
                [&A(2), &A(3), &A(4)],
            ]),
        );

        let mut b_query = world.query::<&B>();
        assert_eq!(
            b_query.iter_combinations::<2>(&world).size_hint(),
            (0, Some(0))
        );
        let values: Vec<[&B; 2]> = b_query.iter_combinations(&world).collect();
        assert_eq!(values, Vec::<[&B; 2]>::new());
    }

    #[test]
    fn query_filtered_iter_combinations() {
        use bevy_ecs::query::{Added, Or, With, Without};

        let mut world = World::new();

        world.spawn((A(1), B(1)));
        world.spawn(A(2));
        world.spawn(A(3));
        world.spawn(A(4));

        let mut a_wout_b = world.query_filtered::<&A, Without<B>>();
        let values: HashSet<[&A; 2]> = a_wout_b.iter_combinations(&world).collect();
        check_combinations(
            values,
            HashSet::from([[&A(2), &A(3)], [&A(2), &A(4)], [&A(3), &A(4)]]),
        );

        let values: HashSet<[&A; 3]> = a_wout_b.iter_combinations(&world).collect();
        check_combinations(values, HashSet::from([[&A(2), &A(3), &A(4)]]));

        let mut query = world.query_filtered::<&A, Or<(With<A>, With<B>)>>();
        let values: HashSet<[&A; 2]> = query.iter_combinations(&world).collect();
        check_combinations(
            values,
            HashSet::from([
                [&A(1), &A(2)],
                [&A(1), &A(3)],
                [&A(1), &A(4)],
                [&A(2), &A(3)],
                [&A(2), &A(4)],
                [&A(3), &A(4)],
            ]),
        );

        let mut query = world.query_filtered::<&mut A, Without<B>>();
        let mut combinations = query.iter_combinations_mut(&mut world);
        while let Some([mut a, mut b, mut c]) = combinations.fetch_next() {
            a.0 += 10;
            b.0 += 100;
            c.0 += 1000;
        }

        let values: HashSet<[&A; 3]> = a_wout_b.iter_combinations(&world).collect();
        check_combinations(values, HashSet::from([[&A(12), &A(103), &A(1004)]]));

        // Check if Added<T>, Changed<T> works
        let mut world = World::new();

        world.spawn((A(1), B(1)));
        world.spawn((A(2), B(2)));
        world.spawn((A(3), B(3)));
        world.spawn((A(4), B(4)));

        let mut query_added = world.query_filtered::<&A, Added<A>>();

        world.clear_trackers();
        world.spawn(A(5));

        assert_eq!(query_added.iter_combinations::<2>(&world).count(), 0);

        world.clear_trackers();
        world.spawn(A(6));
        world.spawn(A(7));

        assert_eq!(query_added.iter_combinations::<2>(&world).count(), 1);

        world.clear_trackers();
        world.spawn(A(8));
        world.spawn(A(9));
        world.spawn(A(10));

        assert_eq!(query_added.iter_combinations::<2>(&world).count(), 3);
    }

    #[test]
    fn query_iter_combinations_sparse() {
        let mut world = World::new();

        world.spawn_batch((1..=4).map(Sparse));

        let values: HashSet<[&Sparse; 3]> =
            world.query::<&Sparse>().iter_combinations(&world).collect();
        check_combinations(
            values,
            HashSet::from([
                [&Sparse(1), &Sparse(2), &Sparse(3)],
                [&Sparse(1), &Sparse(2), &Sparse(4)],
                [&Sparse(1), &Sparse(3), &Sparse(4)],
                [&Sparse(2), &Sparse(3), &Sparse(4)],
            ]),
        );
    }

    #[test]
    fn get_many_only_mut_checks_duplicates() {
        let mut world = World::new();
        let id = world.spawn(A(10)).id();
        let mut query_state = world.query::<&mut A>();
        let mut query = query_state.query_mut(&mut world);
        let result = query.get_many([id, id]);
        assert_eq!(result, Ok([&A(10), &A(10)]));
        let mut_result = query.get_many_mut([id, id]);
        assert!(mut_result.is_err());
    }

    #[test]
    fn multi_storage_query() {
        let mut world = World::new();

        world.spawn((Sparse(1), B(2)));
        world.spawn(Sparse(2));

        let values = world
            .query::<&Sparse>()
            .iter(&world)
            .collect::<HashSet<&Sparse>>();
        assert!(values.contains(&Sparse(1)));
        assert!(values.contains(&Sparse(2)));

        for (_a, mut b) in world.query::<(&Sparse, &mut B)>().iter_mut(&mut world) {
            b.0 = 3;
        }

        let values = world.query::<&B>().iter(&world).collect::<Vec<&B>>();
        assert_eq!(values, vec![&B(3)]);
    }

    #[test]
    fn any_query() {
        let mut world = World::new();

        world.spawn((A(1), B(2)));
        world.spawn(A(2));
        world.spawn(C(3));

        let values: Vec<(Option<&A>, Option<&B>)> =
            world.query::<AnyOf<(&A, &B)>>().iter(&world).collect();

        assert_eq!(
            values,
            vec![(Some(&A(1)), Some(&B(2))), (Some(&A(2)), None),]
        );
    }

    #[test]
    fn has_query() {
        let mut world = World::new();

        world.spawn((A(1), B(1)));
        world.spawn(A(2));
        world.spawn((A(3), B(1)));
        world.spawn(A(4));

        let values: HashSet<(&A, bool)> = world.query::<(&A, Has<B>)>().iter(&world).collect();

        assert!(values.contains(&(&A(1), true)));
        assert!(values.contains(&(&A(2), false)));
        assert!(values.contains(&(&A(3), true)));
        assert!(values.contains(&(&A(4), false)));
    }

    #[test]
    #[should_panic]
    fn self_conflicting_worldquery() {
        #[derive(QueryData)]
        #[query_data(mutable)]
        struct SelfConflicting {
            a: &'static mut A,
            b: &'static mut A,
        }

        let mut world = World::new();
        world.query::<SelfConflicting>();
    }

    #[test]
    fn derived_worldqueries() {
        let mut world = World::new();

        world.spawn((A(10), B(18), C(3), Sparse(4)));

        world.spawn((A(101), B(148), C(13)));
        world.spawn((A(51), B(46), Sparse(72)));
        world.spawn((A(398), C(6), Sparse(9)));
        world.spawn((B(11), C(28), Sparse(92)));

        world.spawn((C(18348), Sparse(101)));
        world.spawn((B(839), Sparse(5)));
        world.spawn((B(6721), C(122)));
        world.spawn((A(220), Sparse(63)));
        world.spawn((A(1092), C(382)));
        world.spawn((A(2058), B(3019)));

        world.spawn((B(38), C(8), Sparse(100)));
        world.spawn((A(111), C(52), Sparse(1)));
        world.spawn((A(599), B(39), Sparse(13)));
        world.spawn((A(55), B(66), C(77)));

        world.spawn_empty();

        {
            #[derive(QueryData)]
            struct CustomAB {
                a: &'static A,
                b: &'static B,
            }

            let custom_param_data = world
                .query::<CustomAB>()
                .iter(&world)
                .map(|item| (*item.a, *item.b))
                .collect::<Vec<_>>();
            let normal_data = world
                .query::<(&A, &B)>()
                .iter(&world)
                .map(|(a, b)| (*a, *b))
                .collect::<Vec<_>>();
            assert_eq!(custom_param_data, normal_data);
        }

        {
            #[derive(QueryData)]
            struct FancyParam {
                e: Entity,
                b: &'static B,
                opt: Option<&'static Sparse>,
            }

            let custom_param_data = world
                .query::<FancyParam>()
                .iter(&world)
                .map(|fancy| (fancy.e, *fancy.b, fancy.opt.copied()))
                .collect::<Vec<_>>();
            let normal_data = world
                .query::<(Entity, &B, Option<&Sparse>)>()
                .iter(&world)
                .map(|(e, b, opt)| (e, *b, opt.copied()))
                .collect::<Vec<_>>();
            assert_eq!(custom_param_data, normal_data);
        }

        {
            #[derive(QueryData)]
            struct MaybeBSparse {
                blah: Option<(&'static B, &'static Sparse)>,
            }
            #[derive(QueryData)]
            struct MatchEverything {
                abcs: AnyOf<(&'static A, &'static B, &'static C)>,
                opt_bsparse: MaybeBSparse,
            }

            let custom_param_data = world
                .query::<MatchEverything>()
                .iter(&world)
                .map(
                    |MatchEverythingItem {
                         abcs: (a, b, c),
                         opt_bsparse: MaybeBSparseItem { blah: bsparse },
                     }| {
                        (
                            (a.copied(), b.copied(), c.copied()),
                            bsparse.map(|(b, sparse)| (*b, *sparse)),
                        )
                    },
                )
                .collect::<Vec<_>>();
            let normal_data = world
                .query::<(AnyOf<(&A, &B, &C)>, Option<(&B, &Sparse)>)>()
                .iter(&world)
                .map(|((a, b, c), bsparse)| {
                    (
                        (a.copied(), b.copied(), c.copied()),
                        bsparse.map(|(b, sparse)| (*b, *sparse)),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(custom_param_data, normal_data);
        }

        {
            #[derive(QueryFilter)]
            struct AOrBFilter {
                a: Or<(With<A>, With<B>)>,
            }
            #[derive(QueryFilter)]
            struct NoSparseThatsSlow {
                no: Without<Sparse>,
            }

            let custom_param_entities = world
                .query_filtered::<Entity, (AOrBFilter, NoSparseThatsSlow)>()
                .iter(&world)
                .collect::<Vec<_>>();
            let normal_entities = world
                .query_filtered::<Entity, (Or<(With<A>, With<B>)>, Without<Sparse>)>()
                .iter(&world)
                .collect::<Vec<_>>();
            assert_eq!(custom_param_entities, normal_entities);
        }

        {
            #[derive(QueryFilter)]
            struct CSparseFilter {
                tuple_structs_pls: With<C>,
                ugh: With<Sparse>,
            }

            let custom_param_entities = world
                .query_filtered::<Entity, CSparseFilter>()
                .iter(&world)
                .collect::<Vec<_>>();
            let normal_entities = world
                .query_filtered::<Entity, (With<C>, With<Sparse>)>()
                .iter(&world)
                .collect::<Vec<_>>();
            assert_eq!(custom_param_entities, normal_entities);
        }

        {
            #[derive(QueryFilter)]
            struct WithoutComps {
                _1: Without<A>,
                _2: Without<B>,
                _3: Without<C>,
            }

            let custom_param_entities = world
                .query_filtered::<Entity, WithoutComps>()
                .iter(&world)
                .collect::<Vec<_>>();
            let normal_entities = world
                .query_filtered::<Entity, (Without<A>, Without<B>, Without<C>)>()
                .iter(&world)
                .collect::<Vec<_>>();
            assert_eq!(custom_param_entities, normal_entities);
        }

        {
            #[derive(QueryData)]
            struct IterCombAB {
                a: &'static A,
                b: &'static B,
            }

            let custom_param_data = world
                .query::<IterCombAB>()
                .iter_combinations::<2>(&world)
                .map(|[item0, item1]| [(*item0.a, *item0.b), (*item1.a, *item1.b)])
                .collect::<Vec<_>>();
            let normal_data = world
                .query::<(&A, &B)>()
                .iter_combinations(&world)
                .map(|[(a0, b0), (a1, b1)]| [(*a0, *b0), (*a1, *b1)])
                .collect::<Vec<_>>();
            assert_eq!(custom_param_data, normal_data);
        }
    }

    #[test]
    fn many_entities() {
        let mut world = World::new();
        world.spawn((A(0), B(0)));
        world.spawn((A(0), B(0)));
        world.spawn(A(0));
        world.spawn(B(0));
        {
            fn system(has_a: Query<Entity, With<A>>, has_a_and_b: Query<(&A, &B)>) {
                assert_eq!(has_a_and_b.iter_many(&has_a).count(), 2);
            }
            let mut system = IntoSystem::into_system(system);
            system.initialize(&mut world);
            system.run((), &mut world).unwrap();
        }
        {
            fn system(has_a: Query<Entity, With<A>>, mut b_query: Query<&mut B>) {
                let mut iter = b_query.iter_many_mut(&has_a);
                while let Some(mut b) = iter.fetch_next() {
                    b.0 = 1;
                }
            }
            let mut system = IntoSystem::into_system(system);
            system.initialize(&mut world);
            system.run((), &mut world).unwrap();
        }
        {
            fn system(query: Query<(Option<&A>, &B)>) {
                for (maybe_a, b) in &query {
                    match maybe_a {
                        Some(_) => assert_eq!(b.0, 1),
                        None => assert_eq!(b.0, 0),
                    }
                }
            }
            let mut system = IntoSystem::into_system(system);
            system.initialize(&mut world);
            system.run((), &mut world).unwrap();
        }
    }

    #[test]
    fn mut_to_immut_query_methods_have_immut_item() {
        #[derive(Component)]
        struct Foo;

        let mut world = World::new();
        let e = world.spawn(Foo).id();

        // state
        let mut q = world.query::<&mut Foo>();
        let _: Option<&Foo> = q.iter(&world).next();
        let _: Option<[&Foo; 2]> = q.iter_combinations::<2>(&world).next();
        let _: Option<&Foo> = q.iter_manual(&world).next();
        let _: Option<&Foo> = q.iter_many(&world, [e]).next();
        q.iter(&world).for_each(|_: &Foo| ());

        let _: Option<&Foo> = q.get(&world, e).ok();
        let _: Option<&Foo> = q.get_manual(&world, e).ok();
        let _: Option<[&Foo; 1]> = q.get_many(&world, [e]).ok();
        let _: Option<&Foo> = q.single(&world).ok();
        let _: &Foo = q.single(&world).unwrap();

        // system param
        let mut q = SystemState::<Query<&mut Foo>>::new(&mut world);
        let q = q.get_mut(&mut world);
        let _: Option<&Foo> = q.iter().next();
        let _: Option<[&Foo; 2]> = q.iter_combinations::<2>().next();
        let _: Option<&Foo> = q.iter_many([e]).next();
        q.iter().for_each(|_: &Foo| ());

        let _: Option<&Foo> = q.get(e).ok();
        let _: Option<[&Foo; 1]> = q.get_many([e]).ok();
        let _: Option<&Foo> = q.single().ok();
        let _: &Foo = q.single().unwrap();
    }

    // regression test for https://github.com/bevyengine/bevy/pull/8029
    #[test]
    fn par_iter_mut_change_detection() {
        let mut world = World::new();
        world.spawn((A(1), B(1)));

        fn propagate_system(mut query: Query<(&A, &mut B), Changed<A>>) {
            query.par_iter_mut().for_each(|(a, mut b)| {
                b.0 = a.0;
            });
        }

        fn modify_system(mut query: Query<&mut A>) {
            for mut a in &mut query {
                a.0 = 2;
            }
        }

        let mut schedule = Schedule::default();
        schedule.add_systems((propagate_system, modify_system).chain());
        schedule.run(&mut world);
        world.clear_trackers();
        schedule.run(&mut world);
        world.clear_trackers();

        let values = world.query::<&B>().iter(&world).collect::<Vec<&B>>();
        assert_eq!(values, vec![&B(2)]);
    }

    #[derive(Resource)]
    struct R;

    /// `QueryData` that performs read access on R to test that resource access is tracked
    struct ReadsRData;

    /// SAFETY:
    /// `update_component_access` adds resource read access for `R`.
    unsafe impl WorldQuery for ReadsRData {
        type Fetch<'w> = ();
        type State = ComponentId;

        fn shrink_fetch<'wlong: 'wshort, 'wshort>(_: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {}

        unsafe fn init_fetch<'w, 's>(
            _world: UnsafeWorldCell<'w>,
            _state: &'s Self::State,
            _last_run: Tick,
            _this_run: Tick,
        ) -> Self::Fetch<'w> {
        }

        const IS_DENSE: bool = true;

        #[inline]
        unsafe fn set_archetype<'w, 's>(
            _fetch: &mut Self::Fetch<'w>,
            _state: &'s Self::State,
            _archetype: &'w Archetype,
            _table: &Table,
        ) {
        }

        #[inline]
        unsafe fn set_table<'w, 's>(
            _fetch: &mut Self::Fetch<'w>,
            _state: &'s Self::State,
            _table: &'w Table,
        ) {
        }

        fn update_component_access(&component_id: &Self::State, access: &mut FilteredAccess) {
            assert!(
                !access.access().has_resource_write(component_id),
                "ReadsRData conflicts with a previous access in this query. Shared access cannot coincide with exclusive access."
            );
            access.add_resource_read(component_id);
        }

        fn init_state(world: &mut World) -> Self::State {
            world.components_registrator().register_resource::<R>()
        }

        fn get_state(components: &Components) -> Option<Self::State> {
            components.resource_id::<R>()
        }

        fn matches_component_set(
            _state: &Self::State,
            _set_contains_id: &impl Fn(ComponentId) -> bool,
        ) -> bool {
            true
        }
    }

    /// SAFETY: `Self` is the same as `Self::ReadOnly`
    unsafe impl QueryData for ReadsRData {
        const IS_READ_ONLY: bool = true;
        type ReadOnly = Self;
        type Item<'w, 's> = ();

        fn shrink<'wlong: 'wshort, 'wshort, 's>(
            _item: Self::Item<'wlong, 's>,
        ) -> Self::Item<'wshort, 's> {
        }

        #[inline(always)]
        unsafe fn fetch<'w, 's>(
            _state: &'s Self::State,
            _fetch: &mut Self::Fetch<'w>,
            _entity: Entity,
            _table_row: TableRow,
        ) -> Self::Item<'w, 's> {
        }
    }

    /// SAFETY: access is read only
    unsafe impl ReadOnlyQueryData for ReadsRData {}

    #[test]
    fn read_res_read_res_no_conflict() {
        fn system(_q1: Query<ReadsRData, With<A>>, _q2: Query<ReadsRData, Without<A>>) {}
        assert_is_system(system);
    }
}
