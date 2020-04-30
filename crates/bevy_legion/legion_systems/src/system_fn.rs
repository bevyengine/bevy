use crate::{
    resource::{ResourceSet, ResourceTypeId},
    schedule::{ArchetypeAccess, Schedulable},
    system_fn_types::{FuncSystem, FuncSystemFnWrapper},
    Access, SystemAccess, SystemQuery,
};
use bit_set::BitSet;
use fxhash::FxHashMap;
use legion_core::{
    borrow::AtomicRefCell,
    filter::{And, EntityFilter, EntityFilterTuple},
    query::{DefaultFilter, IntoQuery, View, ViewElement},
    storage::ComponentTypeId,
};
use std::marker::PhantomData;

pub trait IntoSystem<'a, ResourceArgs, ComponentArgs> {
    fn into_system(self, name: &'static str) -> Box<dyn Schedulable>;
}

macro_rules! impl_system {
    (($(($resource:ident, $resource_var:ident)),*), ($(($view:ident, $filter:ident, $view_var:ident)),*)) => {
        impl<'a,
        Func,
        $($resource: ResourceSet<PreparedResources = $resource> + 'static + Clone,)*
        $($view: for<'b> View<'b> + DefaultFilter<Filter = $filter> + ViewElement,
        $filter: EntityFilter + Sync + 'static),*
    > IntoSystem<'a, ($($resource,)*), ($($view,)*)> for Func
        where
            Func: FnMut($($resource,)* $($view),*) + Send + Sync + 'static,
            $(<$view as View<'a>>::Iter: Iterator<Item = $view>),*
        {
            fn into_system(mut self, name: &'static str) -> Box<dyn Schedulable> {
                let resource_access: Access<ResourceTypeId> = resource_access!(($($resource),*));
                let component_access: Access<ComponentTypeId> = component_access!(($($view),*));

                let run_fn = function_wrapper!(self, ($($resource, $resource_var),*), ($($view, $filter, $view_var),*));

                Box::new(FuncSystem {
                    name: name.into(),
                    queries: AtomicRefCell::new(query!($($view),*)),
                    access: SystemAccess {
                        resources: resource_access,
                        components: component_access,
                        tags: Access::default(),
                    },
                    archetypes: ArchetypeAccess::Some(BitSet::default()),
                    _resources: PhantomData::<tuple!($($resource),*)>,
                    command_buffer: FxHashMap::default(),
                    run_fn: AtomicRefCell::new(run_fn),
                })
            }
        }
    }
}

macro_rules! function_wrapper {
    ($me:ident, ($($resource:ident, $resource_var:ident),*), ($($view:ident, $filter:ident, $view_var:ident),*)) => {
        FuncSystemFnWrapper(
            move |_command_buffer,
                _world,
                _resources: tuple!($($resource),*),
                _query: &mut system_query!($($view, $filter),*)
            | {
                let tuple!($($resource_var),*) = _resources;
                run_function!($me, ($($resource, $resource_var),*), _world, _query, ($($view, $filter, $view_var),*))
            },
            PhantomData,
        )
    };
}

macro_rules! run_function {
    ($me:ident, ($($resource:ident, $resource_var:ident),*), $world:ident, $query:ident, ()) => {
        $me($($resource_var),*);
    };
    ($me:ident, ($($resource:ident, $resource_var:ident),*), $world:ident, $query:ident, ($($view:ident, $filter:ident, $view_var:ident),+)) => {
        for tuple!($($view_var),*) in $query.iter_mut($world) {
            $me($($resource_var.clone(),)* $($view_var),*);
        }
    }
}

macro_rules! tuple {
    () => { () };
    // single value: v1
    ($value:ident) => { $value };
    // multiple values: (v1, v2, v3)
    ($($value:ident),+) => { ($($value),+) }
}

macro_rules! component_access {
    (()) => {Access::default()};
    (($($view:ident),+)) => {{
        let mut component_access: Access<ComponentTypeId> = Access::default();
        component_access
            .reads
            .extend(<tuple!($($view),+) as View>::read_types().iter());
        component_access
            .writes
            .extend(<tuple!($($view),+) as View>::write_types().iter());
        component_access
    }}
}

macro_rules! resource_access {
    (()) => {Access::default()};
    (($($resource:ident),+)) => {{
        let mut component_access: Access<ResourceTypeId> = Access::default();
        component_access
            .reads
            .extend(<tuple!($($resource),+) as ResourceSet>::read_types().iter());
        component_access
            .writes
            .extend(<tuple!($($resource),+) as ResourceSet>::write_types().iter());
        component_access
    }}
}

macro_rules! system_query {
    () => {
        ()
    };
    ($view:ident, $filter:ident) => {
        SystemQuery<
        $view,
        $filter
    >
    };
    ($($view:ident, $filter:ident),+) => {
        SystemQuery<
            ($($view),+),
            EntityFilterTuple<
                And<(
                    $(<$filter as EntityFilter>::ArchetypeFilter),+
                )>,
                And<(
                    $(<$filter as EntityFilter>::ChunksetFilter),+
                )>,
                And<(
                    $(<$filter as EntityFilter>::ChunkFilter),+
                )>,
            >
        >
    }
}

macro_rules! query {
    () => { () };
    ($($query:ident),+) => {
        <tuple!($($query),+)>::query()
    }
}

macro_rules! impl_system_variants {
   ($(($resource:ident, $resource_var:ident)),*) => {
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ()];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1))];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1), (V2, V2F, v2))];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1), (V2, V2F, v2), (V3, V3F, v3))];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1), (V2, V2F, v2), (V3, V3F, v3), (V4, V4F, v4))];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1), (V2, V2F, v2), (V3, V3F, v3), (V4, V4F, v4), (V5, V5F, v5))];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1), (V2, V2F, v2), (V3, V3F, v3), (V4, V4F, v4), (V5, V5F, v5), (V6, V6F, v6))];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1), (V2, V2F, v2), (V3, V3F, v3), (V4, V4F, v4), (V5, V5F, v5), (V6, V6F, v6), (V7, V7F, v7))];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1), (V2, V2F, v2), (V3, V3F, v3), (V4, V4F, v4), (V5, V5F, v5), (V6, V6F, v6), (V7, V7F, v7), (V8, V8F, v8))];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1), (V2, V2F, v2), (V3, V3F, v3), (V4, V4F, v4), (V5, V5F, v5), (V6, V6F, v6), (V7, V7F, v7), (V8, V8F, v8), (V9, V9F, v9))];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1), (V2, V2F, v2), (V3, V3F, v3), (V4, V4F, v4), (V5, V5F, v5), (V6, V6F, v6), (V7, V7F, v7), (V8, V8F, v8), (V9, V9F, v9), (V10, V10F, v10))];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1), (V2, V2F, v2), (V3, V3F, v3), (V4, V4F, v4), (V5, V5F, v5), (V6, V6F, v6), (V7, V7F, v7), (V8, V8F, v8), (V9, V9F, v9), (V10, V10F, v10), (V11, V11F, v11))];
        #[rustfmt::skip]
        impl_system![($(($resource, $resource_var)),*), ((V1, V1F, v1), (V2, V2F, v2), (V3, V3F, v3), (V4, V4F, v4), (V5, V5F, v5), (V6, V6F, v6), (V7, V7F, v7), (V8, V8F, v8), (V9, V9F, v9), (V10, V10F, v10), (V11, V11F, v11), (V12, V12F, v12))];

   }
}

#[rustfmt::skip]
impl_system_variants![];
#[rustfmt::skip]
impl_system_variants![(R1, r1)];
#[rustfmt::skip]
impl_system_variants![(R1, r1), (R2, r2)];
#[rustfmt::skip]
impl_system_variants![(R1, r1), (R2, r2), (R3, r3)];
#[rustfmt::skip]
impl_system_variants![(R1, r1), (R2, r2), (R3, r3), (R4, r4)];
#[rustfmt::skip]
impl_system_variants![(R1, r1), (R2, r2), (R3, r3), (R4, r4), (R5, r5)];
#[rustfmt::skip]
impl_system_variants![(R1, r1), (R2, r2), (R3, r3), (R4, r4), (R5, r5), (R6, r6)];
#[rustfmt::skip]
impl_system_variants![(R1, r1), (R2, r2), (R3, r3), (R4, r4), (R5, r5), (R6, r6), (R7, r7)];
#[rustfmt::skip]
impl_system_variants![(R1, r1), (R2, r2), (R3, r3), (R4, r4), (R5, r5), (R6, r6), (R7, r7), (R8, r8)];
#[rustfmt::skip]
impl_system_variants![(R1, r1), (R2, r2), (R3, r3), (R4, r4), (R5, r5), (R6, r6), (R7, r7), (R8, r8), (R9, r9)];
#[rustfmt::skip]
impl_system_variants![(R1, r1), (R2, r2), (R3, r3), (R4, r4), (R5, r5), (R6, r6), (R7, r7), (R8, r8), (R9, r9), (R10, r10)];
#[rustfmt::skip]
impl_system_variants![(R1, r1), (R2, r2), (R3, r3), (R4, r4), (R5, r5), (R6, r6), (R7, r7), (R8, r8), (R9, r9), (R10, r10), (R11, r11)];
#[rustfmt::skip]
impl_system_variants![(R1, r1), (R2, r2), (R3, r3), (R4, r4), (R5, r5), (R6, r6), (R7, r7), (R8, r8), (R9, r9), (R10, r10), (R11, r11), (R12, r12)];

#[cfg(test)]
mod tests {
    use crate::{
        resource::Resources,
        system_fn_types::{Resource, ResourceMut},
        IntoSystem,
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
    fn test_into_system() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(A(0));
        world.insert((), vec![(X(1), Y(1)), (X(2), Y(2))]);

        fn single_read_system(x: Ref<X>) {
            println!("{}", x.0);
        }
        let mut system = single_read_system.into_system("hi");
        system.run(&mut world, &mut resources);

        fn read_write_system(x: Ref<X>, y: Ref<Y>, mut z: RefMut<A>) {
            z.0 += 1;
            println!("{} {} {}", x.0, y.0, z.0);
        }

        ({
            |x: Resource<A>, y: Ref<Y>, mut z: RefMut<A>| {
                z.0 += 1;
                println!("{} {} {}", x.0, y.0, z.0);
            }
        })
        .into_system("bleh");

        let mut system = read_write_system.into_system("read_write");
        system.run(&mut world, &mut resources);

        fn resource_system(a: Resource<A>, x: Ref<X>, y: Ref<Y>) {
            println!("{} {} {}", a.0, x.0, y.0);
        }

        let mut system = resource_system.into_system("hi");
        system.run(&mut world, &mut resources);

        fn empty_system_mut() {
            println!("hello world");
        }

        let mut system = empty_system_mut.into_system("hi");
        system.run(&mut world, &mut resources);

        fn resource_system_mut(mut a: ResourceMut<A>, x: Ref<X>, y: Ref<Y>) {
            let hi = &mut a;
            a.0 += 1;
            println!("{} {} {}", a.0, x.0, y.0);
        }
        let mut system = resource_system_mut.into_system("hi");
        system.run(&mut world, &mut resources);
    }

    #[test]
    fn test_resource_system_fn() {
        fn my_system(mut a: ResourceMut<A>, x: Ref<X>, mut y: RefMut<Y>) {
            assert_eq!(*a, A(1));
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
        let mut my_system = my_system.into_system("my_system");
        my_system.run(&mut world, &mut resources);
    }
}
