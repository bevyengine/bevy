mod access;
mod fetch;
mod filter;
mod iter;
mod state;

pub use access::*;
pub use fetch::*;
pub use filter::*;
pub use iter::*;
pub use state::*;

#[allow(unreachable_code)]
pub(crate) unsafe fn debug_checked_unreachable() -> ! {
    #[cfg(debug_assertions)]
    unreachable!();
    std::hint::unreachable_unchecked();
}

#[cfg(test)]
mod tests {
    use super::WorldQuery;
    use crate::prelude::{AnyOf, Entity, Or, With, Without};
    use crate::system::{IntoSystem, Query, System};
    use crate::{self as bevy_ecs, component::Component, world::World};
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
        world.spawn().insert_bundle((A(1), B(1)));
        world.spawn().insert_bundle((A(2),));
        let values = world.query::<&A>().iter(&world).collect::<Vec<&A>>();
        assert_eq!(values, vec![&A(1), &A(2)]);

        for (_a, mut b) in world.query::<(&A, &mut B)>().iter_mut(&mut world) {
            b.0 = 3;
        }
        let values = world.query::<&B>().iter(&world).collect::<Vec<&B>>();
        assert_eq!(values, vec![&B(3)]);
    }

    #[test]
    fn query_filtered_len() {
        let mut world = World::new();
        world.spawn().insert_bundle((A(1), B(1)));
        world.spawn().insert_bundle((A(2),));
        world.spawn().insert_bundle((A(3),));

        let mut values = world.query_filtered::<&A, With<B>>();
        let n = 1;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, Without<B>>();
        let n = 2;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);

        let mut world = World::new();
        world.spawn().insert_bundle((A(1), B(1), C(1)));
        world.spawn().insert_bundle((A(2), B(2)));
        world.spawn().insert_bundle((A(3), B(3)));
        world.spawn().insert_bundle((A(4), C(4)));
        world.spawn().insert_bundle((A(5), C(5)));
        world.spawn().insert_bundle((A(6), C(6)));
        world.spawn().insert_bundle((A(7),));
        world.spawn().insert_bundle((A(8),));
        world.spawn().insert_bundle((A(9),));
        world.spawn().insert_bundle((A(10),));

        // With/Without for B and C
        let mut values = world.query_filtered::<&A, With<B>>();
        let n = 3;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, With<C>>();
        let n = 4;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, Without<B>>();
        let n = 7;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, Without<C>>();
        let n = 6;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);

        // With/Without (And) combinations
        let mut values = world.query_filtered::<&A, (With<B>, With<C>)>();
        let n = 1;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, (With<B>, Without<C>)>();
        let n = 2;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, (Without<B>, With<C>)>();
        let n = 3;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, (Without<B>, Without<C>)>();
        let n = 4;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);

        // With/Without Or<()> combinations
        let mut values = world.query_filtered::<&A, Or<(With<B>, With<C>)>>();
        let n = 6;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, Or<(With<B>, Without<C>)>>();
        let n = 7;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, Or<(Without<B>, With<C>)>>();
        let n = 8;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, Or<(Without<B>, Without<C>)>>();
        let n = 9;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);

        let mut values = world.query_filtered::<&A, (Or<(With<B>,)>, Or<(With<C>,)>)>();
        let n = 1;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, Or<(Or<(With<B>, With<C>)>, With<D>)>>();
        let n = 6;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);

        world.spawn().insert_bundle((A(11), D(11)));

        let mut values = world.query_filtered::<&A, Or<(Or<(With<B>, With<C>)>, With<D>)>>();
        let n = 7;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
        let mut values = world.query_filtered::<&A, Or<(Or<(With<B>, With<C>)>, Without<D>)>>();
        let n = 10;
        assert_eq!(values.iter(&world).size_hint().0, n);
        assert_eq!(values.iter(&world).size_hint().1.unwrap(), n);
        assert_eq!(values.iter(&world).len(), n);
        assert_eq!(values.iter(&world).count(), n);
    }

    #[test]
    fn query_iter_combinations() {
        let mut world = World::new();

        world.spawn().insert_bundle((A(1), B(1)));
        world.spawn().insert_bundle((A(2),));
        world.spawn().insert_bundle((A(3),));
        world.spawn().insert_bundle((A(4),));

        let mut a_query = world.query::<&A>();
        let w = &world;
        assert_eq!(a_query.iter_combinations::<0>(w).count(), 0);
        assert_eq!(a_query.iter_combinations::<0>(w).size_hint().1, Some(0));
        assert_eq!(a_query.iter_combinations::<1>(w).count(), 4);
        assert_eq!(a_query.iter_combinations::<1>(w).size_hint().1, Some(4));
        assert_eq!(a_query.iter_combinations::<2>(w).count(), 6);
        assert_eq!(a_query.iter_combinations::<2>(w).size_hint().1, Some(6));
        assert_eq!(a_query.iter_combinations::<3>(w).count(), 4);
        assert_eq!(a_query.iter_combinations::<3>(w).size_hint().1, Some(4));
        assert_eq!(a_query.iter_combinations::<4>(w).count(), 1);
        assert_eq!(a_query.iter_combinations::<4>(w).size_hint().1, Some(1));
        assert_eq!(a_query.iter_combinations::<5>(w).count(), 0);
        assert_eq!(a_query.iter_combinations::<5>(w).size_hint().1, Some(0));
        assert_eq!(a_query.iter_combinations::<128>(w).count(), 0);
        assert_eq!(a_query.iter_combinations::<128>(w).size_hint().1, Some(0));

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
        let size = a_query.iter_combinations::<3>(&world).size_hint();
        assert_eq!(size.1, Some(4));
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

        world.spawn().insert_bundle((A(1), B(1)));
        world.spawn().insert_bundle((A(2),));
        world.spawn().insert_bundle((A(3),));
        world.spawn().insert_bundle((A(4),));

        let mut a_with_b = world.query_filtered::<&A, With<B>>();
        let w = &world;
        assert_eq!(a_with_b.iter_combinations::<0>(w).count(), 0);
        assert_eq!(a_with_b.iter_combinations::<0>(w).size_hint().1, Some(0));
        assert_eq!(a_with_b.iter_combinations::<1>(w).count(), 1);
        assert_eq!(a_with_b.iter_combinations::<1>(w).size_hint().1, Some(1));
        assert_eq!(a_with_b.iter_combinations::<2>(w).count(), 0);
        assert_eq!(a_with_b.iter_combinations::<2>(w).size_hint().1, Some(0));
        assert_eq!(a_with_b.iter_combinations::<3>(w).count(), 0);
        assert_eq!(a_with_b.iter_combinations::<3>(w).size_hint().1, Some(0));
        assert_eq!(a_with_b.iter_combinations::<4>(w).count(), 0);
        assert_eq!(a_with_b.iter_combinations::<4>(w).size_hint().1, Some(0));
        assert_eq!(a_with_b.iter_combinations::<5>(w).count(), 0);
        assert_eq!(a_with_b.iter_combinations::<5>(w).size_hint().1, Some(0));
        assert_eq!(a_with_b.iter_combinations::<128>(w).count(), 0);
        assert_eq!(a_with_b.iter_combinations::<128>(w).size_hint().1, Some(0));

        let mut a_wout_b = world.query_filtered::<&A, Without<B>>();
        let w = &world;
        assert_eq!(a_wout_b.iter_combinations::<0>(w).count(), 0);
        assert_eq!(a_wout_b.iter_combinations::<0>(w).size_hint().1, Some(0));
        assert_eq!(a_wout_b.iter_combinations::<1>(w).count(), 3);
        assert_eq!(a_wout_b.iter_combinations::<1>(w).size_hint().1, Some(3));
        assert_eq!(a_wout_b.iter_combinations::<2>(w).count(), 3);
        assert_eq!(a_wout_b.iter_combinations::<2>(w).size_hint().1, Some(3));
        assert_eq!(a_wout_b.iter_combinations::<3>(w).count(), 1);
        assert_eq!(a_wout_b.iter_combinations::<3>(w).size_hint().1, Some(1));
        assert_eq!(a_wout_b.iter_combinations::<4>(w).count(), 0);
        assert_eq!(a_wout_b.iter_combinations::<4>(w).size_hint().1, Some(0));
        assert_eq!(a_wout_b.iter_combinations::<5>(w).count(), 0);
        assert_eq!(a_wout_b.iter_combinations::<5>(w).size_hint().1, Some(0));
        assert_eq!(a_wout_b.iter_combinations::<128>(w).count(), 0);
        assert_eq!(a_wout_b.iter_combinations::<128>(w).size_hint().1, Some(0));

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

        world.spawn().insert_bundle((A(1), B(1)));
        world.spawn().insert_bundle((A(2), B(2)));
        world.spawn().insert_bundle((A(3), B(3)));
        world.spawn().insert_bundle((A(4), B(4)));

        let mut query_added = world.query_filtered::<&A, Added<A>>();

        world.clear_trackers();
        world.spawn().insert_bundle((A(5),));

        assert_eq!(query_added.iter_combinations::<2>(&world).count(), 0);

        world.clear_trackers();
        world.spawn().insert_bundle((A(6),));
        world.spawn().insert_bundle((A(7),));

        assert_eq!(query_added.iter_combinations::<2>(&world).count(), 1);

        world.clear_trackers();
        world.spawn().insert_bundle((A(8),));
        world.spawn().insert_bundle((A(9),));
        world.spawn().insert_bundle((A(10),));

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

        world.spawn_batch((1..=4).map(|i| (Sparse(i),)));

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

        world.spawn().insert_bundle((Sparse(1), B(2)));
        world.spawn().insert_bundle((Sparse(2),));

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

        world.spawn().insert_bundle((A(1), B(2)));
        world.spawn().insert_bundle((A(2),));
        world.spawn().insert_bundle((C(3),));

        let values: Vec<(Option<&A>, Option<&B>)> =
            world.query::<AnyOf<(&A, &B)>>().iter(&world).collect();

        assert_eq!(
            values,
            vec![(Some(&A(1)), Some(&B(2))), (Some(&A(2)), None),]
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

        world.spawn().insert_bundle((A(10), B(18), C(3), Sparse(4)));

        world.spawn().insert_bundle((A(101), B(148), C(13)));
        world.spawn().insert_bundle((A(51), B(46), Sparse(72)));
        world.spawn().insert_bundle((A(398), C(6), Sparse(9)));
        world.spawn().insert_bundle((B(11), C(28), Sparse(92)));

        world.spawn().insert_bundle((C(18348), Sparse(101)));
        world.spawn().insert_bundle((B(839), Sparse(5)));
        world.spawn().insert_bundle((B(6721), C(122)));
        world.spawn().insert_bundle((A(220), Sparse(63)));
        world.spawn().insert_bundle((A(1092), C(382)));
        world.spawn().insert_bundle((A(2058), B(3019)));

        world.spawn().insert_bundle((B(38), C(8), Sparse(100)));
        world.spawn().insert_bundle((A(111), C(52), Sparse(1)));
        world.spawn().insert_bundle((A(599), B(39), Sparse(13)));
        world.spawn().insert_bundle((A(55), B(66), C(77)));

        world.spawn();

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
    }

    #[test]
    fn many_entities() {
        let mut world = World::new();
        world.spawn().insert_bundle((A(0), B(0)));
        world.spawn().insert_bundle((A(0), B(0)));
        world.spawn().insert(A(0));
        world.spawn().insert(B(0));
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
                b_query.many_for_each_mut(&has_a, |mut b| {
                    b.0 = 1;
                });
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
}
