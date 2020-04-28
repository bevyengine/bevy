use crate::{
    resource::{ResourceSet, ResourceTypeId},
    schedule::{ArchetypeAccess, Schedulable},
    Access, System, SystemAccess, SystemFnWrapper, SystemQuery,
};
use fxhash::FxHashMap;
use legion_core::{
    borrow::{AtomicRefCell},
    filter::EntityFilter,
    query::{DefaultFilter, IntoQuery, View},
    storage::{ComponentTypeId},
};
use std::marker::PhantomData;
use bit_set::BitSet;

pub fn into_system<'a, Q, F, R, X>(name: &'static str, mut system: F) -> Box<dyn Schedulable>
where
    Q: IntoQuery + DefaultFilter<Filter = R>,
    <Q as View<'a>>::Iter: Iterator<Item = Q> + 'a,
    F: FnMut(&mut X, Q) + Send + Sync + 'static,
    R: EntityFilter + Sync + 'static,
    X: ResourceSet<PreparedResources = X> + 'static,
{
    let mut resource_access: Access<ResourceTypeId> = Access::default();
    resource_access.reads.extend(X::read_types().iter());
    resource_access.writes.extend(X::write_types().iter());

    let mut component_access: Access<ComponentTypeId> = Access::default();
    component_access.reads.extend(Q::read_types().iter());
    component_access.writes.extend(Q::write_types().iter());

    let run_fn = SystemFnWrapper(
        move |_,
              world,
              resources: &mut X,
              query: &mut SystemQuery<Q, <Q as DefaultFilter>::Filter>| {
            for components in query.iter_mut(world) {
                system(resources, components);
            }
        },
        PhantomData,
    );

    Box::new(System {
        name: name.into(),
        queries: AtomicRefCell::new(Q::query()),
        access: SystemAccess {
            resources: resource_access,
            components: component_access,
            tags: Access::default(),
        },
        archetypes: ArchetypeAccess::Some(BitSet::default()),
        _resources: PhantomData::<X>,
        command_buffer: FxHashMap::default(),
        run_fn: AtomicRefCell::new(run_fn),
    })
}

pub fn into_resource_system<'a, F, X>(name: &'static str, mut system: F) -> Box<dyn Schedulable>
where
    F: FnMut(&mut X) + Send + Sync + 'static,
    X: ResourceSet<PreparedResources = X> + 'static,
{
    let mut resource_access: Access<ResourceTypeId> = Access::default();
    resource_access.reads.extend(X::read_types().iter());
    resource_access.writes.extend(X::write_types().iter());

    let component_access: Access<ComponentTypeId> = Access::default();
    let run_fn = SystemFnWrapper(
        move |_,
              _,
              resources: &mut X,
              _| {
            system(resources);
        },
        PhantomData,
    );

    Box::new(System {
        name: name.into(),
        queries: AtomicRefCell::new(()),
        access: SystemAccess {
            resources: resource_access,
            components: component_access,
            tags: Access::default(),
        },
        archetypes: ArchetypeAccess::Some(BitSet::default()),
        _resources: PhantomData::<X>,
        command_buffer: FxHashMap::default(),
        run_fn: AtomicRefCell::new(run_fn),
    })
}

#[cfg(test)]
mod tests {
    use super::into_system;
    use crate::{
        resource::{PreparedRead, PreparedWrite, Resources},
    };
    use legion_core::{
        borrow::{Ref, RefMut},
        world::World,
    };

    #[derive(Debug, Eq, PartialEq)]
    struct A(usize);
    #[derive(Debug, Eq, PartialEq)]
    struct B(usize);
    #[derive(Debug, Eq, PartialEq)]
    struct Y(usize);
    #[derive(Debug, Eq, PartialEq)]
    struct X(usize);

    #[test]
    fn test_system_fn() {
        fn read_write_system(_: &mut (), (_x, mut y): (Ref<X>, RefMut<Y>)) {
            y.0 += 1;
        }

        let mut world = World::new();
        let mut resources = Resources::default();
        world.insert((), vec![(X(1), Y(1)), (X(2), Y(2))]);

        let mut system = into_system("read_write", read_write_system);
        system.run(&mut world, &mut resources);
    }

    #[test]
    fn test_resource_system_fn() {
        fn my_system(
            (a, b): &mut (PreparedWrite<A>, PreparedRead<B>),
            (x, mut y): (Ref<X>, RefMut<Y>),
        ) {
            assert_eq!(**b, B(1));
            // assert_eq!(**b, B(0));
            if a.0 == 0 {
                assert_eq!(*x, X(2));
                assert_eq!(*y, Y(3));
            } else if a.0 == 1 {
                assert_eq!(*x, X(4));
                assert_eq!(*y, Y(5));
                y.0 += 1;
                assert_eq!(*y, Y(6));
            } else {
                panic!("unexpected value");
            }

            a.0 += 1;
        }

        let mut world = World::new();
        let mut resources = Resources::default();

        resources.insert(A(0));
        resources.insert(B(1));
        world.insert((), vec![(X(2), Y(3)), (X(4), Y(5))]);
        let mut my_system = into_system("read_resources", my_system);
        my_system.run(&mut world, &mut resources);
    }
}
