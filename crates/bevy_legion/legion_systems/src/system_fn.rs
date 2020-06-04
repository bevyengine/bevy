use crate::{
    resource::{ResourceSet, ResourceTypeId},
    schedule::{ArchetypeAccess, Schedulable},
    system_fn_types::{FuncSystem, FuncSystemFnWrapper},
    Access, SubWorld, SystemAccess, SystemId, SystemQuery,
};
use bit_set::BitSet;
use fxhash::FxHashMap;
use legion_core::{
    borrow::AtomicRefCell,
    command::CommandBuffer,
    filter::{And, EntityFilter, EntityFilterTuple},
    query::{DefaultFilter, IntoQuery, View, ViewElement},
    storage::ComponentTypeId,
};
use legion_fn_system_macro::{impl_fn_query_systems, impl_fn_systems};
use std::marker::PhantomData;

pub trait IntoSystem<CommandBuffer, Resources, Views, Queries, Filters> {
    fn system_id(self, id: SystemId) -> Box<dyn Schedulable>;
    fn system_named(self, name: &'static str) -> Box<dyn Schedulable>;
    fn system(self) -> Box<dyn Schedulable>;
}

impl_fn_systems!();
impl_fn_query_systems!();

#[allow(type_alias_bounds)]
pub type Query<V>
where
    V: for<'a> View<'a> + DefaultFilter,
= SystemQuery<V, <V as DefaultFilter>::Filter>;

#[cfg(test)]
mod tests {
    use crate::{
        resource::Resources,
        system_fn_types::{Res, ResMut},
        IntoSystem, Query, SubWorld,
    };
    use legion_core::{
        borrow::{Ref, RefMut},
        command::CommandBuffer,
        query::{Read, Write},
        world::World,
    };
    use std::fmt::Debug;

    #[derive(Debug, Eq, PartialEq)]
    struct A(usize);
    #[derive(Debug, Eq, PartialEq)]
    struct B(usize);
    #[derive(Debug, Eq, PartialEq)]
    struct Y(usize);
    #[derive(Debug, Eq, PartialEq)]
    struct X(usize);

    #[test]
    fn test_query_system() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(A(0));
        world.insert((), vec![(X(1), Y(1)), (X(2), Y(2))]);

        fn query_system(world: &mut SubWorld, query: &mut Query<(Read<X>, Write<Y>)>) {
            for (x, mut y) in query.iter_mut(world) {
                y.0 = 2;
                println!("{:?}", x);
            }
        }

        fn query_system2(
            world: &mut SubWorld,
            a: Res<A>,
            query: &mut Query<(Read<X>, Write<Y>)>,
            query2: &mut Query<Read<X>>,
        ) {
            println!("{:?}", *a);
            for (x, mut y) in query.iter_mut(world) {
                y.0 = 2;
                println!("{:?}", x);
            }

            for x in query2.iter(world) {
                println!("{:?}", x);
            }
        }

        let mut system = query_system.system();
        let mut system2 = query_system2.system();
        system.run(&mut world, &mut resources);
        system2.run(&mut world, &mut resources);
    }

    #[test]
    fn test_into_system() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(A(0));
        world.insert((), vec![(X(1), Y(1)), (X(2), Y(2))]);

        fn single_read_system(x: Ref<X>) {
            println!("{}", x.0);
        }
        let mut system = single_read_system.system();
        system.run(&mut world, &mut resources);

        fn read_write_system(x: Ref<X>, y: Ref<Y>, mut z: RefMut<A>) {
            z.0 += 1;
            println!("{} {} {}", x.0, y.0, z.0);
        }

        ({
            |x: Res<A>, y: Ref<Y>, mut z: RefMut<A>| {
                z.0 += 1;
                println!("{} {} {}", x.0, y.0, z.0);
            }
        })
        .system();

        let mut system = read_write_system.system();
        system.run(&mut world, &mut resources);

        fn resource_system(a: Res<A>, x: Ref<X>, y: Ref<Y>) {
            println!("{} {} {}", a.0, x.0, y.0);
        }

        let mut system = resource_system.system();
        system.run(&mut world, &mut resources);

        fn empty_system_mut() {
            println!("hello world");
        }

        let mut system = empty_system_mut.system();
        system.run(&mut world, &mut resources);

        fn resource_system_mut(mut a: ResMut<A>, x: Ref<X>, y: Ref<Y>) {
            a.0 += 1;
            println!("{} {} {}", a.0, x.0, y.0);
        }
        let mut system = resource_system_mut.system();
        system.run(&mut world, &mut resources);

        fn command_buffer_system(command_buffer: &mut CommandBuffer, mut a: ResMut<A>) {
            a.0 += 1;
            command_buffer.insert((), vec![(X(1), Y(1)), (X(2), Y(2))]);
            println!("{}", a.0);
        }
        let mut system = command_buffer_system.system();
        system.run(&mut world, &mut resources);
    }

    #[test]
    fn test_resource_system_fn() {
        fn my_system(mut a: ResMut<A>, x: Ref<X>, mut y: RefMut<Y>) {
            if a.0 == 0 {
                assert_eq!(*a, A(0));
                assert_eq!(*x, X(2));
                assert_eq!(*y, Y(3));
            } else if a.0 == 1 {
                assert_eq!(*a, A(1));
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
        let mut my_system = my_system.system();
        my_system.run(&mut world, &mut resources);
    }
}
