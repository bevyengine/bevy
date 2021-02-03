use std::{
    any::TypeId,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use parking_lot::Mutex;

use crate::{
    ArchetypeComponent, ChangedRes, Commands, Fetch, FromResources, Query, QueryAccess,
    QueryFilter, QuerySet, QueryTuple, Res, ResMut, Resource, ResourceIndex, Resources,
    SystemState, TypeAccess, World, WorldQuery,
};

use super::{ParamState, PureParamState, PureSystemParam, SystemParam};

// TODO NOW: Make Local<T> equivalent to &mut T
#[derive(Debug)]
pub struct Local<T>(T);

impl<T> Deref for Local<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Local<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// TODO: Equivalent impl for &Local<T> - would need type
impl<T: FromResources + 'static + Send + Sync> PureSystemParam for &'static mut Local<T> {
    type Config = Option<T>;
    type State = Local<T>;

    fn create_state_pure(config: Self::Config, resources: &mut Resources) -> Self::State {
        Local(config.unwrap_or_else(|| T::from_resources(resources)))
    }

    fn default_config_pure() -> Self::Config {
        None
    }
}

impl<'a, T: Send + Sync + 'static> PureParamState<'a> for Local<T> {
    type Item = &'a mut Local<T>;

    fn view_param(&'a mut self) -> Self::Item {
        self
    }
}

// TODO: Store state here instead of in super::SystemState
pub struct QueryState<Q, F>(PhantomData<(Q, F)>);

impl<Q: WorldQuery + 'static + Send + Sync, F: QueryFilter + 'static + Send + Sync> SystemParam
    for Query<'static, Q, F>
{
    type Config = ();

    type State = QueryState<Q, F>;

    fn create_state(_: Self::Config, _: &mut crate::Resources) -> Self::State {
        QueryState(PhantomData)
    }

    fn default_config() {}
}

impl<'a, Q: WorldQuery + 'static + Send + Sync, F: QueryFilter + 'static + Send + Sync>
    ParamState<'a> for QueryState<Q, F>
{
    type Item = Query<'a, Q, F>;

    #[inline]
    unsafe fn get_param(
        &mut self,
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

    fn init(&mut self, system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
        system_state
            .query_archetype_component_accesses
            .push(TypeAccess::default());
        let access = QueryAccess::union(vec![<Q::Fetch as Fetch<'static>>::access(), F::access()]);
        system_state.query_accesses.push(vec![access]);
        system_state
            .query_type_names
            .push(std::any::type_name::<Q>());
    }
}

// TODO: These can be safely Send + Sync since they are empty.
pub struct QuerySetState<T>(PhantomData<T>);

impl<T: QueryTuple + 'static + Send + Sync> SystemParam for QuerySet<T> {
    type Config = ();
    type State = QuerySetState<T>;

    fn create_state(_: Self::Config, _: &mut Resources) -> Self::State {
        QuerySetState(PhantomData)
    }

    fn default_config() {}
}

impl<'a, T: QueryTuple + Send + Sync + 'static> ParamState<'a> for QuerySetState<T> {
    type Item = QuerySet<T>;

    #[inline]
    unsafe fn get_param(
        &mut self,
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

    fn init(&mut self, system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
        system_state
            .query_archetype_component_accesses
            .push(TypeAccess::default());
        system_state.query_accesses.push(T::get_accesses());
        system_state
            .query_type_names
            .push(std::any::type_name::<T>());
    }
}

impl SystemParam for &'static mut Commands {
    type Config = ();
    type State = Commands;

    fn create_state(_: Self::Config, _: &mut Resources) -> Self::State {
        Commands::default()
    }

    fn default_config() {}
}
impl<'a> ParamState<'a> for Commands {
    type Item = &'a mut Commands;

    fn init(&mut self, _: &mut SystemState, world: &World, _resources: &mut Resources) {
        self.set_entity_reserver(world.get_entity_reserver())
    }

    #[inline]
    unsafe fn get_param(
        &'a mut self,
        _: &'a SystemState,
        _world: &'a World,
        _resources: &'a Resources,
    ) -> Option<Self::Item> {
        Some(self)
    }

    fn run_sync(&mut self, world: &mut World, resources: &mut Resources) {
        self.apply(world, resources);
    }
}

impl SystemParam for Arc<Mutex<Commands>> {
    type Config = ();
    type State = Arc<Mutex<Commands>>;

    fn create_state(_: Self::Config, _: &mut Resources) -> Self::State {
        Arc::new(Mutex::new(Commands::default()))
    }

    fn default_config() {}
}

impl<'a> ParamState<'a> for Arc<Mutex<Commands>> {
    type Item = Arc<Mutex<Commands>>;

    fn init(&mut self, _: &mut SystemState, world: &World, _resources: &mut Resources) {
        // TODO(DJMcNab): init should be combined into create_state
        self.lock().set_entity_reserver(world.get_entity_reserver());
    }

    #[inline]
    unsafe fn get_param(
        &mut self,
        _: &SystemState,
        _world: &World,
        _resources: &Resources,
    ) -> Option<Self::Item> {
        Some(self.clone())
    }

    fn run_sync(&mut self, world: &mut World, resources: &mut Resources) {
        // TODO: try_lock here?
        // Don't want to block the entire world on a single missing lock release
        self.lock().apply(world, resources);
    }
}

pub struct ResState<T>(PhantomData<T>);

impl<T: Resource> SystemParam for Res<'static, T> {
    type Config = ();
    type State = ResState<T>;
    fn create_state(_: Self::Config, _: &mut Resources) -> Self::State {
        ResState(PhantomData)
    }

    fn default_config() {}
}

impl<'a, T: Resource> ParamState<'a> for ResState<T> {
    type Item = Res<'a, T>;

    fn init(&mut self, system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
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
        &mut self,
        _system_state: &'a SystemState,
        _world: &'a World,
        resources: &'a Resources,
    ) -> Option<Self::Item> {
        Some(Res::new(
            resources.get_unsafe_ref::<T>(ResourceIndex::Global),
        ))
    }
}

pub struct ResMutState<T>(PhantomData<T>);

impl<T: Resource> SystemParam for ResMut<'static, T> {
    type Config = ();
    type State = ResMutState<T>;
    fn create_state(_: Self::Config, _: &mut Resources) -> Self::State {
        ResMutState(PhantomData)
    }

    fn default_config() {}
}

impl<'a, T: Resource> ParamState<'a> for ResMutState<T> {
    type Item = ResMut<'a, T>;

    fn init(&mut self, system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
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
        system_state.resource_access.add_read(TypeId::of::<T>());
    }

    #[inline]
    unsafe fn get_param(
        &mut self,
        _system_state: &'a SystemState,
        _world: &'a World,
        resources: &'a Resources,
    ) -> Option<Self::Item> {
        let (value, _added, mutated) =
            resources.get_unsafe_ref_with_added_and_mutated::<T>(ResourceIndex::Global);
        Some(ResMut::new(value, mutated))
    }
}

pub struct ChangedResState<T>(PhantomData<T>);

impl<T: Resource> SystemParam for ChangedRes<'static, T> {
    type Config = ();
    type State = ChangedResState<T>;

    fn create_state(_: Self::Config, _: &mut Resources) -> Self::State {
        ChangedResState(PhantomData)
    }

    fn default_config() {}
}

impl<'a, T: Resource> ParamState<'a> for ChangedResState<T> {
    type Item = ChangedRes<'a, T>;

    fn init(&mut self, system_state: &mut SystemState, _world: &World, _resources: &mut Resources) {
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
        &mut self,
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
