use crate::{
    resource::{ResourceSet, ResourceTypeId},
    schedule::{ArchetypeAccess, Schedulable},
    Access, SystemAccess, SystemQuery, system_fn_types::{FuncSystem, FuncSystemFnWrapper},
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

pub fn into_resource_for_each_system<'a, Q, F, R, X>(
    name: &'static str,
    mut system: F,
) -> Box<dyn Schedulable>
where
    Q: IntoQuery + DefaultFilter<Filter = R>,
    <Q as View<'a>>::Iter: Iterator<Item = Q> + 'a,
    F: FnMut(X, Q) + Send + Sync + 'static,
    R: EntityFilter + Sync + 'static,
    X: ResourceSet<PreparedResources = X> + 'static + Clone,
{
    let mut resource_access: Access<ResourceTypeId> = Access::default();
    resource_access.reads.extend(X::read_types().iter());
    resource_access.writes.extend(X::write_types().iter());

    let mut component_access: Access<ComponentTypeId> = Access::default();
    component_access.reads.extend(Q::read_types().iter());
    component_access.writes.extend(Q::write_types().iter());

    let run_fn = FuncSystemFnWrapper(
        move |_,
              world,
              resources: X,
              query: &mut SystemQuery<Q, <Q as DefaultFilter>::Filter>| {
            for components in query.iter_mut(world) {
                system(resources.clone(), components);
            }
        },
        PhantomData,
    );

    Box::new(FuncSystem {
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
    F: FnMut(X) + Send + Sync + 'static,
    X: ResourceSet<PreparedResources = X> + 'static,
{
    let mut resource_access: Access<ResourceTypeId> = Access::default();
    resource_access.reads.extend(X::read_types().iter());
    resource_access.writes.extend(X::write_types().iter());

    let component_access: Access<ComponentTypeId> = Access::default();
    let run_fn = FuncSystemFnWrapper(
        move |_, _, resources: X, _| {
            system(resources);
        },
        PhantomData,
    );

    Box::new(FuncSystem {
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

pub trait IntoSystem<'a, ResourceArgs, ComponentArgs>
where
    ComponentArgs: IntoQuery + DefaultFilter,
{
    fn into_system(self, name: &'static str) -> Box<dyn Schedulable>;
}

impl<
        'a,
        F,
        X: ResourceSet<PreparedResources = X> + 'static + Clone,
        A: for<'b> View<'b> + DefaultFilter<Filter = AF> + ViewElement,
        AF: EntityFilter + Sync + 'static,
        B: for<'b> View<'b> + DefaultFilter<Filter = BF> + ViewElement,
        BF: EntityFilter + Sync + 'static,
    > IntoSystem<'a, (X,), (A, B)> for F
where
    F: FnMut(X, A, B) + Send + Sync + 'static,
    <A as View<'a>>::Iter: Iterator<Item = A>,
    <B as View<'a>>::Iter: Iterator<Item = B>,
{
    fn into_system(mut self, name: &'static str) -> Box<dyn Schedulable> {
        let mut resource_access: Access<ResourceTypeId> = Access::default();
        resource_access
            .reads
            .extend(<X>::read_types().iter());
        resource_access
            .writes
            .extend(<X>::write_types().iter());
        let mut component_access: Access<ComponentTypeId> = Access::default();
        component_access
            .reads
            .extend(<(A, B) as View>::read_types().iter());
        component_access
            .writes
            .extend(<(A, B) as View>::write_types().iter());

        let run_fn = FuncSystemFnWrapper(
            move |_,
                  world,
                  x: X::PreparedResources,
                  query: &mut SystemQuery<
                (A, B),
                EntityFilterTuple<
                    And<(
                        <AF as EntityFilter>::ArchetypeFilter,
                        <BF as EntityFilter>::ArchetypeFilter,
                    )>,
                    And<(
                        <AF as EntityFilter>::ChunksetFilter,
                        <BF as EntityFilter>::ChunksetFilter,
                    )>,
                    And<(
                        <AF as EntityFilter>::ChunkFilter,
                        <BF as EntityFilter>::ChunkFilter,
                    )>,
                >,
            >| {
                for (a, b) in query.iter_mut(world) {
                    self(x.clone(), a, b);
                }
            },
            PhantomData,
        );

        Box::new(FuncSystem {
            name: name.into(),
            queries: AtomicRefCell::new(<(A, B)>::query()),
            access: SystemAccess {
                resources: resource_access,
                components: component_access,
                tags: Access::default(),
            },
            archetypes: ArchetypeAccess::Some(BitSet::default()),
            _resources: PhantomData::<X::PreparedResources>,
            command_buffer: FxHashMap::default(),
            run_fn: AtomicRefCell::new(run_fn),
        })
    }
}

macro_rules! impl_system {
    ($(($view:ident, $filter:ident, $var:ident)),+) => {
        impl<'a,
        Func,
        $($view: for<'b> View<'b> + DefaultFilter<Filter = $filter> + ViewElement,
        $filter: EntityFilter + Sync + 'static),+
    > IntoSystem<'a, (), ($($view,)+)> for Func
        where
            Func: FnMut($($view),+) + Send + Sync + 'static,
            $(<$view as View<'a>>::Iter: Iterator<Item = $view>),+
        {
            fn into_system(mut self, name: &'static str) -> Box<dyn Schedulable> {
                let resource_access: Access<ResourceTypeId> = Access::default();
                let component_access: Access<ComponentTypeId> = component_access!(($($view),+));

                let run_fn = FuncSystemFnWrapper(
                    move |_,
                        world,
                        _: (),
                        query: &mut system_query!($($view, $filter),+)
                        ,
                    | {
                        for tuple!($($var),+) in query.iter_mut(world) {
                            self($($var),+);
                        }
                    },
                    PhantomData,
                );

                Box::new(FuncSystem {
                    name: name.into(),
                    queries: AtomicRefCell::new(query!($($view),+)),
                    access: SystemAccess {
                        resources: resource_access,
                        components: component_access,
                        tags: Access::default(),
                    },
                    archetypes: ArchetypeAccess::Some(BitSet::default()),
                    _resources: PhantomData::<()>,
                    command_buffer: FxHashMap::default(),
                    run_fn: AtomicRefCell::new(run_fn),
                })
            }
        }
    }
}

macro_rules! tuple {
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

macro_rules! system_query {
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
    (()) => { () };
    ($($query:ident),+) => {
        <tuple!($($query),+)>::query()
    }
}

#[rustfmt::skip]
impl_system![(A, AF, a)];
#[rustfmt::skip]
impl_system![(A, AF, a), (B, BF, b)];
#[rustfmt::skip]
impl_system![(A, AF, a), (B, BF, b), (C, CF, c)];
#[rustfmt::skip]
impl_system![(A, AF, a), (B, BF, b), (C, CF, c), (D, DF, d)];
#[rustfmt::skip]
impl_system![(A, AF, a), (B, BF, b), (C, CF, c), (D, DF, d), (E, EF, e)];
#[rustfmt::skip]
impl_system![(A, AF, a), (B, BF, b), (C, CF, c), (D, DF, d), (E, EF, e), (F, FF, f)];
#[rustfmt::skip]
impl_system![(A, AF, a), (B, BF, b), (C, CF, c), (D, DF, d), (E, EF, e), (F, FF, f), (G, GF, g)];
#[rustfmt::skip]
impl_system![(A, AF, a), (B, BF, b), (C, CF, c), (D, DF, d), (E, EF, e), (F, FF, f), (G, GF, g), (H, HF, h)];
#[rustfmt::skip]
impl_system![(A, AF, a), (B, BF, b), (C, CF, c), (D, DF, d), (E, EF, e), (F, FF, f), (G, GF, g), (H, HF, h), (I, IF, i)];
#[rustfmt::skip]
impl_system![(A, AF, a), (B, BF, b), (C, CF, c), (D, DF, d), (E, EF, e), (F, FF, f), (G, GF, g), (H, HF, h), (I, IF, i), (J, JF, j)];
#[rustfmt::skip]
impl_system![(A, AF, a), (B, BF, b), (C, CF, c), (D, DF, d), (E, EF, e), (F, FF, f), (G, GF, g), (H, HF, h), (I, IF, i), (J, JF, j), (K, KF, k)];
#[rustfmt::skip]
impl_system![(A, AF, a), (B, BF, b), (C, CF, c), (D, DF, d), (E, EF, e), (F, FF, f), (G, GF, g), (H, HF, h), (I, IF, i), (J, JF, j), (K, KF, k), (L, LF, l)];

#[cfg(test)]
mod tests {
    use crate::{
        into_resource_for_each_system,
        resource::{PreparedRead, PreparedWrite, Resources},
        IntoSystem, system_fn_types::{ResourceMut, Resource},
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

        // fn single_read_system(x: Ref<X>) {
        //     println!("{}", x.0);
        // }
        // let mut system = single_read_system.into_system("hi");
        // system.run(&mut world, &mut resources);

        // fn read_write_system(x: Ref<X>, y: Ref<Y>, mut z: RefMut<A>) {
        //     z.0 += 1;
        //     println!("{} {} {}", x.0, y.0, z.0);
        // }

        // (
        //     {
        //     |x: Resource<A>, y: Ref<Y>, mut z: RefMut<A>| {
        //     z.0 += 1;
        //     println!("{} {} {}", x.0, y.0, z.0);
        // }}).into_system("bleh");

        // let mut system = read_write_system.into_system("read_write");
        // system.run(&mut world, &mut resources);

        // fn resource_system(a: Resource<A>, x: Ref<X>, y: Ref<Y>) {
        //     println!("{} {} {}", a.0, x.0, y.0);
        // }

        // let mut system = resource_system.into_system("hi");
        // system.run(&mut world, &mut resources);

        fn resource_system_mut(mut a: ResourceMut<A>, x: Ref<X>, y: Ref<Y>) {
            let hi = &mut a;
            a.0 += 1;
            println!("{} {} {}", a.0, x.0, y.0);
        }
        let mut system = resource_system_mut.into_system("hi");
        system.run(&mut world, &mut resources);

    }

    #[test]
    fn test_system_fn() {
        fn read_write_system(_: (), (_x, mut y): (Ref<X>, RefMut<Y>)) { y.0 += 1; }

        let mut world = World::new();
        let mut resources = Resources::default();
        world.insert((), vec![(X(1), Y(1)), (X(2), Y(2))]);

        let mut system = into_resource_for_each_system("read_write", read_write_system);
        system.run(&mut world, &mut resources);
    }

    #[test]
    fn test_resource_system_fn() {
        // fn my_system(
        //     (a,): (Resource<A>,),
        //     (x, mut y): (Ref<X>, RefMut<Y>),
        // ) {
        //     assert_eq!(*a, A(1));
        //     // assert_eq!(**b, B(0));
        //     if a.0 == 0 {
        //         assert_eq!(*x, X(2));
        //         assert_eq!(*y, Y(3));
        //     } else if a.0 == 1 {
        //         assert_eq!(*x, X(4));
        //         assert_eq!(*y, Y(5));
        //         y.0 += 1;
        //         assert_eq!(*y, Y(6));
        //     } else {
        //         panic!("unexpected value");
        //     }

        //     // a.0 += 1;
        // }

        // let mut world = World::new();
        // let mut resources = Resources::default();

        // resources.insert(A(0));
        // resources.insert(B(1));
        // world.insert((), vec![(X(2), Y(3)), (X(4), Y(5))]);
        // let mut my_system = into_resource_for_each_system("read_resources", my_system);
        // my_system.run(&mut world, &mut resources);
    }
}
