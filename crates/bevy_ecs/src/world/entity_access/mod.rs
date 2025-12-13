mod component_fetch;
mod entity_mut;
mod entity_ref;
mod entry;
mod except;
mod filtered;
mod world_mut;

pub use component_fetch::*;
pub use entity_mut::*;
pub use entity_ref::*;
pub use entry::*;
pub use except::*;
pub use filtered::*;
pub use world_mut::*;

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};
    use bevy_ptr::{OwningPtr, Ptr};
    use core::panic::AssertUnwindSafe;
    use std::sync::OnceLock;

    use crate::change_detection::Tick;
    use crate::lifecycle::HookContext;
    use crate::query::QueryAccessError;
    use crate::{
        change_detection::{MaybeLocation, MutUntyped},
        component::ComponentId,
        prelude::*,
        system::{assert_is_system, RunSystemOnce as _},
        world::{error::EntityComponentError, DeferredWorld, FilteredEntityMut, FilteredEntityRef},
    };

    use super::{EntityMutExcept, EntityRefExcept};

    #[derive(Component, Clone, Copy, Debug, PartialEq)]
    struct TestComponent(u32);

    #[derive(Component, Clone, Copy, Debug, PartialEq)]
    #[component(storage = "SparseSet")]
    struct TestComponent2(u32);

    #[derive(Component)]
    struct Marker;

    #[test]
    fn entity_ref_get_by_id() {
        let mut world = World::new();
        let entity = world.spawn(TestComponent(42)).id();
        let component_id = world
            .components()
            .get_valid_id(core::any::TypeId::of::<TestComponent>())
            .unwrap();

        let entity = world.entity(entity);
        let test_component = entity.get_by_id(component_id).unwrap();
        // SAFETY: points to a valid `TestComponent`
        let test_component = unsafe { test_component.deref::<TestComponent>() };

        assert_eq!(test_component.0, 42);
    }

    #[test]
    fn entity_mut_get_by_id() {
        let mut world = World::new();
        let entity = world.spawn(TestComponent(42)).id();
        let component_id = world
            .components()
            .get_valid_id(core::any::TypeId::of::<TestComponent>())
            .unwrap();

        let mut entity_mut = world.entity_mut(entity);
        let mut test_component = entity_mut.get_mut_by_id(component_id).unwrap();
        {
            test_component.set_changed();
            let test_component =
                // SAFETY: `test_component` has unique access of the `EntityWorldMut` and is not used afterwards
                unsafe { test_component.into_inner().deref_mut::<TestComponent>() };
            test_component.0 = 43;
        }

        let entity = world.entity(entity);
        let test_component = entity.get_by_id(component_id).unwrap();
        // SAFETY: `TestComponent` is the correct component type
        let test_component = unsafe { test_component.deref::<TestComponent>() };

        assert_eq!(test_component.0, 43);
    }

    #[test]
    fn entity_ref_get_by_id_invalid_component_id() {
        let invalid_component_id = ComponentId::new(usize::MAX);

        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let entity = world.entity(entity);
        assert!(entity.get_by_id(invalid_component_id).is_err());
    }

    #[test]
    fn entity_mut_get_by_id_invalid_component_id() {
        let invalid_component_id = ComponentId::new(usize::MAX);

        let mut world = World::new();
        let mut entity = world.spawn_empty();
        assert!(entity.get_by_id(invalid_component_id).is_err());
        assert!(entity.get_mut_by_id(invalid_component_id).is_err());
    }

    #[derive(Resource)]
    struct R(usize);

    #[test]
    fn entity_mut_resource_scope() {
        // Keep in sync with the `resource_scope` test in lib.rs
        let mut world = World::new();
        let mut entity = world.spawn_empty();

        assert!(entity.try_resource_scope::<R, _>(|_, _| {}).is_none());
        entity.world_scope(|world| world.insert_resource(R(0)));
        entity.resource_scope(|entity: &mut EntityWorldMut, mut value: Mut<R>| {
            value.0 += 1;
            assert!(!entity.world().contains_resource::<R>());
        });
        assert_eq!(entity.resource::<R>().0, 1);
    }

    #[test]
    fn entity_mut_resource_scope_panic() {
        let mut world = World::new();
        world.insert_resource(R(0));

        let mut entity = world.spawn_empty();
        let old_location = entity.location();
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            entity.resource_scope(|entity: &mut EntityWorldMut, _: Mut<R>| {
                // Change the entity's `EntityLocation`.
                entity.insert(TestComponent(0));

                // Ensure that the entity location still gets updated even in case of a panic.
                panic!("this should get caught by the outer scope")
            });
        }));
        assert!(result.is_err());

        // Ensure that the location has been properly updated.
        assert_ne!(entity.location(), old_location);
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7387
    #[test]
    fn entity_mut_world_scope_panic() {
        let mut world = World::new();

        let mut entity = world.spawn_empty();
        let old_location = entity.location();
        let id = entity.id();
        let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
            entity.world_scope(|w| {
                // Change the entity's `EntityLocation`, which invalidates the original `EntityWorldMut`.
                // This will get updated at the end of the scope.
                w.entity_mut(id).insert(TestComponent(0));

                // Ensure that the entity location still gets updated even in case of a panic.
                panic!("this should get caught by the outer scope")
            });
        }));
        assert!(res.is_err());

        // Ensure that the location has been properly updated.
        assert_ne!(entity.location(), old_location);
    }

    #[test]
    fn entity_mut_reborrow_scope_panic() {
        let mut world = World::new();

        let mut entity = world.spawn_empty();
        let old_location = entity.location();
        let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
            entity.reborrow_scope(|mut entity| {
                // Change the entity's `EntityLocation`, which invalidates the original `EntityWorldMut`.
                // This will get updated at the end of the scope.
                entity.insert(TestComponent(0));

                // Ensure that the entity location still gets updated even in case of a panic.
                panic!("this should get caught by the outer scope")
            });
        }));
        assert!(res.is_err());

        // Ensure that the location has been properly updated.
        assert_ne!(entity.location(), old_location);
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn removing_sparse_updates_archetype_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn((Dense(0), Sparse)).id();
        let e2 = world.spawn((Dense(1), Sparse)).id();

        world.entity_mut(e1).remove::<Sparse>();
        assert_eq!(world.entity(e2).get::<Dense>().unwrap(), &Dense(1));
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn removing_dense_updates_table_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn((Dense(0), Sparse)).id();
        let e2 = world.spawn((Dense(1), Sparse)).id();

        world.entity_mut(e1).remove::<Dense>();
        assert_eq!(world.entity(e2).get::<Dense>().unwrap(), &Dense(1));
    }

    // Test that calling retain with `()` removes all components.
    #[test]
    fn retain_nothing() {
        #[derive(Component)]
        struct Marker<const N: usize>;

        let mut world = World::new();
        let ent = world.spawn((Marker::<1>, Marker::<2>, Marker::<3>)).id();

        world.entity_mut(ent).retain::<()>();
        assert_eq!(world.entity(ent).archetype().components().len(), 0);
    }

    // Test removing some components with `retain`, including components not on the entity.
    #[test]
    fn retain_some_components() {
        #[derive(Component)]
        struct Marker<const N: usize>;

        let mut world = World::new();
        let ent = world.spawn((Marker::<1>, Marker::<2>, Marker::<3>)).id();

        world.entity_mut(ent).retain::<(Marker<2>, Marker<4>)>();
        // Check that marker 2 was retained.
        assert!(world.entity(ent).get::<Marker<2>>().is_some());
        // Check that only marker 2 was retained.
        assert_eq!(world.entity(ent).archetype().components().len(), 1);
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn inserting_sparse_updates_archetype_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn(Dense(0)).id();
        let e2 = world.spawn(Dense(1)).id();

        world.entity_mut(e1).insert(Sparse);
        assert_eq!(world.entity(e2).get::<Dense>().unwrap(), &Dense(1));
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn inserting_dense_updates_archetype_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        struct Dense2;

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn(Dense(0)).id();
        let e2 = world.spawn(Dense(1)).id();

        world.entity_mut(e1).insert(Sparse).remove::<Sparse>();

        // archetype with [e2, e1]
        // table with [e1, e2]

        world.entity_mut(e2).insert(Dense2);

        assert_eq!(world.entity(e1).get::<Dense>().unwrap(), &Dense(0));
    }

    #[test]
    fn inserting_dense_updates_table_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        struct Dense2;

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn(Dense(0)).id();
        let e2 = world.spawn(Dense(1)).id();

        world.entity_mut(e1).insert(Sparse).remove::<Sparse>();

        // archetype with [e2, e1]
        // table with [e1, e2]

        world.entity_mut(e1).insert(Dense2);

        assert_eq!(world.entity(e2).get::<Dense>().unwrap(), &Dense(1));
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn despawning_entity_updates_archetype_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn(Dense(0)).id();
        let e2 = world.spawn(Dense(1)).id();

        world.entity_mut(e1).insert(Sparse).remove::<Sparse>();

        // archetype with [e2, e1]
        // table with [e1, e2]

        world.entity_mut(e2).despawn();

        assert_eq!(world.entity(e1).get::<Dense>().unwrap(), &Dense(0));
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn despawning_entity_updates_table_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn(Dense(0)).id();
        let e2 = world.spawn(Dense(1)).id();

        world.entity_mut(e1).insert(Sparse).remove::<Sparse>();

        // archetype with [e2, e1]
        // table with [e1, e2]

        world.entity_mut(e1).despawn();

        assert_eq!(world.entity(e2).get::<Dense>().unwrap(), &Dense(1));
    }

    #[test]
    fn entity_mut_insert_by_id() {
        let mut world = World::new();
        let test_component_id = world.register_component::<TestComponent>();

        let mut entity = world.spawn_empty();
        OwningPtr::make(TestComponent(42), |ptr| {
            // SAFETY: `ptr` matches the component id
            unsafe { entity.insert_by_id(test_component_id, ptr) };
        });

        let components: Vec<_> = world.query::<&TestComponent>().iter(&world).collect();

        assert_eq!(components, vec![&TestComponent(42)]);

        // Compare with `insert_bundle_by_id`

        let mut entity = world.spawn_empty();
        OwningPtr::make(TestComponent(84), |ptr| {
            // SAFETY: `ptr` matches the component id
            unsafe { entity.insert_by_ids(&[test_component_id], vec![ptr].into_iter()) };
        });

        let components: Vec<_> = world.query::<&TestComponent>().iter(&world).collect();

        assert_eq!(components, vec![&TestComponent(42), &TestComponent(84)]);
    }

    #[test]
    fn entity_mut_insert_bundle_by_id() {
        let mut world = World::new();
        let test_component_id = world.register_component::<TestComponent>();
        let test_component_2_id = world.register_component::<TestComponent2>();

        let component_ids = [test_component_id, test_component_2_id];
        let test_component_value = TestComponent(42);
        let test_component_2_value = TestComponent2(84);

        let mut entity = world.spawn_empty();
        OwningPtr::make(test_component_value, |ptr1| {
            OwningPtr::make(test_component_2_value, |ptr2| {
                // SAFETY: `ptr1` and `ptr2` match the component ids
                unsafe { entity.insert_by_ids(&component_ids, vec![ptr1, ptr2].into_iter()) };
            });
        });

        let dynamic_components: Vec<_> = world
            .query::<(&TestComponent, &TestComponent2)>()
            .iter(&world)
            .collect();

        assert_eq!(
            dynamic_components,
            vec![(&TestComponent(42), &TestComponent2(84))]
        );

        // Compare with `World` generated using static type equivalents
        let mut static_world = World::new();

        static_world.spawn((test_component_value, test_component_2_value));
        let static_components: Vec<_> = static_world
            .query::<(&TestComponent, &TestComponent2)>()
            .iter(&static_world)
            .collect();

        assert_eq!(dynamic_components, static_components);
    }

    #[test]
    fn entity_mut_remove_by_id() {
        let mut world = World::new();
        let test_component_id = world.register_component::<TestComponent>();

        let mut entity = world.spawn(TestComponent(42));
        entity.remove_by_id(test_component_id);

        let components: Vec<_> = world.query::<&TestComponent>().iter(&world).collect();

        assert_eq!(components, vec![] as Vec<&TestComponent>);

        // remove non-existent component does not panic
        world.spawn_empty().remove_by_id(test_component_id);
    }

    /// Tests that components can be accessed through an `EntityRefExcept`.
    #[test]
    fn entity_ref_except() {
        let mut world = World::new();
        world.register_component::<TestComponent>();
        world.register_component::<TestComponent2>();

        world.spawn((TestComponent(0), TestComponent2(0), Marker));

        let mut query = world.query_filtered::<EntityRefExcept<TestComponent>, With<Marker>>();

        let mut found = false;
        for entity_ref in query.iter_mut(&mut world) {
            found = true;
            assert!(entity_ref.get::<TestComponent>().is_none());
            assert!(entity_ref.get_ref::<TestComponent>().is_none());
            assert!(matches!(
                entity_ref.get::<TestComponent2>(),
                Some(TestComponent2(0))
            ));
        }

        assert!(found);
    }

    // Test that a single query can't both contain a mutable reference to a
    // component C and an `EntityRefExcept` that doesn't include C among its
    // exclusions.
    #[test]
    #[should_panic]
    fn entity_ref_except_conflicts_with_self() {
        let mut world = World::new();
        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        // This should panic, because we have a mutable borrow on
        // `TestComponent` but have a simultaneous indirect immutable borrow on
        // that component via `EntityRefExcept`.
        world.run_system_once(system).unwrap();

        fn system(_: Query<(&mut TestComponent, EntityRefExcept<TestComponent2>)>) {}
    }

    // Test that an `EntityRefExcept` that doesn't include a component C among
    // its exclusions can't coexist with a mutable query for that component.
    #[test]
    #[should_panic]
    fn entity_ref_except_conflicts_with_other() {
        let mut world = World::new();
        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        // This should panic, because we have a mutable borrow on
        // `TestComponent` but have a simultaneous indirect immutable borrow on
        // that component via `EntityRefExcept`.
        world.run_system_once(system).unwrap();

        fn system(_: Query<&mut TestComponent>, _: Query<EntityRefExcept<TestComponent2>>) {}
    }

    // Test that an `EntityRefExcept` with an exception for some component C can
    // coexist with a query for that component C.
    #[test]
    fn entity_ref_except_doesnt_conflict() {
        let mut world = World::new();
        world.spawn((TestComponent(0), TestComponent2(0), Marker));

        world.run_system_once(system).unwrap();

        fn system(
            _: Query<&mut TestComponent, With<Marker>>,
            query: Query<EntityRefExcept<TestComponent>, With<Marker>>,
        ) {
            for entity_ref in query.iter() {
                assert!(matches!(
                    entity_ref.get::<TestComponent2>(),
                    Some(TestComponent2(0))
                ));
            }
        }
    }

    /// Tests that components can be mutably accessed through an
    /// `EntityMutExcept`.
    #[test]
    fn entity_mut_except() {
        let mut world = World::new();
        world.spawn((TestComponent(0), TestComponent2(0), Marker));

        let mut query = world.query_filtered::<EntityMutExcept<TestComponent>, With<Marker>>();

        let mut found = false;
        for mut entity_mut in query.iter_mut(&mut world) {
            found = true;
            assert!(entity_mut.get::<TestComponent>().is_none());
            assert!(entity_mut.get_ref::<TestComponent>().is_none());
            assert!(entity_mut.get_mut::<TestComponent>().is_none());
            assert!(matches!(
                entity_mut.get::<TestComponent2>(),
                Some(TestComponent2(0))
            ));
        }

        assert!(found);
    }

    // Test that a single query can't both contain a mutable reference to a
    // component C and an `EntityMutExcept` that doesn't include C among its
    // exclusions.
    #[test]
    #[should_panic]
    fn entity_mut_except_conflicts_with_self() {
        let mut world = World::new();
        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        // This should panic, because we have a mutable borrow on
        // `TestComponent` but have a simultaneous indirect immutable borrow on
        // that component via `EntityRefExcept`.
        world.run_system_once(system).unwrap();

        fn system(_: Query<(&mut TestComponent, EntityMutExcept<TestComponent2>)>) {}
    }

    // Test that an `EntityMutExcept` that doesn't include a component C among
    // its exclusions can't coexist with a query for that component.
    #[test]
    #[should_panic]
    fn entity_mut_except_conflicts_with_other() {
        let mut world = World::new();
        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        // This should panic, because we have a mutable borrow on
        // `TestComponent` but have a simultaneous indirect immutable borrow on
        // that component via `EntityRefExcept`.
        world.run_system_once(system).unwrap();

        fn system(_: Query<&mut TestComponent>, mut query: Query<EntityMutExcept<TestComponent2>>) {
            for mut entity_mut in query.iter_mut() {
                assert!(entity_mut
                    .get_mut::<TestComponent2>()
                    .is_some_and(|component| component.0 == 0));
            }
        }
    }

    // Test that an `EntityMutExcept` with an exception for some component C can
    // coexist with a query for that component C.
    #[test]
    fn entity_mut_except_doesnt_conflict() {
        let mut world = World::new();
        world.spawn((TestComponent(0), TestComponent2(0), Marker));

        world.run_system_once(system).unwrap();

        fn system(
            _: Query<&mut TestComponent, With<Marker>>,
            mut query: Query<EntityMutExcept<TestComponent>, With<Marker>>,
        ) {
            for mut entity_mut in query.iter_mut() {
                assert!(entity_mut
                    .get_mut::<TestComponent2>()
                    .is_some_and(|component| component.0 == 0));
            }
        }
    }

    #[test]
    fn entity_mut_except_registers_components() {
        // Checks for a bug where `EntityMutExcept` would not register the component and
        // would therefore not include an exception, causing it to conflict with the later query.
        fn system1(_query: Query<EntityMutExcept<TestComponent>>, _: Query<&mut TestComponent>) {}
        let mut world = World::new();
        world.run_system_once(system1).unwrap();

        fn system2(_: Query<&mut TestComponent>, _query: Query<EntityMutExcept<TestComponent>>) {}
        let mut world = World::new();
        world.run_system_once(system2).unwrap();
    }

    #[derive(Component)]
    struct A;

    #[test]
    fn disjoint_access() {
        fn disjoint_readonly(_: Query<EntityMut, With<A>>, _: Query<EntityRef, Without<A>>) {}

        fn disjoint_mutable(_: Query<EntityMut, With<A>>, _: Query<EntityMut, Without<A>>) {}

        assert_is_system(disjoint_readonly);
        assert_is_system(disjoint_mutable);
    }

    #[test]
    fn ref_compatible() {
        fn borrow_system(_: Query<(EntityRef, &A)>, _: Query<&A>) {}

        assert_is_system(borrow_system);
    }

    #[test]
    fn ref_compatible_with_resource() {
        fn borrow_system(_: Query<EntityRef>, _: Res<R>) {}

        assert_is_system(borrow_system);
    }

    #[test]
    fn ref_compatible_with_resource_mut() {
        fn borrow_system(_: Query<EntityRef>, _: ResMut<R>) {}

        assert_is_system(borrow_system);
    }

    #[test]
    #[should_panic]
    fn ref_incompatible_with_mutable_component() {
        fn incompatible_system(_: Query<(EntityRef, &mut A)>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    #[should_panic]
    fn ref_incompatible_with_mutable_query() {
        fn incompatible_system(_: Query<EntityRef>, _: Query<&mut A>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    fn mut_compatible_with_entity() {
        fn borrow_mut_system(_: Query<(Entity, EntityMut)>) {}

        assert_is_system(borrow_mut_system);
    }

    #[test]
    fn mut_compatible_with_resource() {
        fn borrow_mut_system(_: Res<R>, _: Query<EntityMut>) {}

        assert_is_system(borrow_mut_system);
    }

    #[test]
    fn mut_compatible_with_resource_mut() {
        fn borrow_mut_system(_: ResMut<R>, _: Query<EntityMut>) {}

        assert_is_system(borrow_mut_system);
    }

    #[test]
    #[should_panic]
    fn mut_incompatible_with_read_only_component() {
        fn incompatible_system(_: Query<(EntityMut, &A)>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    #[should_panic]
    fn mut_incompatible_with_mutable_component() {
        fn incompatible_system(_: Query<(EntityMut, &mut A)>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    #[should_panic]
    fn mut_incompatible_with_read_only_query() {
        fn incompatible_system(_: Query<EntityMut>, _: Query<&A>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    #[should_panic]
    fn mut_incompatible_with_mutable_query() {
        fn incompatible_system(_: Query<EntityMut>, _: Query<&mut A>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    fn filtered_entity_ref_normal() {
        let mut world = World::new();
        let a_id = world.register_component::<A>();

        let e: FilteredEntityRef = world.spawn(A).into();

        assert!(e.get::<A>().is_some());
        assert!(e.get_ref::<A>().is_some());
        assert!(e.get_change_ticks::<A>().is_some());
        assert!(e.get_by_id(a_id).is_some());
        assert!(e.get_change_ticks_by_id(a_id).is_some());
    }

    #[test]
    fn filtered_entity_ref_missing() {
        let mut world = World::new();
        let a_id = world.register_component::<A>();

        let e: FilteredEntityRef = world.spawn(()).into();

        assert!(e.get::<A>().is_none());
        assert!(e.get_ref::<A>().is_none());
        assert!(e.get_change_ticks::<A>().is_none());
        assert!(e.get_by_id(a_id).is_none());
        assert!(e.get_change_ticks_by_id(a_id).is_none());
    }

    #[test]
    fn filtered_entity_mut_normal() {
        let mut world = World::new();
        let a_id = world.register_component::<A>();

        let mut e: FilteredEntityMut = world.spawn(A).into();

        assert!(e.get::<A>().is_some());
        assert!(e.get_ref::<A>().is_some());
        assert!(e.get_mut::<A>().is_some());
        assert!(e.get_change_ticks::<A>().is_some());
        assert!(e.get_by_id(a_id).is_some());
        assert!(e.get_mut_by_id(a_id).is_some());
        assert!(e.get_change_ticks_by_id(a_id).is_some());
    }

    #[test]
    fn filtered_entity_mut_missing() {
        let mut world = World::new();
        let a_id = world.register_component::<A>();

        let mut e: FilteredEntityMut = world.spawn(()).into();

        assert!(e.get::<A>().is_none());
        assert!(e.get_ref::<A>().is_none());
        assert!(e.get_mut::<A>().is_none());
        assert!(e.get_change_ticks::<A>().is_none());
        assert!(e.get_by_id(a_id).is_none());
        assert!(e.get_mut_by_id(a_id).is_none());
        assert!(e.get_change_ticks_by_id(a_id).is_none());
    }

    #[derive(Component, PartialEq, Eq, Debug)]
    struct X(usize);

    #[derive(Component, PartialEq, Eq, Debug)]
    struct Y(usize);

    #[test]
    fn get_components() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();
        let e2 = world.spawn(X(8)).id();
        let e3 = world.spawn_empty().id();

        assert_eq!(
            Ok((&X(7), &Y(10))),
            world.entity(e1).get_components::<(&X, &Y)>()
        );
        assert_eq!(
            Err(QueryAccessError::EntityDoesNotMatch),
            world.entity(e2).get_components::<(&X, &Y)>()
        );
        assert_eq!(
            Err(QueryAccessError::EntityDoesNotMatch),
            world.entity(e3).get_components::<(&X, &Y)>()
        );
    }

    #[test]
    fn get_components_mut() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();

        let mut entity_mut_1 = world.entity_mut(e1);
        let Ok((mut x, mut y)) = entity_mut_1.get_components_mut::<(&mut X, &mut Y)>() else {
            panic!("could not get components");
        };
        x.0 += 1;
        y.0 += 1;

        assert_eq!(
            Ok((&X(8), &Y(11))),
            world.entity(e1).get_components::<(&X, &Y)>()
        );
    }

    #[test]
    fn get_by_id_array() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();
        let e2 = world.spawn(X(8)).id();
        let e3 = world.spawn_empty().id();

        let x_id = world.register_component::<X>();
        let y_id = world.register_component::<Y>();

        assert_eq!(
            Ok((&X(7), &Y(10))),
            world
                .entity(e1)
                .get_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(y_id)),
            world
                .entity(e2)
                .get_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(x_id)),
            world
                .entity(e3)
                .get_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
    }

    #[test]
    fn get_by_id_vec() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();
        let e2 = world.spawn(X(8)).id();
        let e3 = world.spawn_empty().id();

        let x_id = world.register_component::<X>();
        let y_id = world.register_component::<Y>();

        assert_eq!(
            Ok((&X(7), &Y(10))),
            world
                .entity(e1)
                .get_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[Ptr; 2], _> = ptrs.try_into() else {
                        panic!("get_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(y_id)),
            world
                .entity(e2)
                .get_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[Ptr; 2], _> = ptrs.try_into() else {
                        panic!("get_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(x_id)),
            world
                .entity(e3)
                .get_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[Ptr; 2], _> = ptrs.try_into() else {
                        panic!("get_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
    }

    #[test]
    fn get_mut_by_id_array() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();
        let e2 = world.spawn(X(8)).id();
        let e3 = world.spawn_empty().id();

        let x_id = world.register_component::<X>();
        let y_id = world.register_component::<Y>();

        assert_eq!(
            Ok((&mut X(7), &mut Y(10))),
            world
                .entity_mut(e1)
                .get_mut_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(y_id)),
            world
                .entity_mut(e2)
                .get_mut_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(x_id)),
            world
                .entity_mut(e3)
                .get_mut_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );

        assert_eq!(
            Err(EntityComponentError::AliasedMutability(x_id)),
            world
                .entity_mut(e1)
                .get_mut_by_id([x_id, x_id])
                .map(|_| { unreachable!() })
        );
        assert_eq!(
            Err(EntityComponentError::AliasedMutability(x_id)),
            world
                .entity_mut(e3)
                .get_mut_by_id([x_id, x_id])
                .map(|_| { unreachable!() })
        );
    }

    #[test]
    fn get_mut_by_id_vec() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();
        let e2 = world.spawn(X(8)).id();
        let e3 = world.spawn_empty().id();

        let x_id = world.register_component::<X>();
        let y_id = world.register_component::<Y>();

        assert_eq!(
            Ok((&mut X(7), &mut Y(10))),
            world
                .entity_mut(e1)
                .get_mut_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[MutUntyped; 2], _> = ptrs.try_into() else {
                        panic!("get_mut_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(y_id)),
            world
                .entity_mut(e2)
                .get_mut_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[MutUntyped; 2], _> = ptrs.try_into() else {
                        panic!("get_mut_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(x_id)),
            world
                .entity_mut(e3)
                .get_mut_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[MutUntyped; 2], _> = ptrs.try_into() else {
                        panic!("get_mut_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );

        assert_eq!(
            Err(EntityComponentError::AliasedMutability(x_id)),
            world
                .entity_mut(e1)
                .get_mut_by_id(&[x_id, x_id])
                .map(|_| { unreachable!() })
        );
        assert_eq!(
            Err(EntityComponentError::AliasedMutability(x_id)),
            world
                .entity_mut(e3)
                .get_mut_by_id(&[x_id, x_id])
                .map(|_| { unreachable!() })
        );
    }

    #[test]
    fn get_mut_by_id_unchecked() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();
        let x_id = world.register_component::<X>();
        let y_id = world.register_component::<Y>();

        let e1_mut = &world.get_entity_mut([e1]).unwrap()[0];
        // SAFETY: The entity e1 contains component X.
        let x_ptr = unsafe { e1_mut.get_mut_by_id_unchecked(x_id) }.unwrap();
        // SAFETY: The entity e1 contains component Y, with components X and Y being mutually independent.
        let y_ptr = unsafe { e1_mut.get_mut_by_id_unchecked(y_id) }.unwrap();

        // SAFETY: components match the id they were fetched with
        let x_component = unsafe { x_ptr.into_inner().deref_mut::<X>() };
        x_component.0 += 1;
        // SAFETY: components match the id they were fetched with
        let y_component = unsafe { y_ptr.into_inner().deref_mut::<Y>() };
        y_component.0 -= 1;

        assert_eq!((&mut X(8), &mut Y(9)), (x_component, y_component));
    }

    #[derive(EntityEvent)]
    struct TestEvent(Entity);

    #[test]
    fn adding_observer_updates_location() {
        let mut world = World::new();
        let entity = world
            .spawn_empty()
            .observe(|event: On<TestEvent>, mut commands: Commands| {
                commands
                    .entity(event.event_target())
                    .insert(TestComponent(0));
            })
            .id();

        // this should not be needed, but is currently required to tease out the bug
        world.flush();

        let mut a = world.entity_mut(entity);
        // SAFETY: this _intentionally_ doesn't update the location, to ensure that we're actually testing
        // that observe() updates location
        unsafe { a.world_mut().trigger(TestEvent(entity)) }
        a.observe(|_: On<TestEvent>| {}); // this flushes commands implicitly by spawning
        let location = a.location();
        assert_eq!(world.entities().get(entity).unwrap(), Some(location));
    }

    #[test]
    #[should_panic]
    fn location_on_despawned_entity_panics() {
        let mut world = World::new();
        world.add_observer(|add: On<Add, TestComponent>, mut commands: Commands| {
            commands.entity(add.entity).despawn();
        });
        let entity = world.spawn_empty().id();
        let mut a = world.entity_mut(entity);
        a.insert(TestComponent(0));
        a.location();
    }

    #[derive(Resource)]
    struct TestFlush(usize);

    fn count_flush(world: &mut World) {
        world.resource_mut::<TestFlush>().0 += 1;
    }

    #[test]
    fn archetype_modifications_trigger_flush() {
        let mut world = World::new();
        world.insert_resource(TestFlush(0));
        world.add_observer(|_: On<Add, TestComponent>, mut commands: Commands| {
            commands.queue(count_flush);
        });
        world.add_observer(|_: On<Remove, TestComponent>, mut commands: Commands| {
            commands.queue(count_flush);
        });

        // Spawning an empty should not flush.
        world.commands().queue(count_flush);
        let entity = world.spawn_empty().id();
        assert_eq!(world.resource::<TestFlush>().0, 0);

        world.commands().queue(count_flush);
        world.flush_commands();

        let mut a = world.entity_mut(entity);
        assert_eq!(a.world().resource::<TestFlush>().0, 2);
        a.insert(TestComponent(0));
        assert_eq!(a.world().resource::<TestFlush>().0, 3);
        a.remove::<TestComponent>();
        assert_eq!(a.world().resource::<TestFlush>().0, 4);
        a.insert(TestComponent(0));
        assert_eq!(a.world().resource::<TestFlush>().0, 5);
        let _ = a.take::<TestComponent>();
        assert_eq!(a.world().resource::<TestFlush>().0, 6);
        a.insert(TestComponent(0));
        assert_eq!(a.world().resource::<TestFlush>().0, 7);
        a.retain::<()>();
        assert_eq!(a.world().resource::<TestFlush>().0, 8);
        a.insert(TestComponent(0));
        assert_eq!(a.world().resource::<TestFlush>().0, 9);
        a.clear();
        assert_eq!(a.world().resource::<TestFlush>().0, 10);
        a.insert(TestComponent(0));
        assert_eq!(a.world().resource::<TestFlush>().0, 11);
        a.despawn();
        assert_eq!(world.resource::<TestFlush>().0, 12);
    }

    #[derive(Resource)]
    struct TestVec(Vec<&'static str>);

    #[derive(Component)]
    #[component(on_add = ord_a_hook_on_add, on_insert = ord_a_hook_on_insert, on_replace = ord_a_hook_on_replace, on_remove = ord_a_hook_on_remove)]
    struct OrdA;

    fn ord_a_hook_on_add(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
        world.resource_mut::<TestVec>().0.push("OrdA hook on_add");
        world.commands().entity(entity).insert(OrdB);
    }

    fn ord_a_hook_on_insert(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
        world
            .resource_mut::<TestVec>()
            .0
            .push("OrdA hook on_insert");
        world.commands().entity(entity).remove::<OrdA>();
        world.commands().entity(entity).remove::<OrdB>();
    }

    fn ord_a_hook_on_replace(mut world: DeferredWorld, _: HookContext) {
        world
            .resource_mut::<TestVec>()
            .0
            .push("OrdA hook on_replace");
    }

    fn ord_a_hook_on_remove(mut world: DeferredWorld, _: HookContext) {
        world
            .resource_mut::<TestVec>()
            .0
            .push("OrdA hook on_remove");
    }

    fn ord_a_observer_on_add(_event: On<Add, OrdA>, mut res: ResMut<TestVec>) {
        res.0.push("OrdA observer on_add");
    }

    fn ord_a_observer_on_insert(_event: On<Insert, OrdA>, mut res: ResMut<TestVec>) {
        res.0.push("OrdA observer on_insert");
    }

    fn ord_a_observer_on_replace(_event: On<Replace, OrdA>, mut res: ResMut<TestVec>) {
        res.0.push("OrdA observer on_replace");
    }

    fn ord_a_observer_on_remove(_event: On<Remove, OrdA>, mut res: ResMut<TestVec>) {
        res.0.push("OrdA observer on_remove");
    }

    #[derive(Component)]
    #[component(on_add = ord_b_hook_on_add, on_insert = ord_b_hook_on_insert, on_replace = ord_b_hook_on_replace, on_remove = ord_b_hook_on_remove)]
    struct OrdB;

    fn ord_b_hook_on_add(mut world: DeferredWorld, _: HookContext) {
        world.resource_mut::<TestVec>().0.push("OrdB hook on_add");
        world.commands().queue(|world: &mut World| {
            world
                .resource_mut::<TestVec>()
                .0
                .push("OrdB command on_add");
        });
    }

    fn ord_b_hook_on_insert(mut world: DeferredWorld, _: HookContext) {
        world
            .resource_mut::<TestVec>()
            .0
            .push("OrdB hook on_insert");
    }

    fn ord_b_hook_on_replace(mut world: DeferredWorld, _: HookContext) {
        world
            .resource_mut::<TestVec>()
            .0
            .push("OrdB hook on_replace");
    }

    fn ord_b_hook_on_remove(mut world: DeferredWorld, _: HookContext) {
        world
            .resource_mut::<TestVec>()
            .0
            .push("OrdB hook on_remove");
    }

    fn ord_b_observer_on_add(_event: On<Add, OrdB>, mut res: ResMut<TestVec>) {
        res.0.push("OrdB observer on_add");
    }

    fn ord_b_observer_on_insert(_event: On<Insert, OrdB>, mut res: ResMut<TestVec>) {
        res.0.push("OrdB observer on_insert");
    }

    fn ord_b_observer_on_replace(_event: On<Replace, OrdB>, mut res: ResMut<TestVec>) {
        res.0.push("OrdB observer on_replace");
    }

    fn ord_b_observer_on_remove(_event: On<Remove, OrdB>, mut res: ResMut<TestVec>) {
        res.0.push("OrdB observer on_remove");
    }

    #[test]
    fn command_ordering_is_correct() {
        let mut world = World::new();
        world.insert_resource(TestVec(Vec::new()));
        world.add_observer(ord_a_observer_on_add);
        world.add_observer(ord_a_observer_on_insert);
        world.add_observer(ord_a_observer_on_replace);
        world.add_observer(ord_a_observer_on_remove);
        world.add_observer(ord_b_observer_on_add);
        world.add_observer(ord_b_observer_on_insert);
        world.add_observer(ord_b_observer_on_replace);
        world.add_observer(ord_b_observer_on_remove);
        let _entity = world.spawn(OrdA).id();
        let expected = [
            "OrdA hook on_add", // adds command to insert OrdB
            "OrdA observer on_add",
            "OrdA hook on_insert", // adds command to despawn entity
            "OrdA observer on_insert",
            "OrdB hook on_add", // adds command to just add to this log
            "OrdB observer on_add",
            "OrdB hook on_insert",
            "OrdB observer on_insert",
            "OrdB command on_add", // command added by OrdB hook on_add, needs to run before despawn command
            "OrdA observer on_replace", // start of despawn
            "OrdA hook on_replace",
            "OrdA observer on_remove",
            "OrdA hook on_remove",
            "OrdB observer on_replace",
            "OrdB hook on_replace",
            "OrdB observer on_remove",
            "OrdB hook on_remove",
        ];
        world.flush();
        assert_eq!(world.resource_mut::<TestVec>().0.as_slice(), &expected[..]);
    }

    #[test]
    fn entity_world_mut_clone_and_move_components() {
        #[derive(Component, Clone, PartialEq, Debug)]
        struct A;

        #[derive(Component, Clone, PartialEq, Debug)]
        struct B;

        #[derive(Component, Clone, PartialEq, Debug)]
        struct C(u32);

        let mut world = World::new();
        let entity_a = world.spawn((A, B, C(5))).id();
        let entity_b = world.spawn((A, C(4))).id();

        world.entity_mut(entity_a).clone_components::<B>(entity_b);
        assert_eq!(world.entity(entity_a).get::<B>(), Some(&B));
        assert_eq!(world.entity(entity_b).get::<B>(), Some(&B));

        world.entity_mut(entity_a).move_components::<C>(entity_b);
        assert_eq!(world.entity(entity_a).get::<C>(), None);
        assert_eq!(world.entity(entity_b).get::<C>(), Some(&C(5)));

        assert_eq!(world.entity(entity_a).get::<A>(), Some(&A));
        assert_eq!(world.entity(entity_b).get::<A>(), Some(&A));
    }

    #[test]
    fn entity_world_mut_clone_with_move_and_require() {
        #[derive(Component, Clone, PartialEq, Debug)]
        #[require(B(3))]
        struct A;

        #[derive(Component, Clone, PartialEq, Debug, Default)]
        #[require(C(3))]
        struct B(u32);

        #[derive(Component, Clone, PartialEq, Debug, Default)]
        #[require(D)]
        struct C(u32);

        #[derive(Component, Clone, PartialEq, Debug, Default)]
        struct D;

        let mut world = World::new();
        let entity_a = world.spawn((A, B(5))).id();
        let entity_b = world.spawn_empty().id();

        world
            .entity_mut(entity_a)
            .clone_with_opt_in(entity_b, |builder| {
                builder
                    .move_components(true)
                    .allow::<C>()
                    .without_required_components(|builder| {
                        builder.allow::<A>();
                    });
            });

        assert_eq!(world.entity(entity_a).get::<A>(), None);
        assert_eq!(world.entity(entity_b).get::<A>(), Some(&A));

        assert_eq!(world.entity(entity_a).get::<B>(), Some(&B(5)));
        assert_eq!(world.entity(entity_b).get::<B>(), Some(&B(3)));

        assert_eq!(world.entity(entity_a).get::<C>(), None);
        assert_eq!(world.entity(entity_b).get::<C>(), Some(&C(3)));

        assert_eq!(world.entity(entity_a).get::<D>(), None);
        assert_eq!(world.entity(entity_b).get::<D>(), Some(&D));
    }

    #[test]
    fn update_despawned_by_after_observers() {
        let mut world = World::new();

        #[derive(Component)]
        #[component(on_remove = get_tracked)]
        struct C;

        static TRACKED: OnceLock<(MaybeLocation, Tick)> = OnceLock::new();
        fn get_tracked(world: DeferredWorld, HookContext { entity, .. }: HookContext) {
            TRACKED.get_or_init(|| {
                let by = world
                    .entities
                    .entity_get_spawned_or_despawned_by(entity)
                    .map(|l| l.unwrap());
                let at = world
                    .entities
                    .entity_get_spawn_or_despawn_tick(entity)
                    .unwrap();
                (by, at)
            });
        }

        #[track_caller]
        fn caller_spawn(world: &mut World) -> (Entity, MaybeLocation, Tick) {
            let caller = MaybeLocation::caller();
            (world.spawn(C).id(), caller, world.change_tick())
        }
        let (entity, spawner, spawn_tick) = caller_spawn(&mut world);

        assert_eq!(
            spawner,
            world
                .entities()
                .entity_get_spawned_or_despawned_by(entity)
                .map(|l| l.unwrap())
        );

        #[track_caller]
        fn caller_despawn(world: &mut World, entity: Entity) -> (MaybeLocation, Tick) {
            world.despawn(entity);
            (MaybeLocation::caller(), world.change_tick())
        }
        let (despawner, despawn_tick) = caller_despawn(&mut world, entity);

        assert_eq!((spawner, spawn_tick), *TRACKED.get().unwrap());
        assert_eq!(
            despawner,
            world
                .entities()
                .entity_get_spawned_or_despawned_by(entity)
                .map(|l| l.unwrap())
        );
        assert_eq!(
            despawn_tick,
            world
                .entities()
                .entity_get_spawn_or_despawn_tick(entity)
                .unwrap()
        );
    }

    #[test]
    fn with_component_activates_hooks() {
        use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

        #[derive(Component, PartialEq, Eq, Debug)]
        #[component(immutable)]
        struct Foo(bool);

        static EXPECTED_VALUE: AtomicBool = AtomicBool::new(false);

        static ADD_COUNT: AtomicU8 = AtomicU8::new(0);
        static REMOVE_COUNT: AtomicU8 = AtomicU8::new(0);
        static REPLACE_COUNT: AtomicU8 = AtomicU8::new(0);
        static INSERT_COUNT: AtomicU8 = AtomicU8::new(0);

        let mut world = World::default();

        world.register_component::<Foo>();
        world
            .register_component_hooks::<Foo>()
            .on_add(|world, context| {
                ADD_COUNT.fetch_add(1, Ordering::Relaxed);

                assert_eq!(
                    world.get(context.entity),
                    Some(&Foo(EXPECTED_VALUE.load(Ordering::Relaxed)))
                );
            })
            .on_remove(|world, context| {
                REMOVE_COUNT.fetch_add(1, Ordering::Relaxed);

                assert_eq!(
                    world.get(context.entity),
                    Some(&Foo(EXPECTED_VALUE.load(Ordering::Relaxed)))
                );
            })
            .on_replace(|world, context| {
                REPLACE_COUNT.fetch_add(1, Ordering::Relaxed);

                assert_eq!(
                    world.get(context.entity),
                    Some(&Foo(EXPECTED_VALUE.load(Ordering::Relaxed)))
                );
            })
            .on_insert(|world, context| {
                INSERT_COUNT.fetch_add(1, Ordering::Relaxed);

                assert_eq!(
                    world.get(context.entity),
                    Some(&Foo(EXPECTED_VALUE.load(Ordering::Relaxed)))
                );
            });

        let entity = world.spawn(Foo(false)).id();

        assert_eq!(ADD_COUNT.load(Ordering::Relaxed), 1);
        assert_eq!(REMOVE_COUNT.load(Ordering::Relaxed), 0);
        assert_eq!(REPLACE_COUNT.load(Ordering::Relaxed), 0);
        assert_eq!(INSERT_COUNT.load(Ordering::Relaxed), 1);

        let mut entity = world.entity_mut(entity);

        let archetype_pointer_before = &raw const *entity.archetype();

        assert_eq!(entity.get::<Foo>(), Some(&Foo(false)));

        entity.modify_component(|foo: &mut Foo| {
            foo.0 = true;
            EXPECTED_VALUE.store(foo.0, Ordering::Relaxed);
        });

        let archetype_pointer_after = &raw const *entity.archetype();

        assert_eq!(entity.get::<Foo>(), Some(&Foo(true)));

        assert_eq!(ADD_COUNT.load(Ordering::Relaxed), 1);
        assert_eq!(REMOVE_COUNT.load(Ordering::Relaxed), 0);
        assert_eq!(REPLACE_COUNT.load(Ordering::Relaxed), 1);
        assert_eq!(INSERT_COUNT.load(Ordering::Relaxed), 2);

        assert_eq!(archetype_pointer_before, archetype_pointer_after);
    }

    #[test]
    fn bundle_remove_only_triggers_for_present_components() {
        let mut world = World::default();

        #[derive(Component)]
        struct A;

        #[derive(Component)]
        struct B;

        #[derive(Resource, PartialEq, Eq, Debug)]
        struct Tracker {
            a: bool,
            b: bool,
        }

        world.insert_resource(Tracker { a: false, b: false });
        let entity = world.spawn(A).id();

        world.add_observer(|_: On<Remove, A>, mut tracker: ResMut<Tracker>| {
            tracker.a = true;
        });
        world.add_observer(|_: On<Remove, B>, mut tracker: ResMut<Tracker>| {
            tracker.b = true;
        });

        world.entity_mut(entity).remove::<(A, B)>();

        assert_eq!(
            world.resource::<Tracker>(),
            &Tracker {
                a: true,
                // The entity didn't have a B component, so it should not have been triggered.
                b: false,
            }
        );
    }

    #[test]
    fn spawned_after_swap_remove() {
        #[derive(Component)]
        struct Marker;

        let mut world = World::new();
        let id1 = world.spawn(Marker).id();
        let _id2 = world.spawn(Marker).id();
        let id3 = world.spawn(Marker).id();

        let e1_spawned = world.entity(id1).spawned_by();

        let spawn = world.entity(id3).spawned_by();
        world.entity_mut(id1).despawn();
        let e1_despawned = world.entities().entity_get_spawned_or_despawned_by(id1);

        // These assertions are only possible if the `track_location` feature is enabled
        if let (Some(e1_spawned), Some(e1_despawned)) =
            (e1_spawned.into_option(), e1_despawned.into_option())
        {
            assert!(e1_despawned.is_some());
            assert_ne!(Some(e1_spawned), e1_despawned);
        }

        let spawn_after = world.entity(id3).spawned_by();
        assert_eq!(spawn, spawn_after);
    }

    #[test]
    fn spawned_by_set_before_flush() {
        #[derive(Component)]
        #[component(on_despawn = on_despawn)]
        struct C;

        fn on_despawn(mut world: DeferredWorld, context: HookContext) {
            let spawned = world.entity(context.entity).spawned_by();
            world.commands().queue(move |world: &mut World| {
                // The entity has finished despawning...
                assert!(world.get_entity(context.entity).is_err());
                let despawned = world
                    .entities()
                    .entity_get_spawned_or_despawned_by(context.entity);
                // These assertions are only possible if the `track_location` feature is enabled
                if let (Some(spawned), Some(despawned)) =
                    (spawned.into_option(), despawned.into_option())
                {
                    // ... so ensure that `despawned_by` has been written
                    assert!(despawned.is_some());
                    assert_ne!(Some(spawned), despawned);
                }
            });
        }

        let mut world = World::new();
        let original = world.spawn(C).id();
        world.despawn(original);
    }
}
