use crate::{
    archetype::{Archetype, Archetypes},
    bundle::Bundles,
    component::{Component, ComponentFlags, ComponentId, Components},
    entity::{Entities, Entity},
    query::{FilterFetch, FilteredAccess, FilteredAccessSet, QueryState, WorldQuery},
    system::{CommandQueue, Commands, Query, SystemState},
    world::{FromWorld, World},
};
pub use bevy_ecs_macros::SystemParam;
use bevy_ecs_macros::{all_tuples, impl_query_set};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A parameter that can be used in a system function
///
/// # Derive
/// This trait can be derived.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::system::SystemParam;
///
/// #[derive(SystemParam)]
/// pub struct MyParam<'a> {
///     foo: Res<'a, usize>,
/// }
///
/// fn my_system(param: MyParam) {
///     // Access the resource through `param.foo`
/// }
/// ```
pub trait SystemParam: Sized {
    type Fetch: for<'a> SystemParamFetch<'a>;
}

/// # Safety
/// it is the implementors responsibility to ensure `system_state` is populated with the _exact_
/// [World] access used by the SystemParamState (and associated FetchSystemParam). Additionally, it
/// is the implementor's responsibility to ensure there is no conflicting access across all
/// SystemParams.
pub unsafe trait SystemParamState: Send + Sync + 'static {
    type Config: Default + Send + Sync;
    fn init(world: &mut World, system_state: &mut SystemState, config: Self::Config) -> Self;
    #[inline]
    fn new_archetype(&mut self, _archetype: &Archetype, _system_state: &mut SystemState) {}
    #[inline]
    fn apply(&mut self, _world: &mut World) {}
}

pub trait SystemParamFetch<'a>: SystemParamState {
    type Item;
    /// # Safety
    /// This call might access any of the input parameters in an unsafe way. Make sure the data
    /// access is safe in the context of the system scheduler
    unsafe fn get_param(
        state: &'a mut Self,
        system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item;
}

pub struct QueryFetch<Q, F>(PhantomData<(Q, F)>);

impl<'a, Q: WorldQuery + 'static, F: WorldQuery + 'static> SystemParam for Query<'a, Q, F>
where
    F::Fetch: FilterFetch,
{
    type Fetch = QueryState<Q, F>;
}

// SAFE: Relevant query ComponentId and ArchetypeComponentId access is applied to SystemState. If
// this QueryState conflicts with any prior access, a panic will occur.
unsafe impl<Q: WorldQuery + 'static, F: WorldQuery + 'static> SystemParamState for QueryState<Q, F>
where
    F::Fetch: FilterFetch,
{
    type Config = ();

    fn init(world: &mut World, system_state: &mut SystemState, _config: Self::Config) -> Self {
        let state = QueryState::new(world);
        assert_component_access_compatibility(
            &system_state.name,
            std::any::type_name::<Q>(),
            std::any::type_name::<F>(),
            &system_state.component_access_set,
            &state.component_access,
            world,
        );
        system_state
            .component_access_set
            .add(state.component_access.clone());
        system_state
            .archetype_component_access
            .extend(&state.archetype_component_access);
        state
    }

    fn new_archetype(&mut self, archetype: &Archetype, system_state: &mut SystemState) {
        self.new_archetype(archetype);
        system_state
            .archetype_component_access
            .extend(&self.archetype_component_access);
    }
}

impl<'a, Q: WorldQuery + 'static, F: WorldQuery + 'static> SystemParamFetch<'a> for QueryState<Q, F>
where
    F::Fetch: FilterFetch,
{
    type Item = Query<'a, Q, F>;

    #[inline]
    unsafe fn get_param(
        state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        Query::new(world, state)
    }
}

fn assert_component_access_compatibility(
    system_name: &str,
    query_type: &'static str,
    filter_type: &'static str,
    system_access: &FilteredAccessSet<ComponentId>,
    current: &FilteredAccess<ComponentId>,
    world: &World,
) {
    let mut conflicts = system_access.get_conflicts(current);
    if conflicts.is_empty() {
        return;
    }
    let conflicting_components = conflicts
        .drain(..)
        .map(|component_id| world.components.get_info(component_id).unwrap().name())
        .collect::<Vec<&str>>();
    let accesses = conflicting_components.join(", ");
    panic!("Query<{}, {}> in system {} accesses component(s) {} in a way that conflicts with a previous system parameter. Allowing this would break Rust's mutability rules. Consider merging conflicting Queries into a QuerySet.",
                query_type, filter_type, system_name, accesses);
}

pub struct QuerySet<T>(T);
pub struct QuerySetState<T>(T);

impl_query_set!();

/// Shared borrow of a Resource
///
/// When used as a system parameter, panics if resource does not exist.
///
/// Use `Option<Res<T>>` if the resource might not always exist.
pub struct Res<'w, T> {
    value: &'w T,
    flags: ComponentFlags,
}

impl<'w, T: Component> Res<'w, T> {
    /// Returns true if (and only if) this resource been added since the start of the frame.
    pub fn added(&self) -> bool {
        self.flags.contains(ComponentFlags::ADDED)
    }

    /// Returns true if (and only if) this resource been mutated since the start of the frame.
    pub fn mutated(&self) -> bool {
        self.flags.contains(ComponentFlags::MUTATED)
    }

    /// Returns true if (and only if) this resource been either mutated or added since the start of
    /// the frame.
    pub fn changed(&self) -> bool {
        self.flags
            .intersects(ComponentFlags::ADDED | ComponentFlags::MUTATED)
    }
}

impl<'w, T: Component> Deref for Res<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

pub struct ResState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<'a, T: Component> SystemParam for Res<'a, T> {
    type Fetch = ResState<T>;
}

// SAFE: Res ComponentId and ArchetypeComponentId access is applied to SystemState. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<T: Component> SystemParamState for ResState<T> {
    type Config = ();

    fn init(world: &mut World, system_state: &mut SystemState, _config: Self::Config) -> Self {
        let component_id = world.initialize_resource::<T>();
        let combined_access = system_state.component_access_set.combined_access_mut();
        if combined_access.has_write(component_id) {
            panic!(
                "Res<{}> in system {} conflicts with a previous ResMut<{0}> access. Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_state.name);
        }
        combined_access.add_read(component_id);

        let resource_archetype = world.archetypes.resource();
        let archetype_component_id = resource_archetype
            .get_archetype_component_id(component_id)
            .unwrap();
        system_state
            .archetype_component_access
            .add_read(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }
}

impl<'a, T: Component> SystemParamFetch<'a> for ResState<T> {
    type Item = Res<'a, T>;

    #[inline]
    unsafe fn get_param(
        state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        let column = world
            .get_populated_resource_column(state.component_id)
            .expect("Requested resource does not exist");
        Res {
            value: &*column.get_ptr().as_ptr().cast::<T>(),
            flags: *column.get_flags_mut_ptr(),
        }
    }
}

pub struct OptionResState<T>(ResState<T>);

impl<'a, T: Component> SystemParam for Option<Res<'a, T>> {
    type Fetch = OptionResState<T>;
}

unsafe impl<T: Component> SystemParamState for OptionResState<T> {
    type Config = ();

    fn init(world: &mut World, system_state: &mut SystemState, _config: Self::Config) -> Self {
        Self(ResState::init(world, system_state, ()))
    }
}

impl<'a, T: Component> SystemParamFetch<'a> for OptionResState<T> {
    type Item = Option<Res<'a, T>>;

    #[inline]
    unsafe fn get_param(
        state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        world
            .get_populated_resource_column(state.0.component_id)
            .map(|column| Res {
                value: &*column.get_ptr().as_ptr().cast::<T>(),
                flags: *column.get_flags_mut_ptr(),
            })
    }
}

/// Unique borrow of a Resource
///
/// When used as a system parameter, panics if resource does not exist.
///
/// Use `Option<ResMut<T>>` if the resource might not always exist.
pub struct ResMut<'w, T> {
    value: &'w mut T,
    flags: &'w mut ComponentFlags,
}

impl<'w, T: Component> ResMut<'w, T> {
    /// Returns true if (and only if) this resource been added since the start of the frame.
    pub fn added(&self) -> bool {
        self.flags.contains(ComponentFlags::ADDED)
    }

    /// Returns true if (and only if) this resource been mutated since the start of the frame.
    pub fn mutated(&self) -> bool {
        self.flags.contains(ComponentFlags::MUTATED)
    }

    /// Returns true if (and only if) this resource been either mutated or added since the start of
    /// the frame.
    pub fn changed(&self) -> bool {
        self.flags
            .intersects(ComponentFlags::ADDED | ComponentFlags::MUTATED)
    }
}

impl<'w, T: Component> Deref for ResMut<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T: Component> DerefMut for ResMut<'w, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.flags.insert(ComponentFlags::MUTATED);
        self.value
    }
}

pub struct ResMutState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<'a, T: Component> SystemParam for ResMut<'a, T> {
    type Fetch = ResMutState<T>;
}

// SAFE: Res ComponentId and ArchetypeComponentId access is applied to SystemState. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<T: Component> SystemParamState for ResMutState<T> {
    type Config = ();

    fn init(world: &mut World, system_state: &mut SystemState, _config: Self::Config) -> Self {
        let component_id = world.initialize_resource::<T>();
        let combined_access = system_state.component_access_set.combined_access_mut();
        if combined_access.has_write(component_id) {
            panic!(
                "ResMut<{}> in system {} conflicts with a previous ResMut<{0}> access. Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_state.name);
        } else if combined_access.has_read(component_id) {
            panic!(
                "ResMut<{}> in system {} conflicts with a previous Res<{0}> access. Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_state.name);
        }
        combined_access.add_write(component_id);

        let resource_archetype = world.archetypes.resource();
        let archetype_component_id = resource_archetype
            .get_archetype_component_id(component_id)
            .unwrap();
        system_state
            .archetype_component_access
            .add_write(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }
}

impl<'a, T: Component> SystemParamFetch<'a> for ResMutState<T> {
    type Item = ResMut<'a, T>;

    #[inline]
    unsafe fn get_param(
        state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        let value = world
            .get_resource_unchecked_mut_with_id(state.component_id)
            .expect("Requested resource does not exist");
        ResMut {
            value: value.value,
            flags: value.flags,
        }
    }
}

pub struct OptionResMutState<T>(ResMutState<T>);

impl<'a, T: Component> SystemParam for Option<ResMut<'a, T>> {
    type Fetch = OptionResMutState<T>;
}

unsafe impl<T: Component> SystemParamState for OptionResMutState<T> {
    type Config = ();

    fn init(world: &mut World, system_state: &mut SystemState, _config: Self::Config) -> Self {
        Self(ResMutState::init(world, system_state, ()))
    }
}

impl<'a, T: Component> SystemParamFetch<'a> for OptionResMutState<T> {
    type Item = Option<ResMut<'a, T>>;

    #[inline]
    unsafe fn get_param(
        state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        world
            .get_resource_unchecked_mut_with_id(state.0.component_id)
            .map(|value| ResMut {
                value: value.value,
                flags: value.flags,
            })
    }
}

impl<'a> SystemParam for Commands<'a> {
    type Fetch = CommandQueue;
}

// SAFE: only local state is accessed
unsafe impl SystemParamState for CommandQueue {
    type Config = ();

    fn init(_world: &mut World, _system_state: &mut SystemState, _config: Self::Config) -> Self {
        Default::default()
    }

    fn apply(&mut self, world: &mut World) {
        self.apply(world);
    }
}

impl<'a> SystemParamFetch<'a> for CommandQueue {
    type Item = Commands<'a>;

    #[inline]
    unsafe fn get_param(
        state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        Commands::new(state, world)
    }
}

pub struct Local<'a, T: Component>(&'a mut T);

impl<'a, T: Component> Deref for Local<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, T: Component> DerefMut for Local<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

pub struct LocalState<T: Component>(T);

impl<'a, T: Component + FromWorld> SystemParam for Local<'a, T> {
    type Fetch = LocalState<T>;
}

// SAFE: only local state is accessed
unsafe impl<T: Component + FromWorld> SystemParamState for LocalState<T> {
    type Config = Option<T>;

    fn init(world: &mut World, _system_state: &mut SystemState, config: Self::Config) -> Self {
        Self(config.unwrap_or_else(|| T::from_world(world)))
    }
}

impl<'a, T: Component + FromWorld> SystemParamFetch<'a> for LocalState<T> {
    type Item = Local<'a, T>;

    #[inline]
    unsafe fn get_param(
        state: &'a mut Self,
        _system_state: &'a SystemState,
        _world: &'a World,
    ) -> Self::Item {
        Local(&mut state.0)
    }
}

pub struct RemovedComponents<'a, T> {
    world: &'a World,
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<'a, T> RemovedComponents<'a, T> {
    pub fn iter(&self) -> std::iter::Cloned<std::slice::Iter<'_, Entity>> {
        self.world.removed_with_id(self.component_id)
    }
}

pub struct RemovedComponentsState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<'a, T: Component> SystemParam for RemovedComponents<'a, T> {
    type Fetch = RemovedComponentsState<T>;
}

// SAFE: no component access. removed component entity collections can be read in parallel and are
// never mutably borrowed during system execution
unsafe impl<T: Component> SystemParamState for RemovedComponentsState<T> {
    type Config = ();

    fn init(world: &mut World, _system_state: &mut SystemState, _config: Self::Config) -> Self {
        Self {
            component_id: world.components.get_or_insert_id::<T>(),
            marker: PhantomData,
        }
    }
}

impl<'a, T: Component> SystemParamFetch<'a> for RemovedComponentsState<T> {
    type Item = RemovedComponents<'a, T>;

    #[inline]
    unsafe fn get_param(
        state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        RemovedComponents {
            world,
            component_id: state.component_id,
            marker: PhantomData,
        }
    }
}

/// Shared borrow of a NonSend resource
pub struct NonSend<'w, T> {
    pub(crate) value: &'w T,
}

impl<'w, T: 'static> Deref for NonSend<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

pub struct NonSendState<T> {
    component_id: ComponentId,
    marker: PhantomData<fn() -> T>,
}

impl<'a, T: 'static> SystemParam for NonSend<'a, T> {
    type Fetch = NonSendState<T>;
}

// SAFE: NonSendComponentId and ArchetypeComponentId access is applied to SystemState. If this
// NonSend conflicts with any prior access, a panic will occur.
unsafe impl<T: 'static> SystemParamState for NonSendState<T> {
    type Config = ();

    fn init(world: &mut World, system_state: &mut SystemState, _config: Self::Config) -> Self {
        system_state.set_non_send();

        let component_id = world.initialize_non_send_resource::<T>();
        let combined_access = system_state.component_access_set.combined_access_mut();
        if combined_access.has_write(component_id) {
            panic!(
                "NonSend<{}> in system {} conflicts with a previous mutable resource access ({0}). Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_state.name);
        }
        combined_access.add_read(component_id);

        let resource_archetype = world.archetypes.resource();
        let archetype_component_id = resource_archetype
            .get_archetype_component_id(component_id)
            .unwrap();
        system_state
            .archetype_component_access
            .add_read(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }
}

impl<'a, T: 'static> SystemParamFetch<'a> for NonSendState<T> {
    type Item = NonSend<'a, T>;

    #[inline]
    unsafe fn get_param(
        state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        NonSend {
            value: world
                .get_non_send_with_id::<T>(state.component_id)
                .expect("Requested non-send resource does not exist"),
        }
    }
}

/// Unique borrow of a NonSend resource
pub struct NonSendMut<'a, T: 'static> {
    pub(crate) value: &'a mut T,
    pub(crate) flags: &'a mut ComponentFlags,
}

impl<'a, T: 'static> Deref for NonSendMut<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T: 'static> DerefMut for NonSendMut<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.flags.insert(ComponentFlags::MUTATED);
        self.value
    }
}

impl<'a, T: 'static + core::fmt::Debug> core::fmt::Debug for NonSendMut<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.value.fmt(f)
    }
}

pub struct NonSendMutState<T> {
    component_id: ComponentId,
    marker: PhantomData<fn() -> T>,
}

impl<'a, T: 'static> SystemParam for NonSendMut<'a, T> {
    type Fetch = NonSendMutState<T>;
}

// SAFE: NonSendMut ComponentId and ArchetypeComponentId access is applied to SystemState. If this
// NonSendMut conflicts with any prior access, a panic will occur.
unsafe impl<T: 'static> SystemParamState for NonSendMutState<T> {
    type Config = ();

    fn init(world: &mut World, system_state: &mut SystemState, _config: Self::Config) -> Self {
        system_state.set_non_send();

        let component_id = world.components.get_or_insert_non_send_resource_id::<T>();
        let combined_access = system_state.component_access_set.combined_access_mut();
        if combined_access.has_write(component_id) {
            panic!(
                "NonSendMut<{}> in system {} conflicts with a previous mutable resource access ({0}). Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_state.name);
        } else if combined_access.has_read(component_id) {
            panic!(
                "NonSendMut<{}> in system {} conflicts with a previous immutable resource access ({0}). Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_state.name);
        }
        combined_access.add_write(component_id);

        let resource_archetype = world.archetypes.resource();
        let archetype_component_id = resource_archetype
            .get_archetype_component_id(component_id)
            .unwrap();
        system_state
            .archetype_component_access
            .add_write(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }
}

impl<'a, T: 'static> SystemParamFetch<'a> for NonSendMutState<T> {
    type Item = NonSendMut<'a, T>;

    #[inline]
    unsafe fn get_param(
        state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        let value = world
            .get_non_send_unchecked_mut_with_id(state.component_id)
            .expect("Requested non-send resource does not exist");
        NonSendMut {
            value: value.value,
            flags: value.flags,
        }
    }
}

pub struct OrState<T>(T);

impl<'a> SystemParam for &'a Archetypes {
    type Fetch = ArchetypesState;
}

pub struct ArchetypesState;

// SAFE: no component value access
unsafe impl SystemParamState for ArchetypesState {
    type Config = ();

    fn init(_world: &mut World, _system_state: &mut SystemState, _config: Self::Config) -> Self {
        Self
    }
}

impl<'a> SystemParamFetch<'a> for ArchetypesState {
    type Item = &'a Archetypes;

    #[inline]
    unsafe fn get_param(
        _state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        world.archetypes()
    }
}

impl<'a> SystemParam for &'a Components {
    type Fetch = ComponentsState;
}

pub struct ComponentsState;

// SAFE: no component value access
unsafe impl SystemParamState for ComponentsState {
    type Config = ();

    fn init(_world: &mut World, _system_state: &mut SystemState, _config: Self::Config) -> Self {
        Self
    }
}

impl<'a> SystemParamFetch<'a> for ComponentsState {
    type Item = &'a Components;

    #[inline]
    unsafe fn get_param(
        _state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        world.components()
    }
}

impl<'a> SystemParam for &'a Entities {
    type Fetch = EntitiesState;
}

pub struct EntitiesState;

// SAFE: no component value access
unsafe impl SystemParamState for EntitiesState {
    type Config = ();

    fn init(_world: &mut World, _system_state: &mut SystemState, _config: Self::Config) -> Self {
        Self
    }
}

impl<'a> SystemParamFetch<'a> for EntitiesState {
    type Item = &'a Entities;

    #[inline]
    unsafe fn get_param(
        _state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        world.entities()
    }
}

impl<'a> SystemParam for &'a Bundles {
    type Fetch = BundlesState;
}

pub struct BundlesState;

// SAFE: no component value access
unsafe impl SystemParamState for BundlesState {
    type Config = ();

    fn init(_world: &mut World, _system_state: &mut SystemState, _config: Self::Config) -> Self {
        Self
    }
}

impl<'a> SystemParamFetch<'a> for BundlesState {
    type Item = &'a Bundles;

    #[inline]
    unsafe fn get_param(
        _state: &'a mut Self,
        _system_state: &'a SystemState,
        world: &'a World,
    ) -> Self::Item {
        world.bundles()
    }
}

macro_rules! impl_system_param_tuple {
    ($($param: ident),*) => {
        impl<$($param: SystemParam),*> SystemParam for ($($param,)*) {
            type Fetch = ($($param::Fetch,)*);
        }
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'a, $($param: SystemParamFetch<'a>),*> SystemParamFetch<'a> for ($($param,)*) {
            type Item = ($($param::Item,)*);

            #[inline]
            unsafe fn get_param(
                state: &'a mut Self,
                system_state: &'a SystemState,
                world: &'a World,
            ) -> Self::Item {

                let ($($param,)*) = state;
                ($($param::get_param($param, system_state, world),)*)
            }
        }

        /// SAFE: implementors of each SystemParamState in the tuple have validated their impls
        #[allow(non_snake_case)]
        unsafe impl<$($param: SystemParamState),*> SystemParamState for ($($param,)*) {
            type Config = ($(<$param as SystemParamState>::Config,)*);
            #[inline]
            fn init(_world: &mut World, _system_state: &mut SystemState, config: Self::Config) -> Self {
                let ($($param,)*) = config;
                (($($param::init(_world, _system_state, $param),)*))
            }

            #[inline]
            fn new_archetype(&mut self, _archetype: &Archetype, _system_state: &mut SystemState) {
                let ($($param,)*) = self;
                $($param.new_archetype(_archetype, _system_state);)*
            }

            #[inline]
            fn apply(&mut self, _world: &mut World) {
                let ($($param,)*) = self;
                $($param.apply(_world);)*
            }
        }
    };
}

// TODO: consider creating a Config trait with a default() function, then implementing that for
// tuples. that would allow us to go past tuples of len 12
all_tuples!(impl_system_param_tuple, 0, 12, P);
