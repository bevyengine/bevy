use crate::{
    ArchetypeComponent, ChangedRes, Commands, Fetch, FromResources, Local, Or, Query, QueryAccess,
    QueryFilter, QuerySet, QueryTuple, Res, ResMut, Resource, ResourceIndex, Resources,
    SystemState, TypeAccess, World, WorldQuery,
};
use parking_lot::Mutex;
use std::{any::TypeId, sync::Arc};

pub struct In<Input>(pub Input);

impl<Input> SystemParam<Input> for In<Input> {
    #[inline]
    unsafe fn get_param(
        input: &mut Option<Input>,
        _system_state: &mut SystemState,
        _world: &World,
        _resources: &Resources,
    ) -> Option<Self> {
        Some(In(input.take().unwrap()))
    }

    fn init(_system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {}
}

pub trait SystemParam<Input>: Sized {
    fn init(system_state: &mut SystemState, world: &World, resources: &mut Resources);
    /// # Safety
    /// This call might access any of the input parameters in an unsafe way. Make sure the data access is safe in
    /// the context of the system scheduler
    unsafe fn get_param(
        input: &mut Option<Input>,
        system_state: &mut SystemState,
        world: &World,
        resources: &Resources,
    ) -> Option<Self>;
}

impl<'a, Q: WorldQuery, F: QueryFilter, Input> SystemParam<Input> for Query<'a, Q, F> {
    #[inline]
    unsafe fn get_param(
        _input: &mut Option<Input>,
        system_state: &mut SystemState,
        world: &World,
        _resources: &Resources,
    ) -> Option<Self> {
        let query_index = system_state.current_query_index;
        let world: &'a World = std::mem::transmute(world);
        let archetype_component_access: &'a TypeAccess<ArchetypeComponent> =
            std::mem::transmute(&system_state.query_archetype_component_accesses[query_index]);
        system_state.current_query_index += 1;
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

impl<T: QueryTuple, Input> SystemParam<Input> for QuerySet<T> {
    #[inline]
    unsafe fn get_param(
        _input: &mut Option<Input>,
        system_state: &mut SystemState,
        world: &World,
        _resources: &Resources,
    ) -> Option<Self> {
        let query_index = system_state.current_query_index;
        system_state.current_query_index += 1;
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

impl<'a, Input> SystemParam<Input> for &'a mut Commands {
    fn init(system_state: &mut SystemState, world: &World, _resources: &mut Resources) {
        system_state
            .commands
            .set_entity_reserver(world.get_entity_reserver())
    }

    #[inline]
    unsafe fn get_param(
        _input: &mut Option<Input>,
        system_state: &mut SystemState,
        _world: &World,
        _resources: &Resources,
    ) -> Option<Self> {
        let commands: &'a mut Commands = std::mem::transmute(&mut system_state.commands);
        Some(commands)
    }
}

impl<Input> SystemParam<Input> for Arc<Mutex<Commands>> {
    fn init(system_state: &mut SystemState, world: &World, _resources: &mut Resources) {
        system_state.arc_commands.get_or_insert_with(|| {
            let mut commands = Commands::default();
            commands.set_entity_reserver(world.get_entity_reserver());
            Arc::new(Mutex::new(commands))
        });
    }

    #[inline]
    unsafe fn get_param(
        _input: &mut Option<Input>,
        system_state: &mut SystemState,
        _world: &World,
        _resources: &Resources,
    ) -> Option<Self> {
        Some(system_state.arc_commands.as_ref().unwrap().clone())
    }
}

impl<'a, T: Resource, Input> SystemParam<Input> for Res<'a, T> {
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
        _input: &mut Option<Input>,
        _system_state: &mut SystemState,
        _world: &World,
        resources: &Resources,
    ) -> Option<Self> {
        Some(Res::new(
            resources.get_unsafe_ref::<T>(ResourceIndex::Global),
        ))
    }
}

impl<'a, T: Resource, Input> SystemParam<Input> for ResMut<'a, T> {
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
        _input: &mut Option<Input>,
        _system_state: &mut SystemState,
        _world: &World,
        resources: &Resources,
    ) -> Option<Self> {
        let (value, _added, mutated) =
            resources.get_unsafe_ref_with_added_and_mutated::<T>(ResourceIndex::Global);
        Some(ResMut::new(value, mutated))
    }
}

impl<'a, T: Resource, Input> SystemParam<Input> for ChangedRes<'a, T> {
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
        _input: &mut Option<Input>,
        _system_state: &mut SystemState,
        _world: &World,
        resources: &Resources,
    ) -> Option<Self> {
        let (value, added, mutated) =
            resources.get_unsafe_ref_with_added_and_mutated::<T>(ResourceIndex::Global);
        if *added.as_ptr() || *mutated.as_ptr() {
            Some(ChangedRes::new(value))
        } else {
            None
        }
    }
}

impl<'a, T: Resource + FromResources, Input> SystemParam<Input> for Local<'a, T> {
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
        _input: &mut Option<Input>,
        system_state: &mut SystemState,
        _world: &World,
        resources: &Resources,
    ) -> Option<Self> {
        Some(Local::new(resources, system_state.id))
    }
}

macro_rules! impl_system_param_tuple {
    ($($param: ident),*) => {
        #[allow(unused_variables)]
        impl<Input, $($param: SystemParam<Input>),*> SystemParam<Input> for ($($param,)*) {
            fn init(system_state: &mut SystemState, world: &World, resources: &mut Resources) {
                $($param::init(system_state, world, resources);)*
            }

            #[inline]
            unsafe fn get_param(
                input: &mut Option<Input>,
                system_state: &mut SystemState,
                world: &World,
                resources: &Resources,
            ) -> Option<Self> {
                Some(($($param::get_param(input, system_state, world, resources)?,)*))
            }
        }

        #[allow(unused_variables)]
        #[allow(unused_mut)]
        #[allow(non_snake_case)]
        impl<Input, $($param: SystemParam<Input>),*> SystemParam<Input> for Or<($(Option<$param>,)*)> {
            fn init(system_state: &mut SystemState, world: &World, resources: &mut Resources) {
                $($param::init(system_state, world, resources);)*
            }

            #[inline]
            unsafe fn get_param(
                input: &mut Option<Input>,
                system_state: &mut SystemState,
                world: &World,
                resources: &Resources,
            ) -> Option<Self> {
                let mut has_some = false;
                $(
                    let $param = $param::get_param(input, system_state, world, resources);
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
