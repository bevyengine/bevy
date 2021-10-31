pub use crate::change_detection::{NonSendMut, ResMut};
use crate::{
    archetype::{Archetype, Archetypes},
    bundle::Bundles,
    change_detection::Ticks,
    component::{Component, ComponentId, ComponentTicks, Components},
    entity::{Entities, Entity},
    query::{
        FilterFetch, FilteredAccess, FilteredAccessSet, QueryState, ReadOnlyFetch, WorldQuery,
    },
    system::{CommandQueue, Commands, Query, SystemMeta},
    world::{FromWorld, World},
};
pub use bevy_ecs_macros::SystemParam;
use bevy_ecs_macros::{all_tuples, impl_query_set};
use std::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A parameter that can be used in a [`System`](super::System).
///
/// # Derive
///
/// This trait can be derived with the [`derive@super::SystemParam`] macro. The only requirement
/// is that every struct field must also implement `SystemParam`.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use std::marker::PhantomData;
/// use bevy_ecs::system::SystemParam;
///
/// #[derive(SystemParam)]
/// struct MyParam<'w, 's> {
///     foo: Res<'w, usize>,
///     #[system_param(ignore)]
///     marker: PhantomData<&'s usize>,
/// }
///
/// fn my_system(param: MyParam) {
///     // Access the resource through `param.foo`
/// }
///
/// # my_system.system();
/// ```
pub trait SystemParam: Sized {
    type Fetch: for<'w, 's> SystemParamFetch<'w, 's>;
}

/// The state of a [`SystemParam`].
///
/// # Safety
///
/// It is the implementor's responsibility to ensure `system_meta` is populated with the _exact_
/// [`World`] access used by the `SystemParamState` (and associated [`SystemParamFetch`]).
/// Additionally, it is the implementor's responsibility to ensure there is no
/// conflicting access across all SystemParams.
pub unsafe trait SystemParamState: Send + Sync + 'static {
    /// Values of this type can be used to adjust the behavior of the
    /// system parameter. For instance, this can be used to pass
    /// values from a `Plugin` to a `System`, or to control the
    /// behavior of the `System`.
    ///
    /// The default configuration of the parameter is set by
    /// [`SystemParamState::default_config`]. To change it, invoke
    /// [`FunctionSystem::config`](super::FunctionSystem::config) when
    /// creating the system.
    ///
    /// See [`FunctionSystem::config`](super::FunctionSystem::config)
    /// for more information and examples.
    type Config: Send + Sync;
    fn init(world: &mut World, system_meta: &mut SystemMeta, config: Self::Config) -> Self;
    #[inline]
    fn new_archetype(&mut self, _archetype: &Archetype, _system_meta: &mut SystemMeta) {}
    #[inline]
    fn apply(&mut self, _world: &mut World) {}
    fn default_config() -> Self::Config;
}

/// A [`SystemParamFetch`] that only reads a given [`World`].
///
/// # Safety
/// This must only be implemented for [`SystemParamFetch`] impls that exclusively read the World passed in to [`SystemParamFetch::get_param`]
pub unsafe trait ReadOnlySystemParamFetch {}

pub trait SystemParamFetch<'world, 'state>: SystemParamState {
    type Item;
    /// # Safety
    ///
    /// This call might access any of the input parameters in an unsafe way. Make sure the data
    /// access is safe in the context of the system scheduler.
    unsafe fn get_param(
        state: &'state mut Self,
        system_meta: &SystemMeta,
        world: &'world World,
        change_tick: u32,
    ) -> Self::Item;
}

impl<'w, 's, Q: WorldQuery + 'static, F: WorldQuery + 'static> SystemParam for Query<'w, 's, Q, F>
where
    F::Fetch: FilterFetch,
{
    type Fetch = QueryState<Q, F>;
}

// SAFE: QueryState is constrained to read-only fetches, so it only reads World.
unsafe impl<Q: WorldQuery, F: WorldQuery> ReadOnlySystemParamFetch for QueryState<Q, F>
where
    Q::Fetch: ReadOnlyFetch,
    F::Fetch: FilterFetch,
{
}

// SAFE: Relevant query ComponentId and ArchetypeComponentId access is applied to SystemMeta. If
// this QueryState conflicts with any prior access, a panic will occur.
unsafe impl<Q: WorldQuery + 'static, F: WorldQuery + 'static> SystemParamState for QueryState<Q, F>
where
    F::Fetch: FilterFetch,
{
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        let state = QueryState::new(world);
        assert_component_access_compatibility(
            &system_meta.name,
            std::any::type_name::<Q>(),
            std::any::type_name::<F>(),
            &system_meta.component_access_set,
            &state.component_access,
            world,
        );
        system_meta
            .component_access_set
            .add(state.component_access.clone());
        system_meta
            .archetype_component_access
            .extend(&state.archetype_component_access);
        state
    }

    fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
        self.new_archetype(archetype);
        system_meta
            .archetype_component_access
            .extend(&self.archetype_component_access);
    }

    fn default_config() {}
}

impl<'w, 's, Q: WorldQuery + 'static, F: WorldQuery + 'static> SystemParamFetch<'w, 's>
    for QueryState<Q, F>
where
    F::Fetch: FilterFetch,
{
    type Item = Query<'w, 's, Q, F>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        Query::new(world, state, system_meta.last_change_tick, change_tick)
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
    panic!("Query<{}, {}> in system {} accesses component(s) {} in a way that conflicts with a previous system parameter. Allowing this would break Rust's mutability rules. Consider using `Without<T>` to create disjoint Queries or merging conflicting Queries into a `QuerySet`.",
                query_type, filter_type, system_name, accesses);
}

pub struct QuerySet<'w, 's, T> {
    query_states: &'s T,
    world: &'w World,
    last_change_tick: u32,
    change_tick: u32,
}

pub struct QuerySetState<T>(T);

impl_query_set!();

pub trait Resource: Send + Sync + 'static {}
impl<T> Resource for T where T: Send + Sync + 'static {}

/// Shared borrow of a resource.
///
/// See the [`World`] documentation to see the usage of a resource.
///
/// If you need a unique mutable borrow, use [`ResMut`] instead.
///
/// # Panics
///
/// Panics when used as a [`SystemParameter`](SystemParam) if the resource does not exist.
///
/// Use `Option<Res<T>>` instead if the resource might not always exist.
pub struct Res<'w, T: Resource> {
    value: &'w T,
    ticks: &'w ComponentTicks,
    last_change_tick: u32,
    change_tick: u32,
}

// SAFE: Res only reads a single World resource
unsafe impl<T: Resource> ReadOnlySystemParamFetch for ResState<T> {}

impl<'w, T: Resource> Debug for Res<'w, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Res").field(&self.value).finish()
    }
}

impl<'w, T: Resource> Res<'w, T> {
    /// Returns true if (and only if) this resource been added since the last execution of this
    /// system.
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_change_tick, self.change_tick)
    }

    /// Returns true if (and only if) this resource been changed since the last execution of this
    /// system.
    pub fn is_changed(&self) -> bool {
        self.ticks
            .is_changed(self.last_change_tick, self.change_tick)
    }

    pub fn into_inner(self) -> &'w T {
        self.value
    }
}

impl<'w, T: Resource> Deref for Res<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T: Resource> AsRef<T> for Res<'w, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

/// The [`SystemParamState`] of [`Res<T>`].
pub struct ResState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<'a, T: Resource> SystemParam for Res<'a, T> {
    type Fetch = ResState<T>;
}

// SAFE: Res ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<T: Resource> SystemParamState for ResState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        let component_id = world.initialize_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access_mut();
        if combined_access.has_write(component_id) {
            panic!(
                "Res<{}> in system {} conflicts with a previous ResMut<{0}> access. Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        }
        combined_access.add_read(component_id);

        let resource_archetype = world.archetypes.resource();
        let archetype_component_id = resource_archetype
            .get_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_read(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }

    fn default_config() {}
}

impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for ResState<T> {
    type Item = Res<'w, T>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let column = world
            .get_populated_resource_column(state.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        Res {
            value: &*column.get_data_ptr().cast::<T>().as_ptr(),
            ticks: column.get_ticks_unchecked(0),
            last_change_tick: system_meta.last_change_tick,
            change_tick,
        }
    }
}

/// The [`SystemParamState`] of [`Option<Res<T>>`].
/// See: [`Res<T>`]
pub struct OptionResState<T>(ResState<T>);

impl<'a, T: Resource> SystemParam for Option<Res<'a, T>> {
    type Fetch = OptionResState<T>;
}

// SAFE: Only reads a single World resource
unsafe impl<T: Resource> ReadOnlySystemParamFetch for OptionResState<T> {}

unsafe impl<T: Resource> SystemParamState for OptionResState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        Self(ResState::init(world, system_meta, ()))
    }

    fn default_config() {}
}

impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for OptionResState<T> {
    type Item = Option<Res<'w, T>>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        world
            .get_populated_resource_column(state.0.component_id)
            .map(|column| Res {
                value: &*column.get_data_ptr().cast::<T>().as_ptr(),
                ticks: column.get_ticks_unchecked(0),
                last_change_tick: system_meta.last_change_tick,
                change_tick,
            })
    }
}

/// The [`SystemParamState`] of [`ResMut<T>`].
pub struct ResMutState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<'a, T: Resource> SystemParam for ResMut<'a, T> {
    type Fetch = ResMutState<T>;
}

// SAFE: Res ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<T: Resource> SystemParamState for ResMutState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        let component_id = world.initialize_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access_mut();
        if combined_access.has_write(component_id) {
            panic!(
                "ResMut<{}> in system {} conflicts with a previous ResMut<{0}> access. Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        } else if combined_access.has_read(component_id) {
            panic!(
                "ResMut<{}> in system {} conflicts with a previous Res<{0}> access. Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        }
        combined_access.add_write(component_id);

        let resource_archetype = world.archetypes.resource();
        let archetype_component_id = resource_archetype
            .get_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_write(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }

    fn default_config() {}
}

impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for ResMutState<T> {
    type Item = ResMut<'w, T>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let value = world
            .get_resource_unchecked_mut_with_id(state.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        ResMut {
            value: value.value,
            ticks: Ticks {
                component_ticks: value.ticks.component_ticks,
                last_change_tick: system_meta.last_change_tick,
                change_tick,
            },
        }
    }
}

/// The [`SystemParamState`] of [`Option<ResMut<T>>`].
/// See: [`ResMut<T>`]
pub struct OptionResMutState<T>(ResMutState<T>);

impl<'a, T: Resource> SystemParam for Option<ResMut<'a, T>> {
    type Fetch = OptionResMutState<T>;
}

unsafe impl<T: Resource> SystemParamState for OptionResMutState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        Self(ResMutState::init(world, system_meta, ()))
    }

    fn default_config() {}
}

impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for OptionResMutState<T> {
    type Item = Option<ResMut<'w, T>>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        world
            .get_resource_unchecked_mut_with_id(state.0.component_id)
            .map(|value| ResMut {
                value: value.value,
                ticks: Ticks {
                    component_ticks: value.ticks.component_ticks,
                    last_change_tick: system_meta.last_change_tick,
                    change_tick,
                },
            })
    }
}

impl<'w, 's> SystemParam for Commands<'w, 's> {
    type Fetch = CommandQueue;
}

// SAFE: Commands only accesses internal state
unsafe impl ReadOnlySystemParamFetch for CommandQueue {}

// SAFE: only local state is accessed
unsafe impl SystemParamState for CommandQueue {
    type Config = ();

    fn init(_world: &mut World, _system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        Default::default()
    }

    fn apply(&mut self, world: &mut World) {
        self.apply(world);
    }

    fn default_config() {}
}

impl<'w, 's> SystemParamFetch<'w, 's> for CommandQueue {
    type Item = Commands<'w, 's>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        Commands::new(state, world)
    }
}

/// A system local [`SystemParam`].
///
/// A local may only be accessed by the system itself and is therefore not visible to other systems.
/// If two or more systems specify the same local type each will have their own unique local.
///
/// # Examples
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let world = &mut World::default();
/// fn write_to_local(mut local: Local<usize>) {
///     *local = 42;
/// }
/// fn read_from_local(local: Local<usize>) -> usize {
///     *local
/// }
/// let mut write_system = write_to_local.system();
/// let mut read_system = read_from_local.system();
/// write_system.initialize(world);
/// read_system.initialize(world);
///
/// assert_eq!(read_system.run((), world), 0);
/// write_system.run((), world);
/// // Note how the read local is still 0 due to the locals not being shared.
/// assert_eq!(read_system.run((), world), 0);
/// ```
pub struct Local<'a, T: Resource>(&'a mut T);

// SAFE: Local only accesses internal state
unsafe impl<T: Resource> ReadOnlySystemParamFetch for LocalState<T> {}

impl<'a, T: Resource> Debug for Local<'a, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Local").field(&self.0).finish()
    }
}

impl<'a, T: Resource> Deref for Local<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, T: Resource> DerefMut for Local<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

/// The [`SystemParamState`] of [`Local<T>`].
pub struct LocalState<T: Resource>(T);

impl<'a, T: Resource + FromWorld> SystemParam for Local<'a, T> {
    type Fetch = LocalState<T>;
}

// SAFE: only local state is accessed
unsafe impl<T: Resource + FromWorld> SystemParamState for LocalState<T> {
    type Config = Option<T>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta, config: Self::Config) -> Self {
        Self(config.unwrap_or_else(|| T::from_world(world)))
    }

    fn default_config() -> Option<T> {
        None
    }
}

impl<'w, 's, T: Resource + FromWorld> SystemParamFetch<'w, 's> for LocalState<T> {
    type Item = Local<'s, T>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        Local(&mut state.0)
    }
}

/// A [`SystemParam`] that grants access to the entities that had their `T` [`Component`] removed.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::RemovedComponents;
/// #
/// # #[derive(Component)]
/// # struct MyComponent;
///
/// fn react_on_removal(removed: RemovedComponents<MyComponent>) {
///     removed.iter().for_each(|removed_entity| println!("{:?}", removed_entity));
/// }
///
/// # react_on_removal.system();
/// ```
pub struct RemovedComponents<'a, T: Component> {
    world: &'a World,
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<'a, T: Component> RemovedComponents<'a, T> {
    /// Returns an iterator over the entities that had their `T` [`Component`] removed.
    pub fn iter(&self) -> std::iter::Cloned<std::slice::Iter<'_, Entity>> {
        self.world.removed_with_id(self.component_id)
    }
}

// SAFE: Only reads World components
unsafe impl<T: Component> ReadOnlySystemParamFetch for RemovedComponentsState<T> {}

/// The [`SystemParamState`] of [`RemovedComponents<T>`].
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

    fn init(world: &mut World, _system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        Self {
            component_id: world.init_component::<T>(),
            marker: PhantomData,
        }
    }

    fn default_config() {}
}

impl<'w, 's, T: Component> SystemParamFetch<'w, 's> for RemovedComponentsState<T> {
    type Item = RemovedComponents<'w, T>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        RemovedComponents {
            world,
            component_id: state.component_id,
            marker: PhantomData,
        }
    }
}

/// Shared borrow of a non-[`Send`] resource.
///
/// Only `Send` resources may be accessed with the [`Res`] [`SystemParam`]. In case that the
/// resource does not implement `Send`, this `SystemParam` wrapper can be used. This will instruct
/// the scheduler to instead run the system on the main thread so that it doesn't send the resource
/// over to another thread.
///
/// # Panics
///
/// Panics when used as a `SystemParameter` if the resource does not exist.
///
/// Use `Option<NonSend<T>>` instead if the resource might not always exist.
pub struct NonSend<'w, T: 'static> {
    pub(crate) value: &'w T,
    ticks: ComponentTicks,
    last_change_tick: u32,
    change_tick: u32,
}

// SAFE: Only reads a single World non-send resource
unsafe impl<T> ReadOnlySystemParamFetch for NonSendState<T> {}

impl<'w, T> Debug for NonSend<'w, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NonSend").field(&self.value).finish()
    }
}

impl<'w, T: 'static> NonSend<'w, T> {
    /// Returns true if (and only if) this resource been added since the last execution of this
    /// system.
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_change_tick, self.change_tick)
    }

    /// Returns true if (and only if) this resource been changed since the last execution of this
    /// system.
    pub fn is_changed(&self) -> bool {
        self.ticks
            .is_changed(self.last_change_tick, self.change_tick)
    }
}

impl<'w, T> Deref for NonSend<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

/// The [`SystemParamState`] of [`NonSend<T>`].
pub struct NonSendState<T> {
    component_id: ComponentId,
    marker: PhantomData<fn() -> T>,
}

impl<'a, T: 'static> SystemParam for NonSend<'a, T> {
    type Fetch = NonSendState<T>;
}

// SAFE: NonSendComponentId and ArchetypeComponentId access is applied to SystemMeta. If this
// NonSend conflicts with any prior access, a panic will occur.
unsafe impl<T: 'static> SystemParamState for NonSendState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        system_meta.set_non_send();

        let component_id = world.initialize_non_send_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access_mut();
        if combined_access.has_write(component_id) {
            panic!(
                "NonSend<{}> in system {} conflicts with a previous mutable resource access ({0}). Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        }
        combined_access.add_read(component_id);

        let resource_archetype = world.archetypes.resource();
        let archetype_component_id = resource_archetype
            .get_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_read(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }

    fn default_config() {}
}

impl<'w, 's, T: 'static> SystemParamFetch<'w, 's> for NonSendState<T> {
    type Item = NonSend<'w, T>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        world.validate_non_send_access::<T>();
        let column = world
            .get_populated_resource_column(state.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Non-send resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });

        NonSend {
            value: &*column.get_data_ptr().cast::<T>().as_ptr(),
            ticks: column.get_ticks_unchecked(0).clone(),
            last_change_tick: system_meta.last_change_tick,
            change_tick,
        }
    }
}

/// The [`SystemParamState`] of [`Option<NonSend<T>>`].
/// See: [`NonSend<T>`]
pub struct OptionNonSendState<T>(NonSendState<T>);

impl<'w, T: 'static> SystemParam for Option<NonSend<'w, T>> {
    type Fetch = OptionNonSendState<T>;
}

// SAFE: Only reads a single non-send resource
unsafe impl<T: 'static> ReadOnlySystemParamFetch for OptionNonSendState<T> {}

unsafe impl<T: 'static> SystemParamState for OptionNonSendState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        Self(NonSendState::init(world, system_meta, ()))
    }

    fn default_config() {}
}

impl<'w, 's, T: 'static> SystemParamFetch<'w, 's> for OptionNonSendState<T> {
    type Item = Option<NonSend<'w, T>>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        world.validate_non_send_access::<T>();
        world
            .get_populated_resource_column(state.0.component_id)
            .map(|column| NonSend {
                value: &*column.get_data_ptr().cast::<T>().as_ptr(),
                ticks: column.get_ticks_unchecked(0).clone(),
                last_change_tick: system_meta.last_change_tick,
                change_tick,
            })
    }
}

/// The [`SystemParamState`] of [`NonSendMut<T>`].
pub struct NonSendMutState<T> {
    component_id: ComponentId,
    marker: PhantomData<fn() -> T>,
}

impl<'a, T: 'static> SystemParam for NonSendMut<'a, T> {
    type Fetch = NonSendMutState<T>;
}

// SAFE: NonSendMut ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this
// NonSendMut conflicts with any prior access, a panic will occur.
unsafe impl<T: 'static> SystemParamState for NonSendMutState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        system_meta.set_non_send();

        let component_id = world.initialize_non_send_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access_mut();
        if combined_access.has_write(component_id) {
            panic!(
                "NonSendMut<{}> in system {} conflicts with a previous mutable resource access ({0}). Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        } else if combined_access.has_read(component_id) {
            panic!(
                "NonSendMut<{}> in system {} conflicts with a previous immutable resource access ({0}). Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        }
        combined_access.add_write(component_id);

        let resource_archetype = world.archetypes.resource();
        let archetype_component_id = resource_archetype
            .get_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_write(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }

    fn default_config() {}
}

impl<'w, 's, T: 'static> SystemParamFetch<'w, 's> for NonSendMutState<T> {
    type Item = NonSendMut<'w, T>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        world.validate_non_send_access::<T>();
        let column = world
            .get_populated_resource_column(state.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Non-send resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        NonSendMut {
            value: &mut *column.get_data_ptr().cast::<T>().as_ptr(),
            ticks: Ticks {
                component_ticks: &mut *column.get_ticks_mut_ptr_unchecked(0),
                last_change_tick: system_meta.last_change_tick,
                change_tick,
            },
        }
    }
}

/// The [`SystemParamState`] of [`Option<NonSendMut<T>>`].
/// See: [`NonSendMut<T>`]
pub struct OptionNonSendMutState<T>(NonSendMutState<T>);

impl<'a, T: 'static> SystemParam for Option<NonSendMut<'a, T>> {
    type Fetch = OptionNonSendMutState<T>;
}

unsafe impl<T: 'static> SystemParamState for OptionNonSendMutState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        Self(NonSendMutState::init(world, system_meta, ()))
    }

    fn default_config() {}
}

impl<'w, 's, T: 'static> SystemParamFetch<'w, 's> for OptionNonSendMutState<T> {
    type Item = Option<NonSendMut<'w, T>>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        world.validate_non_send_access::<T>();
        world
            .get_populated_resource_column(state.0.component_id)
            .map(|column| NonSendMut {
                value: &mut *column.get_data_ptr().cast::<T>().as_ptr(),
                ticks: Ticks {
                    component_ticks: &mut *column.get_ticks_mut_ptr_unchecked(0),
                    last_change_tick: system_meta.last_change_tick,
                    change_tick,
                },
            })
    }
}

impl<'a> SystemParam for &'a Archetypes {
    type Fetch = ArchetypesState;
}

// SAFE: Only reads World archetypes
unsafe impl ReadOnlySystemParamFetch for ArchetypesState {}

/// The [`SystemParamState`] of [`Archetypes`].
pub struct ArchetypesState;

// SAFE: no component value access
unsafe impl SystemParamState for ArchetypesState {
    type Config = ();

    fn init(_world: &mut World, _system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        Self
    }

    fn default_config() {}
}

impl<'w, 's> SystemParamFetch<'w, 's> for ArchetypesState {
    type Item = &'w Archetypes;

    #[inline]
    unsafe fn get_param(
        _state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        world.archetypes()
    }
}

impl<'a> SystemParam for &'a Components {
    type Fetch = ComponentsState;
}

// SAFE: Only reads World components
unsafe impl ReadOnlySystemParamFetch for ComponentsState {}

/// The [`SystemParamState`] of [`Components`].
pub struct ComponentsState;

// SAFE: no component value access
unsafe impl SystemParamState for ComponentsState {
    type Config = ();

    fn init(_world: &mut World, _system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        Self
    }

    fn default_config() {}
}

impl<'w, 's> SystemParamFetch<'w, 's> for ComponentsState {
    type Item = &'w Components;

    #[inline]
    unsafe fn get_param(
        _state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        world.components()
    }
}

impl<'a> SystemParam for &'a Entities {
    type Fetch = EntitiesState;
}

// SAFE: Only reads World entities
unsafe impl ReadOnlySystemParamFetch for EntitiesState {}

/// The [`SystemParamState`] of [`Entities`].
pub struct EntitiesState;

// SAFE: no component value access
unsafe impl SystemParamState for EntitiesState {
    type Config = ();

    fn init(_world: &mut World, _system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        Self
    }

    fn default_config() {}
}

impl<'w, 's> SystemParamFetch<'w, 's> for EntitiesState {
    type Item = &'w Entities;

    #[inline]
    unsafe fn get_param(
        _state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        world.entities()
    }
}

impl<'a> SystemParam for &'a Bundles {
    type Fetch = BundlesState;
}

// SAFE: Only reads World bundles
unsafe impl ReadOnlySystemParamFetch for BundlesState {}

/// The [`SystemParamState`] of [`Bundles`].
pub struct BundlesState;

// SAFE: no component value access
unsafe impl SystemParamState for BundlesState {
    type Config = ();

    fn init(_world: &mut World, _system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        Self
    }

    fn default_config() {}
}

impl<'w, 's> SystemParamFetch<'w, 's> for BundlesState {
    type Item = &'w Bundles;

    #[inline]
    unsafe fn get_param(
        _state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        world.bundles()
    }
}

#[derive(Debug)]
pub struct SystemChangeTick {
    pub last_change_tick: u32,
    pub change_tick: u32,
}

// SAFE: Only reads internal system state
unsafe impl ReadOnlySystemParamFetch for SystemChangeTickState {}

impl SystemParam for SystemChangeTick {
    type Fetch = SystemChangeTickState;
}

/// The [`SystemParamState`] of [`SystemChangeTick`].
pub struct SystemChangeTickState {}

unsafe impl SystemParamState for SystemChangeTickState {
    type Config = ();

    fn init(_world: &mut World, _system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        Self {}
    }

    fn default_config() {}
}

impl<'w, 's> SystemParamFetch<'w, 's> for SystemChangeTickState {
    type Item = SystemChangeTick;

    unsafe fn get_param(
        _state: &'s mut Self,
        system_meta: &SystemMeta,
        _world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        SystemChangeTick {
            last_change_tick: system_meta.last_change_tick,
            change_tick,
        }
    }
}

macro_rules! impl_system_param_tuple {
    ($($param: ident),*) => {
        impl<$($param: SystemParam),*> SystemParam for ($($param,)*) {
            type Fetch = ($($param::Fetch,)*);
        }

        // SAFE: tuple consists only of ReadOnlySystemParamFetches
        unsafe impl<$($param: ReadOnlySystemParamFetch),*> ReadOnlySystemParamFetch for ($($param,)*) {}

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'w, 's, $($param: SystemParamFetch<'w, 's>),*> SystemParamFetch<'w, 's> for ($($param,)*) {
            type Item = ($($param::Item,)*);

            #[inline]
            #[allow(clippy::unused_unit)]
            unsafe fn get_param(
                state: &'s mut Self,
                system_meta: &SystemMeta,
                world: &'w World,
                change_tick: u32,
            ) -> Self::Item {

                let ($($param,)*) = state;
                ($($param::get_param($param, system_meta, world, change_tick),)*)
            }
        }

        /// SAFE: implementors of each SystemParamState in the tuple have validated their impls
        #[allow(non_snake_case)]
        unsafe impl<$($param: SystemParamState),*> SystemParamState for ($($param,)*) {
            type Config = ($(<$param as SystemParamState>::Config,)*);
            #[inline]
            fn init(_world: &mut World, _system_meta: &mut SystemMeta, config: Self::Config) -> Self {
                let ($($param,)*) = config;
                (($($param::init(_world, _system_meta, $param),)*))
            }

            #[inline]
            fn new_archetype(&mut self, _archetype: &Archetype, _system_meta: &mut SystemMeta) {
                let ($($param,)*) = self;
                $($param.new_archetype(_archetype, _system_meta);)*
            }

            #[inline]
            fn apply(&mut self, _world: &mut World) {
                let ($($param,)*) = self;
                $($param.apply(_world);)*
            }

            #[allow(clippy::unused_unit)]
            fn default_config() -> ($(<$param as SystemParamState>::Config,)*) {
                ($(<$param as SystemParamState>::default_config(),)*)
            }
        }
    };
}

all_tuples!(impl_system_param_tuple, 0, 16, P);
