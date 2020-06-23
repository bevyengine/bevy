use crate::{
    resource::{ResourceSet, ResourceTypeId},
    schedule::Schedulable,
    system_fn_types::{FuncSystem, FuncSystemFnWrapper},
    SystemAccess, SystemId,
};
// use bit_set::BitSet;
use fxhash::FxHashMap;
use legion_core::{
    borrow::AtomicRefCell,
    command::CommandBuffer,
    permission::Permissions,
    query::Query,
    query::{DefaultFilter, IntoQuery, View, ViewElement},
    storage::ComponentTypeId,
    subworld::{ArchetypeAccess, SubWorld},
};
use legion_fn_system_macro::impl_fn_query_systems;
use std::marker::PhantomData;

pub trait IntoSystem<CommandBuffer, Resources, Queries> {
    fn system_id(self, id: SystemId) -> Box<dyn Schedulable>;
    fn system_named(self, name: &'static str) -> Box<dyn Schedulable>;
    fn system(self) -> Box<dyn Schedulable>;
}

impl_fn_query_systems!();

#[allow(type_alias_bounds)]
pub type SimpleQuery<V>
where
    V: for<'a> View<'a> + DefaultFilter,
= legion_core::query::Query<V, <V as DefaultFilter>::Filter>;

#[cfg(test)]
mod tests {
    use crate::{resource::Resources, system_fn_types::Res, IntoSystem, SimpleQuery};
    use legion_core::{
        query::{Read, Write},
        subworld::SubWorld,
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

        fn query_system(world: &mut SubWorld, query: &mut SimpleQuery<(Read<X>, Write<Y>)>) {
            for (x, mut y) in query.iter_mut(world) {
                y.0 = 2;
                println!("{:?}", x);
            }
        }
        let mut system = query_system.system();
        system.run(&mut world, &mut resources);

        fn query_system2(
            a: Res<A>,
            world: &mut SubWorld,
            query: &mut SimpleQuery<(Read<X>, Write<Y>)>,
            query2: &mut SimpleQuery<Read<X>>,
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

        let mut system2 = query_system2.system();
        system2.run(&mut world, &mut resources);

        fn query_system3(a: Res<A>) {
            println!("{:?}", *a);
        }

        let mut system3 = query_system3.system();
        system3.run(&mut world, &mut resources);
    }
}
