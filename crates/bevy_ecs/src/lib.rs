pub mod archetype;
pub mod bundle;
pub mod change_detection;
pub mod component;
pub mod entity;
pub mod event;
pub mod query;
#[cfg(feature = "bevy_reflect")]
pub mod reflect;
pub mod schedule;
pub mod storage;
pub mod system;
pub mod world;

pub mod prelude {
    #[doc(hidden)]
    #[cfg(feature = "bevy_reflect")]
    pub use crate::reflect::ReflectComponent;
    #[doc(hidden)]
    pub use crate::{
        bundle::Bundle,
        change_detection::DetectChanges,
        entity::Entity,
        event::{EventReader, EventWriter},
        query::{Added, ChangeTrackers, Changed, Or, QueryState, With, WithBundle, Without},
        schedule::{
            AmbiguitySetLabel, ExclusiveSystemDescriptorCoercion, ParallelSystemDescriptorCoercion,
            RunCriteria, RunCriteriaDescriptorCoercion, RunCriteriaLabel, RunCriteriaPiping,
            Schedule, Stage, StageLabel, State, SystemLabel, SystemSet, SystemStage,
        },
        system::{
            Commands, ConfigurableSystem, In, IntoChainSystem, IntoExclusiveSystem, IntoSystem,
            Local, NonSend, NonSendMut, Query, QuerySet, RemovedComponents, Res, ResMut, System,
        },
        world::{FromWorld, Mut, World},
    };
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::{
        bundle::Bundle,
        component::{Component, ComponentDescriptor, ComponentId, StorageType, TypeInfo},
        entity::Entity,
        query::{
            Added, ChangeTrackers, Changed, FilterFetch, FilteredAccess, With, Without, WorldQuery,
        },
        world::{Mut, World},
    };
    use bevy_tasks::TaskPool;
    use parking_lot::Mutex;
    use std::{
        any::TypeId,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    #[derive(Debug, PartialEq, Eq)]
    struct A(usize);
    struct B(usize);
    struct C;

    #[derive(Clone, Debug)]
    struct DropCk(Arc<AtomicUsize>);
    impl DropCk {
        fn new_pair() -> (Self, Arc<AtomicUsize>) {
            let atomic = Arc::new(AtomicUsize::new(0));
            (DropCk(atomic.clone()), atomic)
        }
    }

    impl Drop for DropCk {
        fn drop(&mut self) {
            self.0.as_ref().fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn random_access() {
        let mut world = World::new();
        world
            .register_component(ComponentDescriptor::new::<i32>(StorageType::SparseSet))
            .unwrap();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        let f = world.spawn().insert_bundle(("def", 456, true)).id();
        assert_eq!(*world.get::<&str>(e).unwrap(), "abc");
        assert_eq!(*world.get::<i32>(e).unwrap(), 123);
        assert_eq!(*world.get::<&str>(f).unwrap(), "def");
        assert_eq!(*world.get::<i32>(f).unwrap(), 456);

        // test archetype get_mut()
        *world.get_mut::<&'static str>(e).unwrap() = "xyz";
        assert_eq!(*world.get::<&'static str>(e).unwrap(), "xyz");

        // test sparse set get_mut()
        *world.get_mut::<i32>(f).unwrap() = 42;
        assert_eq!(*world.get::<i32>(f).unwrap(), 42);
    }

    #[test]
    fn bundle_derive() {
        #[derive(Bundle, PartialEq, Debug)]
        struct Foo {
            x: &'static str,
            y: i32,
        }

        assert_eq!(
            <Foo as Bundle>::type_info(),
            vec![TypeInfo::of::<&'static str>(), TypeInfo::of::<i32>(),]
        );

        let mut world = World::new();
        world
            .register_component(ComponentDescriptor::new::<i32>(StorageType::SparseSet))
            .unwrap();
        let e1 = world.spawn().insert_bundle(Foo { x: "abc", y: 123 }).id();
        let e2 = world.spawn().insert_bundle(("def", 456, true)).id();
        assert_eq!(*world.get::<&str>(e1).unwrap(), "abc");
        assert_eq!(*world.get::<i32>(e1).unwrap(), 123);
        assert_eq!(*world.get::<&str>(e2).unwrap(), "def");
        assert_eq!(*world.get::<i32>(e2).unwrap(), 456);

        // test archetype get_mut()
        *world.get_mut::<&'static str>(e1).unwrap() = "xyz";
        assert_eq!(*world.get::<&'static str>(e1).unwrap(), "xyz");

        // test sparse set get_mut()
        *world.get_mut::<i32>(e2).unwrap() = 42;
        assert_eq!(*world.get::<i32>(e2).unwrap(), 42);

        assert_eq!(
            world.entity_mut(e1).remove_bundle::<Foo>().unwrap(),
            Foo { x: "xyz", y: 123 }
        );

        #[derive(Bundle, PartialEq, Debug)]
        struct Nested {
            a: usize,
            #[bundle]
            foo: Foo,
            b: u8,
        }

        assert_eq!(
            <Nested as Bundle>::type_info(),
            vec![
                TypeInfo::of::<usize>(),
                TypeInfo::of::<&'static str>(),
                TypeInfo::of::<i32>(),
                TypeInfo::of::<u8>(),
            ]
        );

        let e3 = world
            .spawn()
            .insert_bundle(Nested {
                a: 1,
                foo: Foo { x: "ghi", y: 789 },
                b: 2,
            })
            .id();

        assert_eq!(*world.get::<&str>(e3).unwrap(), "ghi");
        assert_eq!(*world.get::<i32>(e3).unwrap(), 789);
        assert_eq!(*world.get::<usize>(e3).unwrap(), 1);
        assert_eq!(*world.get::<u8>(e3).unwrap(), 2);
        assert_eq!(
            world.entity_mut(e3).remove_bundle::<Nested>().unwrap(),
            Nested {
                a: 1,
                foo: Foo { x: "ghi", y: 789 },
                b: 2,
            }
        );
    }

    #[test]
    fn despawn_table_storage() {
        let mut world = World::new();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        let f = world.spawn().insert_bundle(("def", 456)).id();
        assert_eq!(world.entities.len(), 2);
        assert!(world.despawn(e));
        assert_eq!(world.entities.len(), 1);
        assert!(world.get::<&str>(e).is_none());
        assert!(world.get::<i32>(e).is_none());
        assert_eq!(*world.get::<&str>(f).unwrap(), "def");
        assert_eq!(*world.get::<i32>(f).unwrap(), 456);
    }

    #[test]
    fn despawn_mixed_storage() {
        let mut world = World::new();
        world
            .register_component(ComponentDescriptor::new::<i32>(StorageType::SparseSet))
            .unwrap();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        let f = world.spawn().insert_bundle(("def", 456)).id();
        assert_eq!(world.entities.len(), 2);
        assert!(world.despawn(e));
        assert_eq!(world.entities.len(), 1);
        assert!(world.get::<&str>(e).is_none());
        assert!(world.get::<i32>(e).is_none());
        assert_eq!(*world.get::<&str>(f).unwrap(), "def");
        assert_eq!(*world.get::<i32>(f).unwrap(), 456);
    }

    #[test]
    fn query_all() {
        let mut world = World::new();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        let f = world.spawn().insert_bundle(("def", 456)).id();

        let ents = world
            .query::<(Entity, &i32, &&str)>()
            .iter(&world)
            .map(|(e, &i, &s)| (e, i, s))
            .collect::<Vec<_>>();
        assert_eq!(ents, &[(e, 123, "abc"), (f, 456, "def")]);
    }

    #[test]
    fn query_all_for_each() {
        let mut world = World::new();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        let f = world.spawn().insert_bundle(("def", 456)).id();

        let mut results = Vec::new();
        world
            .query::<(Entity, &i32, &&str)>()
            .for_each(&world, |(e, &i, &s)| results.push((e, i, s)));
        assert_eq!(results, &[(e, 123, "abc"), (f, 456, "def")]);
    }

    #[test]
    fn query_single_component() {
        let mut world = World::new();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        let f = world.spawn().insert_bundle(("def", 456, true)).id();
        let ents = world
            .query::<(Entity, &i32)>()
            .iter(&world)
            .map(|(e, &i)| (e, i))
            .collect::<Vec<_>>();
        assert_eq!(ents, &[(e, 123), (f, 456)]);
    }

    #[test]
    fn stateful_query_handles_new_archetype() {
        let mut world = World::new();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        let mut query = world.query::<(Entity, &i32)>();

        let ents = query.iter(&world).map(|(e, &i)| (e, i)).collect::<Vec<_>>();
        assert_eq!(ents, &[(e, 123)]);

        let f = world.spawn().insert_bundle(("def", 456, true)).id();
        let ents = query.iter(&world).map(|(e, &i)| (e, i)).collect::<Vec<_>>();
        assert_eq!(ents, &[(e, 123), (f, 456)]);
    }

    #[test]
    fn query_single_component_for_each() {
        let mut world = World::new();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        let f = world.spawn().insert_bundle(("def", 456, true)).id();
        let mut results = Vec::new();
        world
            .query::<(Entity, &i32)>()
            .for_each(&world, |(e, &i)| results.push((e, i)));
        assert_eq!(results, &[(e, 123), (f, 456)]);
    }

    #[test]
    fn par_for_each_dense() {
        let mut world = World::new();
        let task_pool = TaskPool::default();
        let e1 = world.spawn().insert(1).id();
        let e2 = world.spawn().insert(2).id();
        let e3 = world.spawn().insert(3).id();
        let e4 = world.spawn().insert_bundle((4, true)).id();
        let e5 = world.spawn().insert_bundle((5, true)).id();
        let results = Arc::new(Mutex::new(Vec::new()));
        world
            .query::<(Entity, &i32)>()
            .par_for_each(&world, &task_pool, 2, |(e, &i)| results.lock().push((e, i)));
        results.lock().sort();
        assert_eq!(
            &*results.lock(),
            &[(e1, 1), (e2, 2), (e3, 3), (e4, 4), (e5, 5)]
        );
    }

    #[test]
    fn par_for_each_sparse() {
        let mut world = World::new();
        world
            .register_component(ComponentDescriptor::new::<i32>(StorageType::SparseSet))
            .unwrap();
        let task_pool = TaskPool::default();
        let e1 = world.spawn().insert(1).id();
        let e2 = world.spawn().insert(2).id();
        let e3 = world.spawn().insert(3).id();
        let e4 = world.spawn().insert_bundle((4, true)).id();
        let e5 = world.spawn().insert_bundle((5, true)).id();
        let results = Arc::new(Mutex::new(Vec::new()));
        world
            .query::<(Entity, &i32)>()
            .par_for_each(&world, &task_pool, 2, |(e, &i)| results.lock().push((e, i)));
        results.lock().sort();
        assert_eq!(
            &*results.lock(),
            &[(e1, 1), (e2, 2), (e3, 3), (e4, 4), (e5, 5)]
        );
    }

    #[test]
    fn query_missing_component() {
        let mut world = World::new();
        world.spawn().insert_bundle(("abc", 123));
        world.spawn().insert_bundle(("def", 456));
        assert!(world.query::<(&bool, &i32)>().iter(&world).next().is_none());
    }

    #[test]
    fn query_sparse_component() {
        let mut world = World::new();
        world.spawn().insert_bundle(("abc", 123));
        let f = world.spawn().insert_bundle(("def", 456, true)).id();
        let ents = world
            .query::<(Entity, &bool)>()
            .iter(&world)
            .map(|(e, &b)| (e, b))
            .collect::<Vec<_>>();
        assert_eq!(ents, &[(f, true)]);
    }

    #[test]
    fn query_filter_with() {
        let mut world = World::new();
        world.spawn().insert_bundle((123u32, 1.0f32));
        world.spawn().insert(456u32);
        let result = world
            .query_filtered::<&u32, With<f32>>()
            .iter(&world)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(result, vec![123]);
    }

    #[test]
    fn query_filter_with_for_each() {
        let mut world = World::new();
        world.spawn().insert_bundle((123u32, 1.0f32));
        world.spawn().insert(456u32);

        let mut results = Vec::new();
        world
            .query_filtered::<&u32, With<f32>>()
            .for_each(&world, |i| results.push(*i));
        assert_eq!(results, vec![123]);
    }

    #[test]
    fn query_filter_with_sparse() {
        let mut world = World::new();
        world
            .register_component(ComponentDescriptor::new::<f32>(StorageType::SparseSet))
            .unwrap();
        world.spawn().insert_bundle((123u32, 1.0f32));
        world.spawn().insert(456u32);
        let result = world
            .query_filtered::<&u32, With<f32>>()
            .iter(&world)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(result, vec![123]);
    }

    #[test]
    fn query_filter_with_sparse_for_each() {
        let mut world = World::new();
        world
            .register_component(ComponentDescriptor::new::<f32>(StorageType::SparseSet))
            .unwrap();
        world.spawn().insert_bundle((123u32, 1.0f32));
        world.spawn().insert(456u32);
        let mut results = Vec::new();
        world
            .query_filtered::<&u32, With<f32>>()
            .for_each(&world, |i| results.push(*i));
        assert_eq!(results, vec![123]);
    }

    #[test]
    fn query_filter_without() {
        let mut world = World::new();
        world.spawn().insert_bundle((123u32, 1.0f32));
        world.spawn().insert(456u32);
        let result = world
            .query_filtered::<&u32, Without<f32>>()
            .iter(&world)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(result, vec![456]);
    }

    #[test]
    fn query_optional_component_table() {
        let mut world = World::new();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        let f = world.spawn().insert_bundle(("def", 456, true)).id();
        // this should be skipped
        world.spawn().insert("abc");
        let ents = world
            .query::<(Entity, Option<&bool>, &i32)>()
            .iter(&world)
            .map(|(e, b, &i)| (e, b.copied(), i))
            .collect::<Vec<_>>();
        assert_eq!(ents, &[(e, None, 123), (f, Some(true), 456)]);
    }

    #[test]
    fn query_optional_component_sparse() {
        let mut world = World::new();
        world
            .register_component(ComponentDescriptor::new::<bool>(StorageType::SparseSet))
            .unwrap();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        let f = world.spawn().insert_bundle(("def", 456, true)).id();
        // // this should be skipped
        // world.spawn().insert("abc");
        let ents = world
            .query::<(Entity, Option<&bool>, &i32)>()
            .iter(&world)
            .map(|(e, b, &i)| (e, b.copied(), i))
            .collect::<Vec<_>>();
        assert_eq!(ents, &[(e, None, 123), (f, Some(true), 456)]);
    }

    #[test]
    fn query_optional_component_sparse_no_match() {
        let mut world = World::new();
        world
            .register_component(ComponentDescriptor::new::<bool>(StorageType::SparseSet))
            .unwrap();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        let f = world.spawn().insert_bundle(("def", 456)).id();
        // // this should be skipped
        world.spawn().insert("abc");
        let ents = world
            .query::<(Entity, Option<&bool>, &i32)>()
            .iter(&world)
            .map(|(e, b, &i)| (e, b.copied(), i))
            .collect::<Vec<_>>();
        assert_eq!(ents, &[(e, None, 123), (f, None, 456)]);
    }

    #[test]
    fn add_remove_components() {
        let mut world = World::new();
        let e1 = world.spawn().insert(42).insert_bundle((true, "abc")).id();
        let e2 = world.spawn().insert(0).insert_bundle((false, "xyz")).id();

        assert_eq!(
            world
                .query::<(Entity, &i32, &bool)>()
                .iter(&world)
                .map(|(e, &i, &b)| (e, i, b))
                .collect::<Vec<_>>(),
            &[(e1, 42, true), (e2, 0, false)]
        );

        assert_eq!(world.entity_mut(e1).remove::<i32>(), Some(42));
        assert_eq!(
            world
                .query::<(Entity, &i32, &bool)>()
                .iter(&world)
                .map(|(e, &i, &b)| (e, i, b))
                .collect::<Vec<_>>(),
            &[(e2, 0, false)]
        );
        assert_eq!(
            world
                .query::<(Entity, &bool, &&str)>()
                .iter(&world)
                .map(|(e, &b, &s)| (e, b, s))
                .collect::<Vec<_>>(),
            &[(e2, false, "xyz"), (e1, true, "abc")]
        );
        world.entity_mut(e1).insert(43);
        assert_eq!(
            world
                .query::<(Entity, &i32, &bool)>()
                .iter(&world)
                .map(|(e, &i, &b)| (e, i, b))
                .collect::<Vec<_>>(),
            &[(e2, 0, false), (e1, 43, true)]
        );
        world.entity_mut(e1).insert(1.0f32);
        assert_eq!(
            world
                .query::<(Entity, &f32)>()
                .iter(&world)
                .map(|(e, &f)| (e, f))
                .collect::<Vec<_>>(),
            &[(e1, 1.0)]
        );
    }

    #[test]
    fn table_add_remove_many() {
        let mut world = World::default();
        let mut entities = Vec::with_capacity(10_000);
        for _ in 0..1000 {
            entities.push(world.spawn().insert(0.0f32).id());
        }

        for (i, entity) in entities.iter().cloned().enumerate() {
            world.entity_mut(entity).insert(i);
        }

        for (i, entity) in entities.iter().cloned().enumerate() {
            assert_eq!(world.entity_mut(entity).remove::<usize>(), Some(i));
        }
    }

    #[test]
    fn sparse_set_add_remove_many() {
        let mut world = World::default();
        world
            .register_component(ComponentDescriptor::new::<usize>(StorageType::SparseSet))
            .unwrap();
        let mut entities = Vec::with_capacity(1000);
        for _ in 0..4 {
            entities.push(world.spawn().insert(0.0f32).id());
        }

        for (i, entity) in entities.iter().cloned().enumerate() {
            world.entity_mut(entity).insert(i);
        }

        for (i, entity) in entities.iter().cloned().enumerate() {
            assert_eq!(world.entity_mut(entity).remove::<usize>(), Some(i));
        }
    }

    #[test]
    fn remove_missing() {
        let mut world = World::new();
        let e = world.spawn().insert_bundle(("abc", 123)).id();
        assert!(world.entity_mut(e).remove::<bool>().is_none());
    }

    #[test]
    fn spawn_batch() {
        let mut world = World::new();
        world.spawn_batch((0..100).map(|x| (x, "abc")));
        let values = world
            .query::<&i32>()
            .iter(&world)
            .copied()
            .collect::<Vec<_>>();
        let expected = (0..100).collect::<Vec<_>>();
        assert_eq!(values, expected);
    }

    #[test]
    fn query_get() {
        let mut world = World::new();
        let a = world.spawn().insert_bundle(("abc", 123)).id();
        let b = world.spawn().insert_bundle(("def", 456)).id();
        let c = world.spawn().insert_bundle(("ghi", 789, true)).id();

        let mut i32_query = world.query::<&i32>();
        assert_eq!(i32_query.get(&world, a).unwrap(), &123);
        assert_eq!(i32_query.get(&world, b).unwrap(), &456);

        let mut i32_bool_query = world.query::<(&i32, &bool)>();
        assert!(i32_bool_query.get(&world, a).is_err());
        assert_eq!(i32_bool_query.get(&world, c).unwrap(), (&789, &true));
        assert!(world.despawn(a));
        assert!(i32_query.get(&world, a).is_err());
    }

    #[test]
    fn remove_tracking() {
        let mut world = World::new();
        world
            .register_component(ComponentDescriptor::new::<&'static str>(
                StorageType::SparseSet,
            ))
            .unwrap();
        let a = world.spawn().insert_bundle(("abc", 123)).id();
        let b = world.spawn().insert_bundle(("abc", 123)).id();

        world.entity_mut(a).despawn();
        assert_eq!(
            world.removed::<i32>().collect::<Vec<_>>(),
            &[a],
            "despawning results in 'removed component' state for table components"
        );
        assert_eq!(
            world.removed::<&'static str>().collect::<Vec<_>>(),
            &[a],
            "despawning results in 'removed component' state for sparse set components"
        );

        world.entity_mut(b).insert(10.0);
        assert_eq!(
            world.removed::<i32>().collect::<Vec<_>>(),
            &[a],
            "archetype moves does not result in 'removed component' state"
        );

        world.entity_mut(b).remove::<i32>();
        assert_eq!(
            world.removed::<i32>().collect::<Vec<_>>(),
            &[a, b],
            "removing a component results in a 'removed component' state"
        );

        world.clear_trackers();
        assert_eq!(
            world.removed::<i32>().collect::<Vec<_>>(),
            &[],
            "clearning trackers clears removals"
        );
        assert_eq!(
            world.removed::<&'static str>().collect::<Vec<_>>(),
            &[],
            "clearning trackers clears removals"
        );
        assert_eq!(
            world.removed::<f64>().collect::<Vec<_>>(),
            &[],
            "clearning trackers clears removals"
        );

        // TODO: uncomment when world.clear() is implemented
        // let c = world.spawn().insert_bundle(("abc", 123)).id();
        // let d = world.spawn().insert_bundle(("abc", 123)).id();
        // world.clear();
        // assert_eq!(
        //     world.removed::<i32>(),
        //     &[c, d],
        //     "world clears result in 'removed component' states"
        // );
        // assert_eq!(
        //     world.removed::<&'static str>(),
        //     &[c, d, b],
        //     "world clears result in 'removed component' states"
        // );
        // assert_eq!(
        //     world.removed::<f64>(),
        //     &[b],
        //     "world clears result in 'removed component' states"
        // );
    }

    #[test]
    fn added_tracking() {
        let mut world = World::new();
        let a = world.spawn().insert(123i32).id();

        assert_eq!(world.query::<&i32>().iter(&world).count(), 1);
        assert_eq!(
            world
                .query_filtered::<(), Added<i32>>()
                .iter(&world)
                .count(),
            1
        );
        assert_eq!(world.query::<&i32>().iter(&world).count(), 1);
        assert_eq!(
            world
                .query_filtered::<(), Added<i32>>()
                .iter(&world)
                .count(),
            1
        );
        assert!(world.query::<&i32>().get(&world, a).is_ok());
        assert!(world
            .query_filtered::<(), Added<i32>>()
            .get(&world, a)
            .is_ok());
        assert!(world.query::<&i32>().get(&world, a).is_ok());
        assert!(world
            .query_filtered::<(), Added<i32>>()
            .get(&world, a)
            .is_ok());

        world.clear_trackers();

        assert_eq!(world.query::<&i32>().iter(&world).count(), 1);
        assert_eq!(
            world
                .query_filtered::<(), Added<i32>>()
                .iter(&world)
                .count(),
            0
        );
        assert_eq!(world.query::<&i32>().iter(&world).count(), 1);
        assert_eq!(
            world
                .query_filtered::<(), Added<i32>>()
                .iter(&world)
                .count(),
            0
        );
        assert!(world.query::<&i32>().get(&world, a).is_ok());
        assert!(world
            .query_filtered::<(), Added<i32>>()
            .get(&world, a)
            .is_err());
        assert!(world.query::<&i32>().get(&world, a).is_ok());
        assert!(world
            .query_filtered::<(), Added<i32>>()
            .get(&world, a)
            .is_err());
    }

    #[test]
    fn added_queries() {
        let mut world = World::default();
        let e1 = world.spawn().insert(A(0)).id();

        fn get_added<Com: Component>(world: &mut World) -> Vec<Entity> {
            world
                .query_filtered::<Entity, Added<Com>>()
                .iter(&world)
                .collect::<Vec<Entity>>()
        }

        assert_eq!(get_added::<A>(&mut world), vec![e1]);
        world.entity_mut(e1).insert(B(0));
        assert_eq!(get_added::<A>(&mut world), vec![e1]);
        assert_eq!(get_added::<B>(&mut world), vec![e1]);

        world.clear_trackers();
        assert!(get_added::<A>(&mut world).is_empty());
        let e2 = world.spawn().insert_bundle((A(1), B(1))).id();
        assert_eq!(get_added::<A>(&mut world), vec![e2]);
        assert_eq!(get_added::<B>(&mut world), vec![e2]);

        let added = world
            .query_filtered::<Entity, (Added<A>, Added<B>)>()
            .iter(&world)
            .collect::<Vec<Entity>>();
        assert_eq!(added, vec![e2]);
    }

    #[test]
    fn changed_trackers() {
        let mut world = World::default();
        let e1 = world.spawn().insert_bundle((A(0), B(0))).id();
        let e2 = world.spawn().insert_bundle((A(0), B(0))).id();
        let e3 = world.spawn().insert_bundle((A(0), B(0))).id();
        world.spawn().insert_bundle((A(0), B));

        world.clear_trackers();

        for (i, mut a) in world.query::<&mut A>().iter_mut(&mut world).enumerate() {
            if i % 2 == 0 {
                a.0 += 1;
            }
        }

        fn get_filtered<F: WorldQuery>(world: &mut World) -> Vec<Entity>
        where
            F::Fetch: FilterFetch,
        {
            world
                .query_filtered::<Entity, F>()
                .iter(&world)
                .collect::<Vec<Entity>>()
        }

        assert_eq!(get_filtered::<Changed<A>>(&mut world), vec![e1, e3]);

        // ensure changing an entity's archetypes also moves its changed state
        world.entity_mut(e1).insert(C);

        assert_eq!(get_filtered::<Changed<A>>(&mut world), vec![e3, e1], "changed entities list should not change (although the order will due to archetype moves)");

        // spawning a new A entity should not change existing changed state
        world.entity_mut(e1).insert_bundle((A(0), B));
        assert_eq!(
            get_filtered::<Changed<A>>(&mut world),
            vec![e3, e1],
            "changed entities list should not change"
        );

        // removing an unchanged entity should not change changed state
        assert!(world.despawn(e2));
        assert_eq!(
            get_filtered::<Changed<A>>(&mut world),
            vec![e3, e1],
            "changed entities list should not change"
        );

        // removing a changed entity should remove it from enumeration
        assert!(world.despawn(e1));
        assert_eq!(
            get_filtered::<Changed<A>>(&mut world),
            vec![e3],
            "e1 should no longer be returned"
        );

        world.clear_trackers();

        assert!(get_filtered::<Changed<A>>(&mut world).is_empty());

        let e4 = world.spawn().id();

        world.entity_mut(e4).insert(A(0));
        assert_eq!(get_filtered::<Changed<A>>(&mut world), vec![e4]);
        assert_eq!(get_filtered::<Added<A>>(&mut world), vec![e4]);

        world.entity_mut(e4).insert(A(1));
        assert_eq!(get_filtered::<Changed<A>>(&mut world), vec![e4]);

        world.clear_trackers();

        // ensure inserting multiple components set changed state for all components and set added
        // state for non existing components even when changing archetype.
        world.entity_mut(e4).insert_bundle((A(0), B(0)));

        assert!(get_filtered::<Added<A>>(&mut world).is_empty());
        assert_eq!(get_filtered::<Changed<A>>(&mut world), vec![e4]);
        assert_eq!(get_filtered::<Added<B>>(&mut world), vec![e4]);
        assert_eq!(get_filtered::<Changed<B>>(&mut world), vec![e4]);
    }

    #[test]
    fn empty_spawn() {
        let mut world = World::default();
        let e = world.spawn().id();
        let mut e_mut = world.entity_mut(e);
        e_mut.insert(A(0));
        assert_eq!(e_mut.get::<A>().unwrap(), &A(0));
    }

    #[test]
    fn reserve_and_spawn() {
        let mut world = World::default();
        let e = world.entities().reserve_entity();
        world.flush();
        let mut e_mut = world.entity_mut(e);
        e_mut.insert(A(0));
        assert_eq!(e_mut.get::<A>().unwrap(), &A(0));
    }

    #[test]
    fn changed_query() {
        let mut world = World::default();
        let e1 = world.spawn().insert_bundle((A(0), B(0))).id();

        fn get_changed(world: &mut World) -> Vec<Entity> {
            world
                .query_filtered::<Entity, Changed<A>>()
                .iter(&world)
                .collect::<Vec<Entity>>()
        }
        assert_eq!(get_changed(&mut world), vec![e1]);
        world.clear_trackers();
        assert_eq!(get_changed(&mut world), vec![]);
        *world.get_mut(e1).unwrap() = A(1);
        assert_eq!(get_changed(&mut world), vec![e1]);
    }

    #[test]
    fn resource() {
        let mut world = World::default();
        assert!(world.get_resource::<i32>().is_none());
        assert!(!world.contains_resource::<i32>());

        world.insert_resource(123);
        let resource_id = world
            .components()
            .get_resource_id(TypeId::of::<i32>())
            .unwrap();
        let archetype_component_id = world
            .archetypes()
            .resource()
            .get_archetype_component_id(resource_id)
            .unwrap();

        assert_eq!(*world.get_resource::<i32>().expect("resource exists"), 123);
        assert!(world.contains_resource::<i32>());

        world.insert_resource(456u64);
        assert_eq!(
            *world.get_resource::<u64>().expect("resource exists"),
            456u64
        );

        world.insert_resource(789u64);
        assert_eq!(*world.get_resource::<u64>().expect("resource exists"), 789);

        {
            let mut value = world.get_resource_mut::<u64>().expect("resource exists");
            assert_eq!(*value, 789);
            *value = 10;
        }

        assert_eq!(
            world.get_resource::<u64>(),
            Some(&10),
            "resource changes are preserved"
        );

        assert_eq!(
            world.remove_resource::<u64>(),
            Some(10),
            "removed resource has the correct value"
        );
        assert_eq!(
            world.get_resource::<u64>(),
            None,
            "removed resource no longer exists"
        );
        assert_eq!(
            world.remove_resource::<u64>(),
            None,
            "double remove returns nothing"
        );

        world.insert_resource(1u64);
        assert_eq!(
            world.get_resource::<u64>(),
            Some(&1u64),
            "re-inserting resources works"
        );

        assert_eq!(
            world.get_resource::<i32>(),
            Some(&123),
            "other resources are unaffected"
        );

        let current_resource_id = world
            .components()
            .get_resource_id(TypeId::of::<i32>())
            .unwrap();
        assert_eq!(
            resource_id, current_resource_id,
            "resource id does not change after removing / re-adding"
        );

        let current_archetype_component_id = world
            .archetypes()
            .resource()
            .get_archetype_component_id(current_resource_id)
            .unwrap();

        assert_eq!(
            archetype_component_id, current_archetype_component_id,
            "resource archetype component id does not change after removing / re-adding"
        );
    }

    #[test]
    fn remove_intersection() {
        let mut world = World::default();
        let e1 = world.spawn().insert_bundle((1, 1.0, "a")).id();

        let mut e = world.entity_mut(e1);
        assert_eq!(e.get::<&'static str>(), Some(&"a"));
        assert_eq!(e.get::<i32>(), Some(&1));
        assert_eq!(e.get::<f64>(), Some(&1.0));
        assert_eq!(
            e.get::<usize>(),
            None,
            "usize is not in the entity, so it should not exist"
        );

        e.remove_bundle_intersection::<(i32, f64, usize)>();
        assert_eq!(
            e.get::<&'static str>(),
            Some(&"a"),
            "&'static str is not in the removed bundle, so it should exist"
        );
        assert_eq!(
            e.get::<i32>(),
            None,
            "i32 is in the removed bundle, so it should not exist"
        );
        assert_eq!(
            e.get::<f64>(),
            None,
            "f64 is in the removed bundle, so it should not exist"
        );
        assert_eq!(
            e.get::<usize>(),
            None,
            "usize is in the removed bundle, so it should not exist"
        );
    }

    #[test]
    fn remove_bundle() {
        let mut world = World::default();
        world.spawn().insert_bundle((1, 1.0, 1usize)).id();
        let e2 = world.spawn().insert_bundle((2, 2.0, 2usize)).id();
        world.spawn().insert_bundle((3, 3.0, 3usize)).id();

        let mut query = world.query::<(&f64, &usize)>();
        let results = query
            .iter(&world)
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results, vec![(1.0, 1usize), (2.0, 2usize), (3.0, 3usize),]);

        let removed_bundle = world
            .entity_mut(e2)
            .remove_bundle::<(f64, usize)>()
            .unwrap();
        assert_eq!(removed_bundle, (2.0, 2usize));

        let results = query
            .iter(&world)
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();
        assert_eq!(results, vec![(1.0, 1usize), (3.0, 3usize),]);

        let mut i32_query = world.query::<&i32>();
        let results = i32_query.iter(&world).cloned().collect::<Vec<_>>();
        assert_eq!(results, vec![1, 3, 2]);

        let entity_ref = world.entity(e2);
        assert_eq!(
            entity_ref.get::<i32>(),
            Some(&2),
            "i32 is not in the removed bundle, so it should exist"
        );
        assert_eq!(
            entity_ref.get::<f64>(),
            None,
            "f64 is in the removed bundle, so it should not exist"
        );
        assert_eq!(
            entity_ref.get::<usize>(),
            None,
            "usize is in the removed bundle, so it should not exist"
        );
    }

    #[test]
    fn non_send_resource() {
        let mut world = World::default();
        world.insert_non_send(123i32);
        world.insert_non_send(456i64);
        assert_eq!(*world.get_non_send_resource::<i32>().unwrap(), 123);
        assert_eq!(*world.get_non_send_resource_mut::<i64>().unwrap(), 456);
    }

    #[test]
    #[should_panic]
    fn non_send_resource_panic() {
        let mut world = World::default();
        world.insert_non_send(0i32);
        std::thread::spawn(move || {
            let _ = world.get_non_send_resource_mut::<i32>();
        })
        .join()
        .unwrap();
    }

    #[test]
    fn trackers_query() {
        let mut world = World::default();
        let e1 = world.spawn().insert_bundle((A(0), B(0))).id();
        world.spawn().insert(B(0));

        let mut trackers_query = world.query::<Option<ChangeTrackers<A>>>();
        let trackers = trackers_query.iter(&world).collect::<Vec<_>>();
        let a_trackers = trackers[0].as_ref().unwrap();
        assert!(trackers[1].is_none());
        assert!(a_trackers.is_added());
        assert!(a_trackers.is_changed());
        world.clear_trackers();
        let trackers = trackers_query.iter(&world).collect::<Vec<_>>();
        let a_trackers = trackers[0].as_ref().unwrap();
        assert!(!a_trackers.is_added());
        assert!(!a_trackers.is_changed());
        *world.get_mut(e1).unwrap() = A(1);
        let trackers = trackers_query.iter(&world).collect::<Vec<_>>();
        let a_trackers = trackers[0].as_ref().unwrap();
        assert!(!a_trackers.is_added());
        assert!(a_trackers.is_changed());
    }

    #[test]
    fn exact_size_query() {
        let mut world = World::default();
        world.spawn().insert_bundle((A(0), B(0)));
        world.spawn().insert_bundle((A(0), B(0)));
        world.spawn().insert_bundle((A(0), B(0), C));
        world.spawn().insert(C);

        let mut query = world.query::<(&A, &B)>();
        assert_eq!(query.iter(&world).len(), 3);
    }

    #[test]
    #[should_panic]
    fn duplicate_components_panic() {
        let mut world = World::new();
        world.spawn().insert_bundle((1, 2));
    }

    #[test]
    #[should_panic]
    fn ref_and_mut_query_panic() {
        let mut world = World::new();
        world.query::<(&A, &mut A)>();
    }

    #[test]
    #[should_panic]
    fn mut_and_ref_query_panic() {
        let mut world = World::new();
        world.query::<(&mut A, &A)>();
    }

    #[test]
    #[should_panic]
    fn mut_and_mut_query_panic() {
        let mut world = World::new();
        world.query::<(&mut A, &mut A)>();
    }

    #[test]
    #[should_panic]
    fn multiple_worlds_same_query_iter() {
        let mut world_a = World::new();
        let world_b = World::new();
        let mut query = world_a.query::<&i32>();
        query.iter(&world_a);
        query.iter(&world_b);
    }

    #[test]
    fn query_filters_dont_collide_with_fetches() {
        let mut world = World::new();
        world.query_filtered::<&mut i32, Changed<i32>>();
    }

    #[test]
    fn filtered_query_access() {
        let mut world = World::new();
        let query = world.query_filtered::<&mut i32, Changed<f64>>();

        let mut expected = FilteredAccess::<ComponentId>::default();
        let i32_id = world.components.get_id(TypeId::of::<i32>()).unwrap();
        let f64_id = world.components.get_id(TypeId::of::<f64>()).unwrap();
        expected.add_write(i32_id);
        expected.add_read(f64_id);
        assert!(
            query.component_access.eq(&expected),
            "ComponentId access from query fetch and query filter should be combined"
        );
    }

    #[test]
    #[should_panic]
    fn multiple_worlds_same_query_get() {
        let mut world_a = World::new();
        let world_b = World::new();
        let mut query = world_a.query::<&i32>();
        let _ = query.get(&world_a, Entity::new(0));
        let _ = query.get(&world_b, Entity::new(0));
    }

    #[test]
    #[should_panic]
    fn multiple_worlds_same_query_for_each() {
        let mut world_a = World::new();
        let world_b = World::new();
        let mut query = world_a.query::<&i32>();
        query.for_each(&world_a, |_| {});
        query.for_each(&world_b, |_| {});
    }

    #[test]
    fn resource_scope() {
        let mut world = World::default();
        world.insert_resource::<i32>(0);
        world.resource_scope(|world: &mut World, mut value: Mut<i32>| {
            *value += 1;
            assert!(!world.contains_resource::<i32>());
        });
        assert_eq!(*world.get_resource::<i32>().unwrap(), 1);
    }

    #[test]
    fn insert_overwrite_drop() {
        let (dropck1, dropped1) = DropCk::new_pair();
        let (dropck2, dropped2) = DropCk::new_pair();
        let mut world = World::default();
        world.spawn().insert(dropck1).insert(dropck2);
        assert_eq!(dropped1.load(Ordering::Relaxed), 1);
        assert_eq!(dropped2.load(Ordering::Relaxed), 0);
        drop(world);
        assert_eq!(dropped1.load(Ordering::Relaxed), 1);
        assert_eq!(dropped2.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn insert_overwrite_drop_sparse() {
        let (dropck1, dropped1) = DropCk::new_pair();
        let (dropck2, dropped2) = DropCk::new_pair();
        let mut world = World::default();
        world
            .register_component(ComponentDescriptor::new::<DropCk>(StorageType::SparseSet))
            .unwrap();
        world.spawn().insert(dropck1).insert(dropck2);
        assert_eq!(dropped1.load(Ordering::Relaxed), 1);
        assert_eq!(dropped2.load(Ordering::Relaxed), 0);
        drop(world);
        assert_eq!(dropped1.load(Ordering::Relaxed), 1);
        assert_eq!(dropped2.load(Ordering::Relaxed), 1);
    }
}
