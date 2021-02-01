use crate::{
    ArchetypeComponent, ChangedRes, Commands, Fetch, FromResources, Local, Or, Query, QueryAccess,
    QueryFilter, QuerySet, QueryTuple, Res, ResMut, Resource, ResourceIndex, Resources,
    SystemState, TypeAccess, World, WorldQuery,
};
use parking_lot::Mutex;
use std::{any::TypeId, marker::PhantomData, sync::Arc};
pub trait SystemParam: Sized {
    type Fetch: for<'a> FetchSystemParam<'a>;
}

pub trait FetchSystemParam<'a> {
    type Item;
    fn init(system_state: &mut SystemState, world: &World, resources: &mut Resources);
    /// # Safety
    /// This call might access any of the input parameters in an unsafe way. Make sure the data access is safe in
    /// the context of the system scheduler
    unsafe fn get_param(
        system_state: &'a SystemState,
        world: &'a World,
        resources: &'a Resources,
    ) -> Option<Self::Item>;
}

pub struct FetchQuery<Q, F>(PhantomData<(Q, F)>);

impl<'a, Q: WorldQuery, F: QueryFilter> SystemParam for Query<'a, Q, F> {
    type Fetch = FetchQuery<Q, F>;
}

impl<'a, Q: WorldQuery, F: QueryFilter> FetchSystemParam<'a> for FetchQuery<Q, F> {
    type Item = Query<'a, Q, F>;

    #[inline]
    unsafe fn get_param(
        system_state: &'a SystemState,
        world: &'a World,
        _resources: &'a Resources,
    ) -> Option<Self::Item> {
        let query_index = *system_state.current_query_index.get();
        let archetype_component_access: &'a TypeAccess<ArchetypeComponent> =
            &system_state.query_archetype_component_accesses[query_index];
        *system_state.current_query_index.get() += 1;
        Some(Query::new(world, archetype_component_access))
    }

    fn init(system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
        system_state
            .query_archetype_component_accesses
            .push(TypeAccess::default());
        let access = QueryAccess::union(vec![Q::Fetch::access(), F::access()]);
        system_state.query_accesses.push(vec![access]);
        system_state
            .query_type_names
            .push(std::any::type_name::<Q>());
    }
}

pub struct FetchQuerySet<T>(PhantomData<T>);

impl<T: QueryTuple> SystemParam for QuerySet<T> {
    type Fetch = FetchQuerySet<T>;
}

impl<'a, T: QueryTuple> FetchSystemParam<'a> for FetchQuerySet<T> {
    type Item = QuerySet<T>;

    #[inline]
    unsafe fn get_param(
        system_state: &'a SystemState,
        world: &'a World,
        _resources: &'a Resources,
    ) -> Option<Self::Item> {
        let query_index = *system_state.current_query_index.get();
        *system_state.current_query_index.get() += 1;
        Some(QuerySet::new(
            world,
            &system_state.query_archetype_component_accesses[query_index],
        ))
    }

    fn init(system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
        system_state
            .query_archetype_component_accesses
            .push(TypeAccess::default());
        system_state.query_accesses.push(T::get_accesses());
        system_state
            .query_type_names
            .push(std::any::type_name::<T>());
    }
}

pub struct FetchCommands;

impl<'a> SystemParam for &'a mut Commands {
    type Fetch = FetchCommands;
}
impl<'a> FetchSystemParam<'a> for FetchCommands {
    type Item = &'a mut Commands;

    fn init(system_state: &mut SystemState, world: &World, _resources: &mut Resources) {
        // SAFE: this is called with unique access to SystemState
        unsafe {
            (&mut *system_state.commands.get()).set_entity_reserver(world.get_entity_reserver())
        }
    }

    #[inline]
    unsafe fn get_param(
        system_state: &'a SystemState,
        _world: &'a World,
        _resources: &'a Resources,
    ) -> Option<Self::Item> {
        Some(&mut *system_state.commands.get())
    }
}

pub struct FetchArcCommands;
impl SystemParam for Arc<Mutex<Commands>> {
    type Fetch = FetchArcCommands;
}

impl<'a> FetchSystemParam<'a> for FetchArcCommands {
    type Item = Arc<Mutex<Commands>>;

    fn init(system_state: &mut SystemState, world: &World, _resources: &mut Resources) {
        system_state.arc_commands.get_or_insert_with(|| {
            let mut commands = Commands::default();
            commands.set_entity_reserver(world.get_entity_reserver());
            Arc::new(Mutex::new(commands))
        });
    }

    #[inline]
    unsafe fn get_param(
        system_state: &SystemState,
        _world: &World,
        _resources: &Resources,
    ) -> Option<Self::Item> {
        Some(system_state.arc_commands.as_ref().unwrap().clone())
    }
}

pub struct FetchRes<T>(PhantomData<T>);

impl<'a, T: Resource> SystemParam for Res<'a, T> {
    type Fetch = FetchRes<T>;
}

impl<'a, T: Resource> FetchSystemParam<'a> for FetchRes<T> {
    type Item = Res<'a, T>;

    fn init(system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
        if system_state.resource_access.is_write(&TypeId::of::<T>()) {
            panic!(
                "System `{}` has a `Res<{res}>` parameter that conflicts with \
                another parameter with mutable access to the same `{res}` resource.",
                system_state.name,
                res = std::any::type_name::<T>()
            );
        }
        system_state.resource_access.add_read(TypeId::of::<T>());
    }

    #[inline]
    unsafe fn get_param(
        _system_state: &'a SystemState,
        _world: &'a World,
        resources: &'a Resources,
    ) -> Option<Self::Item> {
        Some(Res::new(
            resources.get_unsafe_ref::<T>(ResourceIndex::Global),
        ))
    }
}

pub struct FetchResMut<T>(PhantomData<T>);

impl<'a, T: Resource> SystemParam for ResMut<'a, T> {
    type Fetch = FetchResMut<T>;
}
impl<'a, T: Resource> FetchSystemParam<'a> for FetchResMut<T> {
    type Item = ResMut<'a, T>;

    fn init(system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
        // If a system already has access to the resource in another parameter, then we fail early.
        // e.g. `fn(Res<Foo>, ResMut<Foo>)` or `fn(ResMut<Foo>, ResMut<Foo>)` must not be allowed.
        if system_state
            .resource_access
            .is_read_or_write(&TypeId::of::<T>())
        {
            panic!(
                "System `{}` has a `ResMut<{res}>` parameter that conflicts with \
                another parameter to the same `{res}` resource. `ResMut` must have unique access.",
                system_state.name,
                res = std::any::type_name::<T>()
            );
        }
        system_state.resource_access.add_write(TypeId::of::<T>());
    }

    #[inline]
    unsafe fn get_param(
        _system_state: &'a SystemState,
        _world: &'a World,
        resources: &'a Resources,
    ) -> Option<Self::Item> {
        let (value, _added, mutated) =
            resources.get_unsafe_ref_with_added_and_mutated::<T>(ResourceIndex::Global);
        Some(ResMut::new(value, mutated))
    }
}

pub struct FetchChangedRes<T>(PhantomData<T>);

impl<'a, T: Resource> SystemParam for ChangedRes<'a, T> {
    type Fetch = FetchChangedRes<T>;
}

impl<'a, T: Resource> FetchSystemParam<'a> for FetchChangedRes<T> {
    type Item = ChangedRes<'a, T>;

    fn init(system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
        if system_state.resource_access.is_write(&TypeId::of::<T>()) {
            panic!(
                "System `{}` has a `ChangedRes<{res}>` parameter that conflicts with \
                another parameter with mutable access to the same `{res}` resource.",
                system_state.name,
                res = std::any::type_name::<T>()
            );
        }
        system_state.resource_access.add_read(TypeId::of::<T>());
    }

    #[inline]
    unsafe fn get_param(
        _system_state: &'a SystemState,
        _world: &'a World,
        resources: &'a Resources,
    ) -> Option<Self::Item> {
        let (value, added, mutated) =
            resources.get_unsafe_ref_with_added_and_mutated::<T>(ResourceIndex::Global);
        if *added.as_ptr() || *mutated.as_ptr() {
            Some(ChangedRes::new(value))
        } else {
            None
        }
    }
}

pub struct FetchLocal<T>(PhantomData<T>);

impl<'a, T: Resource + FromResources> SystemParam for Local<'a, T> {
    type Fetch = FetchLocal<T>;
}
impl<'a, T: Resource + FromResources> FetchSystemParam<'a> for FetchLocal<T> {
    type Item = Local<'a, T>;

    fn init(system_state: &mut SystemState, _world: &World, resources: &mut Resources) {
        if system_state
            .local_resource_access
            .is_read_or_write(&TypeId::of::<T>())
        {
            panic!(
                "System `{}` has multiple parameters requesting access to a local resource of type `{}`. \
                There may be at most one `Local` parameter per resource type.",
                system_state.name,
                std::any::type_name::<T>()
            );
        }

        // A resource could have been already initialized by another system with
        // `Commands::insert_local_resource` or `Resources::insert_local`
        if resources.get_local::<T>(system_state.id).is_none() {
            let value = T::from_resources(resources);
            resources.insert_local(system_state.id, value);
        }

        system_state
            .local_resource_access
            .add_write(TypeId::of::<T>());
    }

    #[inline]
    unsafe fn get_param(
        system_state: &'a SystemState,
        _world: &'a World,
        resources: &'a Resources,
    ) -> Option<Self::Item> {
        Some(Local::new(resources, system_state.id))
    }
}

pub struct FetchParamTuple<T>(PhantomData<T>);
pub struct FetchOr<T>(PhantomData<T>);

macro_rules! impl_system_param_tuple {
    ($($param: ident),*) => {
        impl<$($param: SystemParam),*> SystemParam for ($($param,)*) {
            type Fetch = FetchParamTuple<($($param::Fetch,)*)>;
        }
        #[allow(unused_variables)]
        impl<'a, $($param: FetchSystemParam<'a>),*> FetchSystemParam<'a> for FetchParamTuple<($($param,)*)> {
            type Item = ($($param::Item,)*);
            fn init(system_state: &mut SystemState, world: &World, resources: &mut Resources) {
                $($param::init(system_state, world, resources);)*
            }

            #[inline]
            unsafe fn get_param(
                system_state: &'a SystemState,
                world: &'a World,
                resources: &'a Resources,
            ) -> Option<Self::Item> {
                Some(($($param::get_param(system_state, world, resources)?,)*))
            }
        }

        impl<$($param: SystemParam),*> SystemParam for Or<($(Option<$param>,)*)> {
            type Fetch = FetchOr<($($param::Fetch,)*)>;
        }

        #[allow(unused_variables)]
        #[allow(unused_mut)]
        #[allow(non_snake_case)]
        impl<'a, $($param: FetchSystemParam<'a>),*> FetchSystemParam<'a> for FetchOr<($($param,)*)> {
            type Item = Or<($(Option<$param::Item>,)*)>;
            fn init(system_state: &mut SystemState, world: &World, resources: &mut Resources) {
                $($param::init(system_state, world, resources);)*
            }

            #[inline]
            unsafe fn get_param(
                system_state: &'a SystemState,
                world: &'a World,
                resources: &'a Resources,
            ) -> Option<Self::Item> {
                let mut has_some = false;
                $(
                    let $param = $param::get_param(system_state, world, resources);
                    if $param.is_some() {
                        has_some = true;
                    }
                )*

                if has_some {
                    Some(Or(($($param,)*)))
                } else {
                    None
                }
            }
        }
    };
}

impl_system_param_tuple!();
impl_system_param_tuple!(A);
impl_system_param_tuple!(A, B);
impl_system_param_tuple!(A, B, C);
impl_system_param_tuple!(A, B, C, D);
impl_system_param_tuple!(A, B, C, D, E);
impl_system_param_tuple!(A, B, C, D, E, F);
impl_system_param_tuple!(A, B, C, D, E, F, G);
impl_system_param_tuple!(A, B, C, D, E, F, G, H);
impl_system_param_tuple!(A, B, C, D, E, F, G, H, I);
impl_system_param_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_system_param_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_system_param_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_system_param_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_system_param_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_system_param_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_system_param_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
