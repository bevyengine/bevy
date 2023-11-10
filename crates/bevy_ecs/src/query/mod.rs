//! Contains APIs for retrieving component data from the world.

mod access;
mod error;
mod fetch;
mod filter;
mod iter;
mod par_iter;
mod state;

pub use access::*;
pub use error::*;
pub use fetch::*;
pub use filter::*;
pub use iter::*;
pub use par_iter::*;
pub use state::*;

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

#[cfg(not(debug_assertions))]
impl<T> DebugCheckedUnwrap for Option<T> {
    type Item = T;

    #[inline(always)]
    unsafe fn debug_checked_unwrap(self) -> Self::Item {
        if let Some(inner) = self {
            inner
        } else {
            std::hint::unreachable_unchecked()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ReadOnlyWorldQuery, WorldQuery};
    use crate::prelude::{AnyOf, Changed, Entity, Or, QueryState, With, Without};
    use crate::query::{ArchetypeFilter, Has, QueryCombinationIter};
    use crate::schedule::{IntoSystemConfigs, Schedule};
    use crate::system::{IntoSystem, Query, System, SystemState};
    use crate::{self as bevy_ecs, component::Component, world::World};
    use std::any::type_name;
    use std::collections::HashSet;

    #[derive(Component, Debug, Hash, Eq, PartialEq, Clone, Copy)]
    struct A(usize);
    #[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
    struct B(usize);
    #[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
    struct C(usize);
    #[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
    struct D(usize);

    #[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
    #[component(storage = "SparseSet")]
    struct Sparse(usize);

    #[test]
    fn query() {
        let mut world = World::new();
        world.spawn((A(1), B(1)));
        world.spawn(A(2));
        let values = world.query::<&A>().iter(&world).collect::<Vec<&A>>();
        assert_eq!(values, vec![&A(1), &A(2)]);

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
        fn assert_combination<Q, F, const K: usize>(world: &mut World, expected_size: usize)
        where
            Q: ReadOnlyWorldQuery,
            F: ReadOnlyWorldQuery,
            F::ReadOnly: ArchetypeFilter,
        {
            let mut query = world.query_filtered::<Q, F>();
            let query_type = type_name::<QueryCombinationIter<Q, F, K>>();
            let iter = query.iter_combinations::<K>(world);
            assert_all_sizes_iterator_equal(iter, expected_size, 0, query_type);
            let iter = query.iter_combinations::<K>(world);
            assert_all_sizes_iterator_equal(iter, expected_size, 1, query_type);
            let iter = query.iter_combinations::<K>(world);
            assert_all_sizes_iterator_equal(iter, expected_size, 5, query_type);
        }
        fn assert_all_sizes_equal<Q, F>(world: &mut World, expected_size: usize)
        where
            Q: ReadOnlyWorldQuery,
            F: ReadOnlyWorldQuery,
            F::ReadOnly: ArchetypeFilter,
        {
            let mut query = world.query_filtered::<Q, F>();
            let query_type = type_name::<QueryState<Q, F>>();
            assert_all_exact_sizes_iterator_equal(query.iter(world), expected_size, 0, query_type);
            assert_all_exact_sizes_iterator_equal(query.iter(world), expected_size, 1, query_type);
            assert_all_exact_sizes_iterator_equal(query.iter(world), expected_size, 5, query_type);

            let expected = expected_size;
            assert_combination::<Q, F, 0>(world, choose(expected, 0));
            assert_combination::<Q, F, 1>(world, choose(expected, 1));
            assert_combination::<Q, F, 2>(world, choose(expected, 2));
            assert_combination::<Q, F, 5>(world, choose(expected, 5));
            assert_combination::<Q, F, 43>(world, choose(expected, 43));
            assert_combination::<Q, F, 64>(world, choose(expected, 64));
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

    #[test]
    fn query_iter_combinations() {
        let mut world = World::new();

        world.spawn((A(1), B(1)));
        world.spawn(A(2));
        world.spawn(A(3));
        world.spawn(A(4));

        let values: Vec<[&A; 2]> = world.query::<&A>().iter_combinations(&world).collect();
        assert_eq!(
            values,
            vec![
                [&A(1), &A(2)],
                [&A(1), &A(3)],
                [&A(1), &A(4)],
                [&A(2), &A(3)],
                [&A(2), &A(4)],
                [&A(3), &A(4)],
            ]
        );
        let mut a_query = world.query::<&A>();
        let values: Vec<[&A; 3]> = a_query.iter_combinations(&world).collect();
        assert_eq!(
            values,
            vec![
                [&A(1), &A(2), &A(3)],
                [&A(1), &A(2), &A(4)],
                [&A(1), &A(3), &A(4)],
                [&A(2), &A(3), &A(4)],
            ]
        );

        let mut query = world.query::<&mut A>();
        let mut combinations = query.iter_combinations_mut(&mut world);
        while let Some([mut a, mut b, mut c]) = combinations.fetch_next() {
            a.0 += 10;
            b.0 += 100;
            c.0 += 1000;
        }

        let values: Vec<[&A; 3]> = a_query.iter_combinations(&world).collect();
        assert_eq!(
            values,
            vec![
                [&A(31), &A(212), &A(1203)],
                [&A(31), &A(212), &A(3004)],
                [&A(31), &A(1203), &A(3004)],
                [&A(212), &A(1203), &A(3004)]
            ]
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
        use bevy_ecs::query::{Added, Changed, Or, With, Without};

        let mut world = World::new();

        world.spawn((A(1), B(1)));
        world.spawn(A(2));
        world.spawn(A(3));
        world.spawn(A(4));

        let mut a_wout_b = world.query_filtered::<&A, Without<B>>();
        let values: HashSet<[&A; 2]> = a_wout_b.iter_combinations(&world).collect();
        assert_eq!(
            values,
            [[&A(2), &A(3)], [&A(2), &A(4)], [&A(3), &A(4)]]
                .into_iter()
                .collect::<HashSet<_>>()
        );

        let values: HashSet<[&A; 3]> = a_wout_b.iter_combinations(&world).collect();
        assert_eq!(
            values,
            [[&A(2), &A(3), &A(4)],].into_iter().collect::<HashSet<_>>()
        );

        let mut query = world.query_filtered::<&A, Or<(With<A>, With<B>)>>();
        let values: HashSet<[&A; 2]> = query.iter_combinations(&world).collect();
        assert_eq!(
            values,
            [
                [&A(1), &A(2)],
                [&A(1), &A(3)],
                [&A(1), &A(4)],
                [&A(2), &A(3)],
                [&A(2), &A(4)],
                [&A(3), &A(4)],
            ]
            .into_iter()
            .collect::<HashSet<_>>()
        );

        let mut query = world.query_filtered::<&mut A, Without<B>>();
        let mut combinations = query.iter_combinations_mut(&mut world);
        while let Some([mut a, mut b, mut c]) = combinations.fetch_next() {
            a.0 += 10;
            b.0 += 100;
            c.0 += 1000;
        }

        let values: HashSet<[&A; 3]> = a_wout_b.iter_combinations(&world).collect();
        assert_eq!(
            values,
            [[&A(12), &A(103), &A(1004)],]
                .into_iter()
                .collect::<HashSet<_>>()
        );

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

        world.clear_trackers();

        let mut query_changed = world.query_filtered::<&A, Changed<A>>();

        let mut query = world.query_filtered::<&mut A, With<B>>();
        let mut combinations = query.iter_combinations_mut(&mut world);
        while let Some([mut a, mut b, mut c]) = combinations.fetch_next() {
            a.0 += 10;
            b.0 += 100;
            c.0 += 1000;
        }

        let values: HashSet<[&A; 3]> = query_changed.iter_combinations(&world).collect();
        assert_eq!(
            values,
            [
                [&A(31), &A(212), &A(1203)],
                [&A(31), &A(212), &A(3004)],
                [&A(31), &A(1203), &A(3004)],
                [&A(212), &A(1203), &A(3004)]
            ]
            .into_iter()
            .collect::<HashSet<_>>()
        );
    }

    #[test]
    fn query_iter_combinations_sparse() {
        let mut world = World::new();

        world.spawn_batch((1..=4).map(Sparse));

        let mut query = world.query::<&mut Sparse>();
        let mut combinations = query.iter_combinations_mut(&mut world);
        while let Some([mut a, mut b, mut c]) = combinations.fetch_next() {
            a.0 += 10;
            b.0 += 100;
            c.0 += 1000;
        }

        let mut query = world.query::<&Sparse>();
        let values: Vec<[&Sparse; 3]> = query.iter_combinations(&world).collect();
        assert_eq!(
            values,
            vec![
                [&Sparse(31), &Sparse(212), &Sparse(1203)],
                [&Sparse(31), &Sparse(212), &Sparse(3004)],
                [&Sparse(31), &Sparse(1203), &Sparse(3004)],
                [&Sparse(212), &Sparse(1203), &Sparse(3004)]
            ]
        );
    }

    #[test]
    fn multi_storage_query() {
        let mut world = World::new();

        world.spawn((Sparse(1), B(2)));
        world.spawn(Sparse(2));

        let values = world
            .query::<&Sparse>()
            .iter(&world)
            .collect::<Vec<&Sparse>>();
        assert_eq!(values, vec![&Sparse(1), &Sparse(2)]);

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

        let values: Vec<(&A, bool)> = world.query::<(&A, Has<B>)>().iter(&world).collect();

        // The query seems to put the components with B first
        assert_eq!(
            values,
            vec![(&A(1), true), (&A(3), true), (&A(2), false), (&A(4), false),]
        );
    }

    #[test]
    #[should_panic = "&mut bevy_ecs::query::tests::A conflicts with a previous access in this query."]
    fn self_conflicting_worldquery() {
        #[derive(WorldQuery)]
        #[world_query(mutable)]
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
            #[derive(WorldQuery)]
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
            #[derive(WorldQuery)]
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
            #[derive(WorldQuery)]
            struct MaybeBSparse {
                blah: Option<(&'static B, &'static Sparse)>,
            }
            #[derive(WorldQuery)]
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
            #[derive(WorldQuery)]
            struct AOrBFilter {
                a: Or<(With<A>, With<B>)>,
            }
            #[derive(WorldQuery)]
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
            #[derive(WorldQuery)]
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
            #[derive(WorldQuery)]
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
            #[derive(WorldQuery)]
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
            system.run((), &mut world);
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
            system.run((), &mut world);
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
            system.run((), &mut world);
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
        q.for_each(&world, |_: &Foo| ());

        let _: Option<&Foo> = q.get(&world, e).ok();
        let _: Option<&Foo> = q.get_manual(&world, e).ok();
        let _: Option<[&Foo; 1]> = q.get_many(&world, [e]).ok();
        let _: Option<&Foo> = q.get_single(&world).ok();
        let _: &Foo = q.single(&world);

        // system param
        let mut q = SystemState::<Query<&mut Foo>>::new(&mut world);
        let q = q.get_mut(&mut world);
        let _: Option<&Foo> = q.iter().next();
        let _: Option<[&Foo; 2]> = q.iter_combinations::<2>().next();
        let _: Option<&Foo> = q.iter_many([e]).next();
        q.for_each(|_: &Foo| ());

        let _: Option<&Foo> = q.get(e).ok();
        let _: Option<&Foo> = q.get_component(e).ok();
        let _: Option<[&Foo; 1]> = q.get_many([e]).ok();
        let _: Option<&Foo> = q.get_single().ok();
        let _: [&Foo; 1] = q.many([e]);
        let _: &Foo = q.single();
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
}
