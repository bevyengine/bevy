mod builder;
mod iter;
mod query;
mod state;
mod terms;

pub use builder::*;
pub use iter::*;
pub use query::*;
pub use state::*;
pub use terms::*;

#[cfg(test)]
mod tests {
    use bevy_ptr::Ptr;

    use crate as bevy_ecs;
    use crate::prelude::*;
    use crate::term_query::{QueryTerm, QueryTermGroup, TermQuery, TermQueryState};

    use super::QueryBuilder;

    #[derive(Component, PartialEq, Debug)]
    struct A(usize);

    #[derive(Component, PartialEq, Debug)]
    struct B(usize);

    #[derive(Component, PartialEq, Debug)]
    struct C(usize);

    #[test]
    fn builder_with_without_static() {
        let mut world = World::new();
        let entity_a = world.spawn((A(0), B(0))).id();
        let entity_b = world.spawn((A(0), C(0))).id();

        let mut query_a = QueryBuilder::<Entity>::new(&mut world)
            .with::<A>()
            .without::<C>()
            .build();
        assert_eq!(entity_a, query_a.single(&world));

        let mut query_b = QueryBuilder::<Entity>::new(&mut world)
            .with::<A>()
            .without::<B>()
            .build();
        assert_eq!(entity_b, query_b.single(&world));
    }

    #[test]
    fn builder_with_without_dynamic() {
        let mut world = World::new();
        let entity_a = world.spawn((A(0), B(0))).id();
        let entity_b = world.spawn((A(0), C(0))).id();
        let component_id_a = world.init_component::<A>();
        let component_id_b = world.init_component::<B>();
        let component_id_c = world.init_component::<C>();

        let mut query_a = QueryBuilder::<Entity>::new(&mut world)
            .with_by_id(component_id_a)
            .without_by_id(component_id_c)
            .build();
        assert_eq!(entity_a, query_a.single(&world));

        let mut query_b = QueryBuilder::<Entity>::new(&mut world)
            .with_by_id(component_id_a)
            .without_by_id(component_id_b)
            .build();
        assert_eq!(entity_b, query_b.single(&world));
    }

    #[test]
    fn builder_or() {
        let mut world = World::new();
        world.spawn((A(0), B(0)));
        world.spawn(B(0));
        world.spawn(C(0));

        let mut query_a = QueryBuilder::<Entity>::new(&mut world)
            .term::<Or<(With<A>, With<B>)>>()
            .build();
        assert_eq!(2, query_a.iter(&world).count());

        let mut query_b = QueryBuilder::<Entity>::new(&mut world)
            .with::<B>()
            .term::<Or<(With<A>, Without<A>)>>()
            .build();
        assert_eq!(2, query_b.iter(&world).count());

        let mut query_c = QueryBuilder::<Entity>::new(&mut world)
            .term::<Or<(With<A>, With<B>, With<C>)>>()
            .build();
        assert_eq!(3, query_c.iter(&world).count());
    }

    #[test]
    fn builder_transmute() {
        let mut world = World::new();
        world.spawn(A(0));
        world.spawn((A(1), B(0)));
        let mut query = QueryBuilder::<()>::new(&mut world)
            .term::<&A>()
            .with::<B>()
            .build();
        unsafe {
            query
                .transmute_mut::<&A>()
                .iter(&world)
                .for_each(|a| assert_eq!(a.0, 1));
        }
    }

    #[test]
    fn builder_ptr_static() {
        let mut world = World::new();
        let entity = world.spawn((A(0), B(1))).id();

        // Using term_at is currently unsafe as it allows you to edit the targets of arbitrary terms
        // possibly putting the terms in the iterator out of sync with the internal state
        let mut query = unsafe {
            QueryBuilder::<(Entity, Ptr, Ptr)>::new(&mut world)
                .term_at(1)
                .set_dynamic::<A>()
                .term_at(2)
                .set_dynamic::<B>()
                .build()
        };

        let (e, a, b) = query.single(&world);

        assert_eq!(e, entity);

        let a = unsafe { a.deref::<A>() };
        let b = unsafe { b.deref::<B>() };

        assert_eq!(0, a.0);
        assert_eq!(1, b.0);
    }

    #[test]
    fn builder_ptr_dynamic() {
        let mut world = World::new();
        let entity = world.spawn((A(0), B(1))).id();
        let component_id_a = world.init_component::<A>();
        let component_id_b = world.init_component::<B>();

        // Using term_at is currently unsafe as it allows you to edit the targets of arbitrary terms
        // possibly putting the terms in the iterator out of sync with the internal state
        let mut query = unsafe {
            QueryBuilder::<(Entity, Ptr, Ptr)>::new(&mut world)
                .term_at(1)
                .set_dynamic_by_id(component_id_a)
                .term_at(2)
                .set_dynamic_by_id(component_id_b)
                .build()
        };

        let (e, a, b) = query.single(&world);

        assert_eq!(e, entity);

        let a = unsafe { a.deref::<A>() };
        let b = unsafe { b.deref::<B>() };

        assert_eq!(0, a.0);
        assert_eq!(1, b.0);
    }

    #[test]
    fn term_query_raw() {
        let mut world = World::new();
        let entity = world.spawn((A(0), B(1))).id();
        let mut query = TermQueryState::<(Entity, &A, &B)>::new(&mut world);

        // Our result is completely untyped
        let (_, fetches) = query.single_raw(&mut world);
        // Consume our fetched terms to produce a set of term items
        let (e, a, b) = unsafe { <(Entity, &A, &B)>::from_fetches(&mut fetches.iter()) };

        assert_eq!(e, entity);
        assert_eq!(0, a.0);
        assert_eq!(1, b.0);

        // Alternatively extract individual terms dynamically
        let (_, fetches) = query.single_raw(&mut world);

        assert_eq!(0, unsafe { <&A>::from_fetch(&fetches[1]) }.0);
        assert_eq!(e, unsafe { Entity::from_fetch(&fetches[0]) });
        assert_eq!(1, unsafe { <&B>::from_fetch(&fetches[2]) }.0);
    }

    #[test]
    fn builder_raw_parts() {
        let mut world = World::new();
        let entity = world.spawn((A(0), B(1))).id();
        let mut query = QueryBuilder::<()>::new(&mut world)
            .term::<(Entity, &A)>()
            .term::<(Entity, &B)>()
            .build();

        let (_, fetches) = query.single_raw(&mut world);
        let mut iter = fetches.iter();

        // Seperately consume our two terms
        let (e1, a) = unsafe { <(Entity, &A)>::from_fetches(&mut iter) };
        let (e2, b) = unsafe { <(Entity, &B)>::from_fetches(&mut iter) };

        assert_eq!(e1, entity);
        assert_eq!(e1, e2);
        assert_eq!(0, a.0);
        assert_eq!(1, b.0);
    }

    #[test]
    fn term_query_system() {
        let mut world = World::new();
        world.spawn(A(1));
        let entity = world.spawn((A(0), B(1))).id();

        let sys = move |query: TermQuery<(Entity, &A, &B)>| {
            let (e, a, b) = query.single();
            assert_eq!(e, entity);
            assert_eq!(0, a.0);
            assert_eq!(1, b.0);
        };

        let mut system = IntoSystem::into_system(sys);
        system.initialize(&mut world);
        system.run((), &mut world);
    }

    #[test]
    fn builder_query_system() {
        let mut world = World::new();
        world.spawn(A(0));
        let entity = world.spawn((A(1), B(0))).id();

        let sys = move |query: TermQuery<(Entity, &A)>| {
            let (e, a) = query.single();
            assert_eq!(e, entity);
            assert_eq!(1, a.0);
        };

        // Add additional terms that don't appear in the original query
        let query = QueryBuilder::<(Entity, &A)>::new(&mut world)
            .with::<B>()
            .build();
        let mut system = IntoSystem::into_system(sys);
        system.initialize(&mut world);
        unsafe { system.state_mut().0 = query };
        system.run((), &mut world);

        // Alternatively truncate terms from a query to match the system
        let query = QueryBuilder::<(Entity, &A, &B)>::new(&mut world).build();
        let mut system = IntoSystem::into_system(sys);
        system.initialize(&mut world);
        unsafe { system.state_mut().0 = query.transmute() };
        system.run((), &mut world);
    }

    #[test]
    fn term_query_has() {
        let mut world = World::new();
        world.spawn((A(0), B(0), C(0)));
        world.spawn((A(0), B(0)));

        let mut query = QueryBuilder::<(Has<B>, Has<C>)>::new(&mut world)
            .with::<A>()
            .build();
        assert_eq!(
            vec![(true, true), (true, false)],
            query.iter(&world).collect::<Vec<_>>()
        );
    }

    #[test]
    fn term_query_added() {
        let mut world = World::new();
        let entity_a = world.spawn(A(0)).id();

        let mut query = QueryBuilder::<(Entity, Has<B>)>::new(&mut world)
            .term::<Added<A>>()
            .build();

        assert_eq!((entity_a, false), query.single(&world));

        world.clear_trackers();

        let entity_b = world.spawn((A(0), B(0))).id();
        assert_eq!((entity_b, true), query.single(&world));

        world.clear_trackers();

        assert!(query.get_single(&world).is_err());
    }

    #[test]
    fn term_query_changed() {
        let mut world = World::new();
        let entity_a = world.spawn(A(0)).id();

        let mut detection_query = QueryBuilder::<Entity>::new(&mut world)
            .term::<Changed<A>>()
            .build();

        let mut change_query = QueryBuilder::<&mut A>::new(&mut world).build();
        assert_eq!(entity_a, detection_query.single(&world));

        world.clear_trackers();

        assert!(detection_query.get_single(&world).is_err());

        change_query.single_mut(&mut world).0 = 1;

        assert_eq!(entity_a, detection_query.single(&world));
    }

    #[test]
    fn term_query_any_of() {
        let mut world = World::new();
        let entity_a = world.spawn((A(0), C(0))).id();
        let entity_b = world.spawn((A(0), B(0))).id();

        let mut query = QueryBuilder::<(Entity, AnyOf<(&B, &C)>)>::new(&mut world).build();

        assert_eq!(
            vec![
                (entity_a, (None, Some(&C(0)))),
                (entity_b, (Some(&B(0)), None))
            ],
            query.iter(&world).collect::<Vec<_>>()
        );
    }

    #[test]
    fn term_query_entity_ref() {
        let mut world = World::new();
        world.spawn((A(0), B(0)));

        let mut query = QueryBuilder::<EntityRef>::new(&mut world)
            .with::<A>()
            .build();

        let entity = query.single(&world);
        assert_eq!(Some(&A(0)), entity.get::<A>());
        assert_eq!(Some(&B(0)), entity.get::<B>());
        assert_eq!(None, entity.get::<C>());
    }

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct S(usize);

    #[test]
    fn term_query_sparse_set() {
        let mut world = World::new();
        let entity_a = world.spawn((A(0), S(1))).id();

        let mut query = world.term_query::<(Entity, &A, &S)>();

        let (e, a, s) = query.single(&world);
        assert_eq!(entity_a, e);
        assert_eq!(0, a.0);
        assert_eq!(1, s.0);
    }

    #[test]
    fn term_query_iteration() {
        let mut world = World::new();
        let entity = world.spawn((A(1), B(0), C(0))).id();
        world.spawn_batch((1..1000).map(|i| (A(i), B(0))));

        let mut query = world.term_query::<(&A, &mut B)>();

        query
            .iter_mut(&mut world)
            .for_each(|(a, mut b)| b.0 = a.0 * 2);

        let mut query = world.term_query_filtered::<(Entity, &B), With<C>>();
        let (e, b) = query.single(&world);

        assert_eq!(e, entity);
        assert_eq!(2, b.0);
    }
}
