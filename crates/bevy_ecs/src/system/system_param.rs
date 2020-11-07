pub use bevy_hecs::SystemParam;

use crate::{
    ChangedRes, Commands, FromResources, Local, Query, QuerySet, QueryTuple, Res, ResMut, Resource,
    ResourceIndex, Resources, SystemState,
};
use bevy_hecs::{ArchetypeComponent, Fetch, Query as HecsQuery, TypeAccess, World};
use std::any::TypeId;
pub trait SystemParam {
    fn init(system_state: &mut SystemState, world: &World, resources: &mut Resources);
    unsafe fn get_param(
        system_state: &mut SystemState,
        world: &World,
        resources: &Resources,
    ) -> Self;
}

impl<'a, Q: HecsQuery> SystemParam for Query<'a, Q> {
    #[inline]
    unsafe fn get_param(
        system_state: &mut SystemState,
        world: &World,
        _resources: &Resources,
    ) -> Self {
        let query_index = system_state.current_query_index;
        let world: &'a World = std::mem::transmute(world);
        let archetype_component_access: &'a TypeAccess<ArchetypeComponent> =
            std::mem::transmute(&system_state.query_archetype_component_accesses[query_index]);
        system_state.current_query_index += 1;
        Query::new(world, archetype_component_access)
    }

    fn init(system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
        system_state
            .query_archetype_component_accesses
            .push(TypeAccess::default());
        system_state
            .query_accesses
            .push(vec![<Q::Fetch as Fetch>::access()]);
        system_state
            .query_type_names
            .push(std::any::type_name::<Q>());
    }
}

impl<T: QueryTuple> SystemParam for QuerySet<T> {
    #[inline]
    unsafe fn get_param(
        system_state: &mut SystemState,
        world: &World,
        _resources: &Resources,
    ) -> Self {
        let query_index = system_state.current_query_index;
        system_state.current_query_index += 1;
        QuerySet::new(
            world,
            &system_state.query_archetype_component_accesses[query_index],
        )
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

impl SystemParam for Commands {
    fn init(system_state: &mut SystemState, world: &World, _resources: &mut Resources) {
        system_state
            .commands
            .set_entity_reserver(world.get_entity_reserver())
    }

    #[inline]
    unsafe fn get_param(
        system_state: &mut SystemState,
        _world: &World,
        _resources: &Resources,
    ) -> Self {
        system_state.commands.clone()
    }
}

impl<'a, T: Resource> SystemParam for Res<'a, T> {
    fn init(system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
        system_state.resource_access.add_read(TypeId::of::<T>());
    }

    unsafe fn get_param(
        _system_state: &mut SystemState,
        _world: &World,
        resources: &Resources,
    ) -> Self {
        Res::new(resources.get_unsafe_ref::<T>(ResourceIndex::Global))
    }
}

impl<'a, T: Resource> SystemParam for ResMut<'a, T> {
    fn init(system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
        system_state.resource_access.add_write(TypeId::of::<T>());
    }

    unsafe fn get_param(
        _system_state: &mut SystemState,
        _world: &World,
        resources: &Resources,
    ) -> Self {
        let (value, type_state) =
            resources.get_unsafe_ref_with_type_state::<T>(ResourceIndex::Global);
        ResMut::new(value, type_state.mutated())
    }
}

impl<'a, T: Resource> SystemParam for ChangedRes<'a, T> {
    fn init(system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
        system_state.resource_access.add_read(TypeId::of::<T>());
    }

    unsafe fn get_param(
        _system_state: &mut SystemState,
        _world: &World,
        resources: &Resources,
    ) -> Self {
        let (added, mutated) = resources.get_unsafe_added_and_mutated::<T>(ResourceIndex::Global);
        if *added.as_ptr() || *mutated.as_ptr() {
            ChangedRes::new(resources.get_unsafe_ref::<T>(ResourceIndex::Global))
        } else {
            todo!("return option");
        }
    }
}

impl<'a, T: Resource + FromResources> SystemParam for Local<'a, T> {
    fn init(system_state: &mut SystemState, _world: &World, resources: &mut Resources) {
        system_state.resource_access.add_write(TypeId::of::<T>());
        if resources.get_local::<T>(system_state.id).is_none() {
            let value = T::from_resources(resources);
            resources.insert_local(system_state.id, value);
        }
    }

    unsafe fn get_param(
        system_state: &mut SystemState,
        _world: &World,
        resources: &Resources,
    ) -> Self {
        Local::new(resources, system_state.id)
    }
}
