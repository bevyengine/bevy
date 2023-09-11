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

    #[derive(Component)]
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
            .with_id(component_id_a)
            .without_id(component_id_c)
            .build();
        assert_eq!(entity_a, query_a.single(&world));

        let mut query_b = QueryBuilder::<Entity>::new(&mut world)
            .with_id(component_id_a)
            .without_id(component_id_b)
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
            .term::<Or<(With<A>, Or<(With<B>, With<C>)>)>>()
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
                .set::<A>()
                .term_at(2)
                .set::<B>()
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
                .set_id(component_id_a)
                .term_at(2)
                .set_id(component_id_b)
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
        let terms = query.single_raw(&mut world);
        // Consume our fetched terms to produce a set of term items
        let (e, a, b) = unsafe { <(Entity, &A, &B)>::from_fetches(&mut terms.into_iter()) };

        assert_eq!(e, entity);
        assert_eq!(0, a.0);
        assert_eq!(1, b.0);

        // Alternatively extract individual terms dynamically
        let terms = query.single_raw(&mut world);

        // Turn into options so we can consume them out of order
        let mut terms = terms.into_iter().map(|t| Some(t)).collect::<Vec<_>>();

        assert_eq!(0, unsafe { <&A>::from_fetch(terms[1].take().unwrap()) }.0);
        assert_eq!(e, unsafe { Entity::from_fetch(terms[0].take().unwrap()) });
        assert_eq!(1, unsafe { <&B>::from_fetch(terms[2].take().unwrap()) }.0);
    }

    #[test]
    fn builder_raw_parts() {
        let mut world = World::new();
        let entity = world.spawn((A(0), B(1))).id();
        let mut query = QueryBuilder::<()>::new(&mut world)
            .term::<(Entity, &A)>()
            .term::<(Entity, &B)>()
            .build();

        let mut terms = query.single_raw(&mut world).into_iter();

        // Seperately consume our two terms
        let (e1, a) = unsafe { <(Entity, &A)>::from_fetches(&mut terms) };
        let (e2, b) = unsafe { <(Entity, &B)>::from_fetches(&mut terms) };

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
}
