pub use crate::change_detection::{NonSendMut, ResMut};
use crate::{
    archetype::{Archetype, Archetypes},
    bundle::Bundles,
    change_detection::Ticks,
    component::{Component, ComponentId, ComponentTicks, Components, Tick},
    entity::{Entities, Entity},
    query::{
        Access, FilteredAccess, FilteredAccessSet, QueryState, ReadOnlyWorldQuery, WorldQuery,
    },
    system::{CommandQueue, Commands, Query, SystemMeta},
    world::{FromWorld, World},
};
pub use bevy_ecs_macros::Resource;
pub use bevy_ecs_macros::SystemParam;
use bevy_ecs_macros::{all_tuples, impl_param_set};
use bevy_ptr::UnsafeCellDeref;
use bevy_utils::synccell::SyncCell;
use std::{
    borrow::Cow,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A parameter that can be used in a [`System`](super::System).
///
/// # Derive
///
/// This trait can be derived with the [`derive@super::SystemParam`] macro.
/// This macro only works if each field on the derived struct implements [`SystemParam`].
/// Note: There are additional requirements on the field types.
/// See the *Generic `SystemParam`s* section for details and workarounds of the probable
/// cause if this derive causes an error to be emitted.
///
/// Derived `SystemParam` structs may have two lifetimes: `'w` for data stored in the [`World`],
/// and `'s` for data stored in the parameter's state.
///
/// ## Attributes
///
/// `#[system_param(ignore)]`:
/// Can be added to any field in the struct. Fields decorated with this attribute
/// will be created with the default value upon realisation.
/// This is most useful for `PhantomData` fields, such as markers for generic types.
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Resource)]
/// # struct SomeResource;
/// use std::marker::PhantomData;
/// use bevy_ecs::system::SystemParam;
///
/// #[derive(SystemParam)]
/// struct MyParam<'w, Marker: 'static> {
///     foo: Res<'w, SomeResource>,
///     #[system_param(ignore)]
///     marker: PhantomData<Marker>,
/// }
///
/// fn my_system<T: 'static>(param: MyParam<T>) {
///     // Access the resource through `param.foo`
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system::<()>);
/// ```
///
/// # Generic `SystemParam`s
///
/// When using the derive macro, you may see an error in the form of:
///
/// ```text
/// expected ... [ParamType]
/// found associated type `<<[ParamType] as SystemParam>::State as SystemParamState>::Item<'_, '_>`
/// ```
/// where `[ParamType]` is the type of one of your fields.
/// To solve this error, you can wrap the field of type `[ParamType]` with [`StaticSystemParam`]
/// (i.e. `StaticSystemParam<[ParamType]>`).
///
/// ## Details
///
/// The derive macro requires that the [`SystemParam`] implementation of
/// each field `F`'s [`State`](`SystemParam::State`)'s [`Item`](`SystemParamState::Item`) is itself `F`
/// (ignoring lifetimes for simplicity).
/// This assumption is due to type inference reasons, so that the derived [`SystemParam`] can be
/// used as an argument to a function system.
/// If the compiler cannot validate this property for `[ParamType]`, it will error in the form shown above.
///
/// This will most commonly occur when working with `SystemParam`s generically, as the requirement
/// has not been proven to the compiler.
///
/// # `!Sync` Resources
/// A `!Sync` type cannot implement `Resource`. However, it is possible to wrap a `Send` but not `Sync`
/// type in [`SyncCell`] or the currently unstable [`Exclusive`] to make it `Sync`. This forces only
/// having mutable access (`&mut T` only, never `&T`), but makes it safe to reference across multiple
/// threads.
///
/// This will fail to compile since `RefCell` is `!Sync`.
/// ```compile_fail
/// # use std::cell::RefCell;
/// # use bevy_ecs::system::Resource;
///
/// #[derive(Resource)]
/// struct NotSync {
///    counter: RefCell<usize>,
/// }
/// ```
///
/// This will compile since the `RefCell` is wrapped with `SyncCell`.
/// ```
/// # use std::cell::RefCell;
/// # use bevy_ecs::system::Resource;
/// use bevy_utils::synccell::SyncCell;
///
/// #[derive(Resource)]
/// struct ActuallySync {
///    counter: SyncCell<RefCell<usize>>,
/// }
/// ```
///
/// [`SyncCell`]: bevy_utils::synccell::SyncCell
/// [`Exclusive`]: https://doc.rust-lang.org/nightly/std/sync/struct.Exclusive.html
pub trait SystemParam: Sized {
    type State: SystemParamState;
}

pub type SystemParamItem<'w, 's, P> = <<P as SystemParam>::State as SystemParamState>::Item<'w, 's>;

/// The state of a [`SystemParam`].
///
/// # Safety
///
/// It is the implementor's responsibility to ensure `system_meta` is populated with the _exact_
/// [`World`] access used by the [`SystemParamState`].
/// Additionally, it is the implementor's responsibility to ensure there is no
/// conflicting access across all [`SystemParam`]'s.
pub unsafe trait SystemParamState: Send + Sync + 'static {
    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self;
    #[inline]
    fn new_archetype(&mut self, _archetype: &Archetype, _system_meta: &mut SystemMeta) {}
    #[inline]
    #[allow(unused_variables)]
    fn apply(&mut self, system_meta: &SystemMeta, _world: &mut World) {}

    type Item<'world, 'state>: SystemParam<State = Self>;
    /// # Safety
    ///
    /// This call might access any of the input parameters in an unsafe way. Make sure the data
    /// access is safe in the context of the system scheduler.
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self,
        system_meta: &SystemMeta,
        world: &'world World,
        change_tick: u32,
    ) -> Self::Item<'world, 'state>;
}

/// A [`SystemParam`] that only reads a given [`World`].
///
/// # Safety
/// This must only be implemented for [`SystemParam`] impls that exclusively read the World passed in to [`SystemParamState::get_param`]
pub unsafe trait ReadOnlySystemParam: SystemParam {}

impl<'w, 's, Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static> SystemParam
    for Query<'w, 's, Q, F>
{
    type State = QueryState<Q, F>;
}

// SAFETY: QueryState is constrained to read-only fetches, so it only reads World.
unsafe impl<'w, 's, Q: ReadOnlyWorldQuery + 'static, F: ReadOnlyWorldQuery + 'static>
    ReadOnlySystemParam for Query<'w, 's, Q, F>
{
}

// SAFETY: Relevant query ComponentId and ArchetypeComponentId access is applied to SystemMeta. If
// this QueryState conflicts with any prior access, a panic will occur.
unsafe impl<Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static> SystemParamState
    for QueryState<Q, F>
{
    type Item<'w, 's> = Query<'w, 's, Q, F>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
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

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item<'w, 's> {
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
    let conflicts = system_access.get_conflicts_single(current);
    if conflicts.is_empty() {
        return;
    }
    let conflicting_components = conflicts
        .into_iter()
        .map(|component_id| world.components.get_info(component_id).unwrap().name())
        .collect::<Vec<&str>>();
    let accesses = conflicting_components.join(", ");
    panic!("error[B0001]: Query<{}, {}> in system {} accesses component(s) {} in a way that conflicts with a previous system parameter. Consider using `Without<T>` to create disjoint Queries or merging conflicting Queries into a `ParamSet`.",
           query_type, filter_type, system_name, accesses);
}

pub struct ParamSet<'w, 's, T: SystemParam> {
    param_states: &'s mut T::State,
    world: &'w World,
    system_meta: SystemMeta,
    change_tick: u32,
}
/// The [`SystemParamState`] of [`ParamSet<T::Item>`].
pub struct ParamSetState<T: SystemParamState>(T);

impl_param_set!();

/// A type that can be inserted into a [`World`] as a singleton.
///
/// You can access resource data in systems using the [`Res`] and [`ResMut`] system parameters
///
/// Only one resource of each type can be stored in a [`World`] at any given time.
///
/// # Examples
///
/// ```
/// # let mut world = World::default();
/// # let mut schedule = Schedule::default();
/// # schedule.add_stage("update", SystemStage::parallel());
/// # use bevy_ecs::prelude::*;
/// #[derive(Resource)]
/// struct MyResource { value: u32 }
///
/// world.insert_resource(MyResource { value: 42 });
///
/// fn read_resource_system(resource: Res<MyResource>) {
///     assert_eq!(resource.value, 42);
/// }
///
/// fn write_resource_system(mut resource: ResMut<MyResource>) {
///     assert_eq!(resource.value, 42);
///     resource.value = 0;
///     assert_eq!(resource.value, 0);
/// }
/// # schedule.add_system_to_stage("update", read_resource_system.label("first"));
/// # schedule.add_system_to_stage("update", write_resource_system.after("first"));
/// # schedule.run_once(&mut world);
/// ```
pub trait Resource: Send + Sync + 'static {}

/// Shared borrow of a [`Resource`].
///
/// See the [`Resource`] documentation for usage.
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
    added: &'w Tick,
    changed: &'w Tick,
    last_change_tick: u32,
    change_tick: u32,
}

// SAFETY: Res only reads a single World resource
unsafe impl<'w, T: Resource> ReadOnlySystemParam for Res<'w, T> {}

impl<'w, T: Resource> Debug for Res<'w, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Res").field(&self.value).finish()
    }
}

impl<'w, T: Resource> Res<'w, T> {
    // no it shouldn't clippy
    #[allow(clippy::should_implement_trait)]
    pub fn clone(this: &Self) -> Self {
        Self {
            value: this.value,
            added: this.added,
            changed: this.changed,
            last_change_tick: this.last_change_tick,
            change_tick: this.change_tick,
        }
    }

    /// Returns `true` if the resource was added after the system last ran.
    pub fn is_added(&self) -> bool {
        self.added
            .is_older_than(self.last_change_tick, self.change_tick)
    }

    /// Returns `true` if the resource was added or mutably dereferenced after the system last ran.
    pub fn is_changed(&self) -> bool {
        self.changed
            .is_older_than(self.last_change_tick, self.change_tick)
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

impl<'w, T: Resource> From<ResMut<'w, T>> for Res<'w, T> {
    fn from(res: ResMut<'w, T>) -> Self {
        Self {
            value: res.value,
            added: res.ticks.added,
            changed: res.ticks.changed,
            change_tick: res.ticks.change_tick,
            last_change_tick: res.ticks.last_change_tick,
        }
    }
}

impl<'w, 'a, T: Resource> IntoIterator for &'a Res<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}

/// The [`SystemParamState`] of [`Res<T>`].
#[doc(hidden)]
pub struct ResState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<'a, T: Resource> SystemParam for Res<'a, T> {
    type State = ResState<T>;
}

// SAFETY: Res ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<T: Resource> SystemParamState for ResState<T> {
    type Item<'w, 's> = Res<'w, T>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        let component_id = world.initialize_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access();
        assert!(
            !combined_access.has_write(component_id),
            "error[B0002]: Res<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access.",
            std::any::type_name::<T>(),
            system_meta.name,
        );
        system_meta
            .component_access_set
            .add_unfiltered_read(component_id);

        let archetype_component_id = world
            .get_resource_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_read(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks) = world
            .get_resource_with_ticks(state.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        Res {
            value: ptr.deref(),
            added: ticks.added.deref(),
            changed: ticks.changed.deref(),
            last_change_tick: system_meta.last_change_tick,
            change_tick,
        }
    }
}

/// The [`SystemParamState`] of [`Option<Res<T>>`].
/// See: [`Res<T>`]
#[doc(hidden)]
pub struct OptionResState<T>(ResState<T>);

impl<'a, T: Resource> SystemParam for Option<Res<'a, T>> {
    type State = OptionResState<T>;
}

// SAFETY: Only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for Option<Res<'a, T>> {}

// SAFETY: this impl defers to `ResState`, which initializes
// and validates the correct world access
unsafe impl<T: Resource> SystemParamState for OptionResState<T> {
    type Item<'w, 's> = Option<Res<'w, T>>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        Self(ResState::init(world, system_meta))
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world
            .get_resource_with_ticks(state.0.component_id)
            .map(|(ptr, ticks)| Res {
                value: ptr.deref(),
                added: ticks.added.deref(),
                changed: ticks.changed.deref(),
                last_change_tick: system_meta.last_change_tick,
                change_tick,
            })
    }
}

/// The [`SystemParamState`] of [`ResMut<T>`].
#[doc(hidden)]
pub struct ResMutState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<'a, T: Resource> SystemParam for ResMut<'a, T> {
    type State = ResMutState<T>;
}

// SAFETY: Res ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<T: Resource> SystemParamState for ResMutState<T> {
    type Item<'w, 's> = ResMut<'w, T>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        let component_id = world.initialize_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access();
        if combined_access.has_write(component_id) {
            panic!(
                "error[B0002]: ResMut<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        } else if combined_access.has_read(component_id) {
            panic!(
                "error[B0002]: ResMut<{}> in system {} conflicts with a previous Res<{0}> access. Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        }
        system_meta
            .component_access_set
            .add_unfiltered_write(component_id);

        let archetype_component_id = world
            .get_resource_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_write(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item<'w, 's> {
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
                added: value.ticks.added,
                changed: value.ticks.changed,
                last_change_tick: system_meta.last_change_tick,
                change_tick,
            },
        }
    }
}

/// The [`SystemParamState`] of [`Option<ResMut<T>>`].
/// See: [`ResMut<T>`]
#[doc(hidden)]
pub struct OptionResMutState<T>(ResMutState<T>);

impl<'a, T: Resource> SystemParam for Option<ResMut<'a, T>> {
    type State = OptionResMutState<T>;
}

// SAFETY: this impl defers to `ResMutState`, which initializes
// and validates the correct world access
unsafe impl<T: Resource> SystemParamState for OptionResMutState<T> {
    type Item<'w, 's> = Option<ResMut<'w, T>>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        Self(ResMutState::init(world, system_meta))
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world
            .get_resource_unchecked_mut_with_id(state.0.component_id)
            .map(|value| ResMut {
                value: value.value,
                ticks: Ticks {
                    added: value.ticks.added,
                    changed: value.ticks.changed,
                    last_change_tick: system_meta.last_change_tick,
                    change_tick,
                },
            })
    }
}

impl<'w, 's> SystemParam for Commands<'w, 's> {
    type State = CommandQueue;
}

// SAFETY: Commands only accesses internal state
unsafe impl<'w, 's> ReadOnlySystemParam for Commands<'w, 's> {}

// SAFETY: only local state is accessed
unsafe impl SystemParamState for CommandQueue {
    type Item<'w, 's> = Commands<'w, 's>;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        Default::default()
    }

    fn apply(&mut self, _system_meta: &SystemMeta, world: &mut World) {
        #[cfg(feature = "trace")]
        let _system_span =
            bevy_utils::tracing::info_span!("system_commands", name = _system_meta.name())
                .entered();
        self.apply(world);
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item<'w, 's> {
        Commands::new(state, world)
    }
}

/// SAFETY: only reads world
unsafe impl<'w> ReadOnlySystemParam for &'w World {}

/// The [`SystemParamState`] of [`&World`](crate::world::World).
#[doc(hidden)]
pub struct WorldState;

impl<'w> SystemParam for &'w World {
    type State = WorldState;
}

// SAFETY: `read_all` access is set and conflicts result in a panic
unsafe impl SystemParamState for WorldState {
    type Item<'w, 's> = &'w World;

    fn init(_world: &mut World, system_meta: &mut SystemMeta) -> Self {
        let mut access = Access::default();
        access.read_all();
        if !system_meta
            .archetype_component_access
            .is_compatible(&access)
        {
            panic!("&World conflicts with a previous mutable system parameter. Allowing this would break Rust's mutability rules");
        }
        system_meta.archetype_component_access.extend(&access);

        let mut filtered_access = FilteredAccess::default();

        filtered_access.read_all();
        if !system_meta
            .component_access_set
            .get_conflicts_single(&filtered_access)
            .is_empty()
        {
            panic!("&World conflicts with a previous mutable system parameter. Allowing this would break Rust's mutability rules");
        }
        system_meta.component_access_set.add(filtered_access);

        WorldState
    }

    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world
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
/// let mut write_system = IntoSystem::into_system(write_to_local);
/// let mut read_system = IntoSystem::into_system(read_from_local);
/// write_system.initialize(world);
/// read_system.initialize(world);
///
/// assert_eq!(read_system.run((), world), 0);
/// write_system.run((), world);
/// // Note how the read local is still 0 due to the locals not being shared.
/// assert_eq!(read_system.run((), world), 0);
/// ```
///
/// N.B. A [`Local`]s value cannot be read or written to outside of the containing system.
/// To add configuration to a system, convert a capturing closure into the system instead:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::assert_is_system;
/// struct Config(u32);
/// #[derive(Resource)]
/// struct Myu32Wrapper(u32);
/// fn reset_to_system(value: Config) -> impl FnMut(ResMut<Myu32Wrapper>) {
///     move |mut val| val.0 = value.0
/// }
///
/// // .add_system(reset_to_system(my_config))
/// # assert_is_system(reset_to_system(Config(10)));
/// ```
pub struct Local<'a, T: FromWorld + Send + 'static>(pub(crate) &'a mut T);

// SAFETY: Local only accesses internal state
unsafe impl<'a, T: FromWorld + Send + 'static> ReadOnlySystemParam for Local<'a, T> {}

impl<'a, T: FromWorld + Send + Sync + 'static> Debug for Local<'a, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Local").field(&self.0).finish()
    }
}

impl<'a, T: FromWorld + Send + Sync + 'static> Deref for Local<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, T: FromWorld + Send + Sync + 'static> DerefMut for Local<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'w, 'a, T: FromWorld + Send + 'static> IntoIterator for &'a Local<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'w, 'a, T: FromWorld + Send + 'static> IntoIterator for &'a mut Local<'w, T>
where
    &'a mut T: IntoIterator,
{
    type Item = <&'a mut T as IntoIterator>::Item;
    type IntoIter = <&'a mut T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// The [`SystemParamState`] of [`Local<T>`].
#[doc(hidden)]
pub struct LocalState<T: Send + 'static>(pub(crate) SyncCell<T>);

impl<'a, T: FromWorld + Send + 'static> SystemParam for Local<'a, T> {
    type State = LocalState<T>;
}

// SAFETY: only local state is accessed
unsafe impl<T: FromWorld + Send + 'static> SystemParamState for LocalState<T> {
    type Item<'w, 's> = Local<'s, T>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        Self(SyncCell::new(T::from_world(world)))
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Item<'w, 's> {
        Local(state.0.get())
    }
}

/// A [`SystemParam`] that grants access to the entities that had their `T` [`Component`] removed.
///
/// Note that this does not allow you to see which data existed before removal.
/// If you need this, you will need to track the component data value on your own,
/// using a regularly scheduled system that requests `Query<(Entity, &T), Changed<T>>`
/// and stores the data somewhere safe to later cross-reference.
///
/// If you are using `bevy_ecs` as a standalone crate,
/// note that the `RemovedComponents` list will not be automatically cleared for you,
/// and will need to be manually flushed using [`World::clear_trackers`]
///
/// For users of `bevy` and `bevy_app`, this is automatically done in `bevy_app::App::update`.
/// For the main world, [`World::clear_trackers`] is run after the main schedule is run and after
/// `SubApp`'s have run.
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
/// # bevy_ecs::system::assert_is_system(react_on_removal);
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

impl<'a, T: Component> IntoIterator for &'a RemovedComponents<'a, T> {
    type Item = Entity;
    type IntoIter = std::iter::Cloned<std::slice::Iter<'a, Entity>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// SAFETY: Only reads World components
unsafe impl<'a, T: Component> ReadOnlySystemParam for RemovedComponents<'a, T> {}

/// The [`SystemParamState`] of [`RemovedComponents<T>`].
#[doc(hidden)]
pub struct RemovedComponentsState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<'a, T: Component> SystemParam for RemovedComponents<'a, T> {
    type State = RemovedComponentsState<T>;
}

// SAFETY: no component access. removed component entity collections can be read in parallel and are
// never mutably borrowed during system execution
unsafe impl<T: Component> SystemParamState for RemovedComponentsState<T> {
    type Item<'w, 's> = RemovedComponents<'w, T>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        Self {
            component_id: world.init_component::<T>(),
            marker: PhantomData,
        }
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item<'w, 's> {
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

// SAFETY: Only reads a single World non-send resource
unsafe impl<'w, T> ReadOnlySystemParam for NonSend<'w, T> {}

impl<'w, T> Debug for NonSend<'w, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NonSend").field(&self.value).finish()
    }
}

impl<'w, T: 'static> NonSend<'w, T> {
    /// Returns `true` if the resource was added after the system last ran.
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_change_tick, self.change_tick)
    }

    /// Returns `true` if the resource was added or mutably dereferenced after the system last ran.
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
impl<'a, T> From<NonSendMut<'a, T>> for NonSend<'a, T> {
    fn from(nsm: NonSendMut<'a, T>) -> Self {
        Self {
            value: nsm.value,
            ticks: ComponentTicks {
                added: nsm.ticks.added.to_owned(),
                changed: nsm.ticks.changed.to_owned(),
            },
            change_tick: nsm.ticks.change_tick,
            last_change_tick: nsm.ticks.last_change_tick,
        }
    }
}

/// The [`SystemParamState`] of [`NonSend<T>`].
#[doc(hidden)]
pub struct NonSendState<T> {
    component_id: ComponentId,
    marker: PhantomData<fn() -> T>,
}

impl<'a, T: 'static> SystemParam for NonSend<'a, T> {
    type State = NonSendState<T>;
}

// SAFETY: NonSendComponentId and ArchetypeComponentId access is applied to SystemMeta. If this
// NonSend conflicts with any prior access, a panic will occur.
unsafe impl<T: 'static> SystemParamState for NonSendState<T> {
    type Item<'w, 's> = NonSend<'w, T>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        system_meta.set_non_send();

        let component_id = world.initialize_non_send_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access();
        assert!(
            !combined_access.has_write(component_id),
            "error[B0002]: NonSend<{}> in system {} conflicts with a previous mutable resource access ({0}). Consider removing the duplicate access.",
            std::any::type_name::<T>(),
            system_meta.name,
        );
        system_meta
            .component_access_set
            .add_unfiltered_read(component_id);

        let archetype_component_id = world
            .get_resource_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_read(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world.validate_non_send_access::<T>();
        let (ptr, ticks) = world
            .get_resource_with_ticks(state.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Non-send resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });

        NonSend {
            value: ptr.deref(),
            ticks: ticks.read(),
            last_change_tick: system_meta.last_change_tick,
            change_tick,
        }
    }
}

/// The [`SystemParamState`] of [`Option<NonSend<T>>`].
/// See: [`NonSend<T>`]
#[doc(hidden)]
pub struct OptionNonSendState<T>(NonSendState<T>);

impl<'w, T: 'static> SystemParam for Option<NonSend<'w, T>> {
    type State = OptionNonSendState<T>;
}

// SAFETY: Only reads a single non-send resource
unsafe impl<'w, T: 'static> ReadOnlySystemParam for Option<NonSend<'w, T>> {}

// SAFETY: this impl defers to `NonSendState`, which initializes
// and validates the correct world access
unsafe impl<T: 'static> SystemParamState for OptionNonSendState<T> {
    type Item<'w, 's> = Option<NonSend<'w, T>>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        Self(NonSendState::init(world, system_meta))
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world.validate_non_send_access::<T>();
        world
            .get_resource_with_ticks(state.0.component_id)
            .map(|(ptr, ticks)| NonSend {
                value: ptr.deref(),
                ticks: ticks.read(),
                last_change_tick: system_meta.last_change_tick,
                change_tick,
            })
    }
}

/// The [`SystemParamState`] of [`NonSendMut<T>`].
#[doc(hidden)]
pub struct NonSendMutState<T> {
    component_id: ComponentId,
    marker: PhantomData<fn() -> T>,
}

impl<'a, T: 'static> SystemParam for NonSendMut<'a, T> {
    type State = NonSendMutState<T>;
}

// SAFETY: NonSendMut ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this
// NonSendMut conflicts with any prior access, a panic will occur.
unsafe impl<T: 'static> SystemParamState for NonSendMutState<T> {
    type Item<'w, 's> = NonSendMut<'w, T>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        system_meta.set_non_send();

        let component_id = world.initialize_non_send_resource::<T>();
        let combined_access = system_meta.component_access_set.combined_access();
        if combined_access.has_write(component_id) {
            panic!(
                "error[B0002]: NonSendMut<{}> in system {} conflicts with a previous mutable resource access ({0}). Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        } else if combined_access.has_read(component_id) {
            panic!(
                "error[B0002]: NonSendMut<{}> in system {} conflicts with a previous immutable resource access ({0}). Consider removing the duplicate access.",
                std::any::type_name::<T>(), system_meta.name);
        }
        system_meta
            .component_access_set
            .add_unfiltered_write(component_id);

        let archetype_component_id = world
            .get_resource_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_write(archetype_component_id);
        Self {
            component_id,
            marker: PhantomData,
        }
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world.validate_non_send_access::<T>();
        let (ptr, ticks) = world
            .get_resource_with_ticks(state.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Non-send resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        NonSendMut {
            value: ptr.assert_unique().deref_mut(),
            ticks: Ticks::from_tick_cells(ticks, system_meta.last_change_tick, change_tick),
        }
    }
}

/// The [`SystemParamState`] of [`Option<NonSendMut<T>>`].
/// See: [`NonSendMut<T>`]
#[doc(hidden)]
pub struct OptionNonSendMutState<T>(NonSendMutState<T>);

impl<'a, T: 'static> SystemParam for Option<NonSendMut<'a, T>> {
    type State = OptionNonSendMutState<T>;
}

// SAFETY: this impl defers to `NonSendMutState`, which initializes
// and validates the correct world access
unsafe impl<T: 'static> SystemParamState for OptionNonSendMutState<T> {
    type Item<'w, 's> = Option<NonSendMut<'w, T>>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        Self(NonSendMutState::init(world, system_meta))
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world.validate_non_send_access::<T>();
        world
            .get_resource_with_ticks(state.0.component_id)
            .map(|(ptr, ticks)| NonSendMut {
                value: ptr.assert_unique().deref_mut(),
                ticks: Ticks::from_tick_cells(ticks, system_meta.last_change_tick, change_tick),
            })
    }
}

impl<'a> SystemParam for &'a Archetypes {
    type State = ArchetypesState;
}

// SAFETY: Only reads World archetypes
unsafe impl<'a> ReadOnlySystemParam for &'a Archetypes {}

/// The [`SystemParamState`] of [`Archetypes`].
#[doc(hidden)]
pub struct ArchetypesState;

// SAFETY: no component value access
unsafe impl SystemParamState for ArchetypesState {
    type Item<'w, 's> = &'w Archetypes;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        Self
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world.archetypes()
    }
}

impl<'a> SystemParam for &'a Components {
    type State = ComponentsState;
}

// SAFETY: Only reads World components
unsafe impl<'a> ReadOnlySystemParam for &'a Components {}

/// The [`SystemParamState`] of [`Components`].
#[doc(hidden)]
pub struct ComponentsState;

// SAFETY: no component value access
unsafe impl SystemParamState for ComponentsState {
    type Item<'w, 's> = &'w Components;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        Self
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world.components()
    }
}

impl<'a> SystemParam for &'a Entities {
    type State = EntitiesState;
}

// SAFETY: Only reads World entities
unsafe impl<'a> ReadOnlySystemParam for &'a Entities {}

/// The [`SystemParamState`] of [`Entities`].
#[doc(hidden)]
pub struct EntitiesState;

// SAFETY: no component value access
unsafe impl SystemParamState for EntitiesState {
    type Item<'w, 's> = &'w Entities;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        Self
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world.entities()
    }
}

impl<'a> SystemParam for &'a Bundles {
    type State = BundlesState;
}

// SAFETY: Only reads World bundles
unsafe impl<'a> ReadOnlySystemParam for &'a Bundles {}

/// The [`SystemParamState`] of [`Bundles`].
#[doc(hidden)]
pub struct BundlesState;

// SAFETY: no component value access
unsafe impl SystemParamState for BundlesState {
    type Item<'w, 's> = &'w Bundles;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        Self
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self,
        _system_meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item<'w, 's> {
        world.bundles()
    }
}

/// A [`SystemParam`] that reads the previous and current change ticks of the system.
///
/// A system's change ticks are updated each time it runs:
/// - `last_change_tick` copies the previous value of `change_tick`
/// - `change_tick` copies the current value of [`World::read_change_tick`]
///
/// Component change ticks that are more recent than `last_change_tick` will be detected by the system.
/// Those can be read by calling [`last_changed`](crate::change_detection::DetectChanges::last_changed)
/// on a [`Mut<T>`](crate::change_detection::Mut) or [`ResMut<T>`](crate::change_detection::ResMut).
#[derive(Debug)]
pub struct SystemChangeTick {
    last_change_tick: u32,
    change_tick: u32,
}

impl SystemChangeTick {
    /// Returns the current [`World`] change tick seen by the system.
    #[inline]
    pub fn change_tick(&self) -> u32 {
        self.change_tick
    }

    /// Returns the [`World`] change tick seen by the system the previous time it ran.
    #[inline]
    pub fn last_change_tick(&self) -> u32 {
        self.last_change_tick
    }
}

// SAFETY: Only reads internal system state
unsafe impl ReadOnlySystemParam for SystemChangeTick {}

impl SystemParam for SystemChangeTick {
    type State = SystemChangeTickState;
}

/// The [`SystemParamState`] of [`SystemChangeTick`].
#[doc(hidden)]
pub struct SystemChangeTickState {}

// SAFETY: `SystemParamTickState` doesn't require any world access
unsafe impl SystemParamState for SystemChangeTickState {
    type Item<'w, 's> = SystemChangeTick;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        Self {}
    }

    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self,
        system_meta: &SystemMeta,
        _world: &'w World,
        change_tick: u32,
    ) -> Self::Item<'w, 's> {
        SystemChangeTick {
            last_change_tick: system_meta.last_change_tick,
            change_tick,
        }
    }
}

/// Name of the system that corresponds to this [`crate::system::SystemState`].
///
/// This is not a reliable identifier, it is more so useful for debugging
/// purposes of finding where a system parameter is being used incorrectly.
pub struct SystemName<'s> {
    name: &'s str,
}

impl<'s> SystemName<'s> {
    pub fn name(&self) -> &str {
        self.name
    }
}

impl<'s> Deref for SystemName<'s> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.name()
    }
}

impl<'s> AsRef<str> for SystemName<'s> {
    fn as_ref(&self) -> &str {
        self.name()
    }
}

impl<'s> From<SystemName<'s>> for &'s str {
    fn from(name: SystemName<'s>) -> &'s str {
        name.name
    }
}

impl<'s> std::fmt::Debug for SystemName<'s> {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("SystemName").field(&self.name()).finish()
    }
}

impl<'s> std::fmt::Display for SystemName<'s> {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.name(), f)
    }
}

impl<'s> SystemParam for SystemName<'s> {
    type State = SystemNameState;
}

// SAFETY: Only reads internal system state
unsafe impl<'s> ReadOnlySystemParam for SystemName<'s> {}

/// The [`SystemParamState`] of [`SystemName`].
#[doc(hidden)]
pub struct SystemNameState {
    name: Cow<'static, str>,
}

// SAFETY: no component value access
unsafe impl SystemParamState for SystemNameState {
    type Item<'w, 's> = SystemName<'s>;

    fn init(_world: &mut World, system_meta: &mut SystemMeta) -> Self {
        Self {
            name: system_meta.name.clone(),
        }
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Item<'w, 's> {
        SystemName {
            name: state.name.as_ref(),
        }
    }
}

macro_rules! impl_system_param_tuple {
    ($($param: ident),*) => {
        impl<$($param: SystemParam),*> SystemParam for ($($param,)*) {
            type State = ($($param::State,)*);
        }

        // SAFETY: tuple consists only of ReadOnlySystemParams
        unsafe impl<$($param: ReadOnlySystemParam),*> ReadOnlySystemParam for ($($param,)*) {}


        // SAFETY: implementors of each `SystemParamState` in the tuple have validated their impls
        #[allow(clippy::undocumented_unsafe_blocks)] // false positive by clippy
        #[allow(non_snake_case)]
        unsafe impl<$($param: SystemParamState),*> SystemParamState for ($($param,)*) {
            type Item<'w, 's> = ($($param::Item::<'w, 's>,)*);

            #[inline]
            fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self {
                (($($param::init(_world, _system_meta),)*))
            }

            #[inline]
            fn new_archetype(&mut self, _archetype: &Archetype, _system_meta: &mut SystemMeta) {
                let ($($param,)*) = self;
                $($param.new_archetype(_archetype, _system_meta);)*
            }

            #[inline]
            fn apply(&mut self, _system_meta: &SystemMeta, _world: &mut World) {
                let ($($param,)*) = self;
                $($param.apply(_system_meta, _world);)*
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            unsafe fn get_param<'w, 's>(
                state: &'s mut Self,
                _system_meta: &SystemMeta,
                _world: &'w World,
                _change_tick: u32,
            ) -> Self::Item<'w, 's> {

                let ($($param,)*) = state;
                ($($param::get_param($param, _system_meta, _world, _change_tick),)*)
            }
        }
    };
}

all_tuples!(impl_system_param_tuple, 0, 16, P);

pub mod lifetimeless {
    pub type SQuery<Q, F = ()> = super::Query<'static, 'static, Q, F>;
    pub type Read<T> = &'static T;
    pub type Write<T> = &'static mut T;
    pub type SRes<T> = super::Res<'static, T>;
    pub type SResMut<T> = super::ResMut<'static, T>;
    pub type SCommands = crate::system::Commands<'static, 'static>;
}

/// A helper for using system parameters in generic contexts
///
/// This type is a [`SystemParam`] adapter which always has
/// `Self::State::Item == Self` (ignoring lifetimes for brevity),
/// no matter the argument [`SystemParam`] (`P`) (other than
/// that `P` must be `'static`)
///
/// This makes it useful for having arbitrary [`SystemParam`] type arguments
/// to function systems, or for generic types using the [`derive@SystemParam`]
/// derive:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::system::{SystemParam, StaticSystemParam};
/// #[derive(SystemParam)]
/// struct GenericParam<'w,'s, T: SystemParam + 'static> {
///     field: StaticSystemParam<'w, 's, T>,
/// }
/// fn do_thing_generically<T: SystemParam + 'static>(t: StaticSystemParam<T>) {}
///
/// fn check_always_is_system<T: SystemParam + 'static>(){
///     bevy_ecs::system::assert_is_system(do_thing_generically::<T>);
/// }
/// ```
/// Note that in a real case you'd generally want
/// additional bounds on `P`, for your use of the parameter
/// to have a reason to be generic.
///
/// For example, using this would allow a type to be generic over
/// whether a resource is accessed mutably or not, with
/// impls being bounded on [`P: Deref<Target=MyType>`](Deref), and
/// [`P: DerefMut<Target=MyType>`](DerefMut) depending on whether the
/// method requires mutable access or not.
///
/// The method which doesn't use this type will not compile:
/// ```compile_fail
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::{SystemParam, StaticSystemParam};
///
/// fn do_thing_generically<T: SystemParam + 'static>(t: T) {}
///
/// #[derive(SystemParam)]
/// struct GenericParam<'w, 's, T: SystemParam> {
///     field: T,
///     #[system_param(ignore)]
///     // Use the lifetimes in this type, or they will be unbound.
///     phantom: core::marker::PhantomData<&'w &'s ()>
/// }
/// # fn check_always_is_system<T: SystemParam + 'static>(){
/// #    bevy_ecs::system::assert_is_system(do_thing_generically::<T>);
/// # }
/// ```
///
pub struct StaticSystemParam<'w, 's, P: SystemParam>(SystemParamItem<'w, 's, P>);

impl<'w, 's, P: SystemParam> Deref for StaticSystemParam<'w, 's, P> {
    type Target = SystemParamItem<'w, 's, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'w, 's, P: SystemParam> DerefMut for StaticSystemParam<'w, 's, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'w, 's, P: SystemParam> StaticSystemParam<'w, 's, P> {
    /// Get the value of the parameter
    pub fn into_inner(self) -> SystemParamItem<'w, 's, P> {
        self.0
    }
}

/// The [`SystemParamState`] of [`StaticSystemParam`].
#[doc(hidden)]
pub struct StaticSystemParamState<S, P>(S, PhantomData<fn() -> P>);

// SAFETY: This doesn't add any more reads, and the delegated fetch confirms it
unsafe impl<'w, 's, P: ReadOnlySystemParam + 'static> ReadOnlySystemParam
    for StaticSystemParam<'w, 's, P>
{
}

impl<'world, 'state, P: SystemParam + 'static> SystemParam
    for StaticSystemParam<'world, 'state, P>
{
    type State = StaticSystemParamState<P::State, P>;
}

// SAFETY: all methods are just delegated to `S`'s `SystemParamState` implementation
unsafe impl<S: SystemParamState, P: SystemParam<State = S> + 'static> SystemParamState
    for StaticSystemParamState<S, P>
{
    type Item<'world, 'state> = StaticSystemParam<'world, 'state, P>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        Self(S::init(world, system_meta), PhantomData)
    }

    fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
        self.0.new_archetype(archetype, system_meta);
    }

    fn apply(&mut self, system_meta: &SystemMeta, world: &mut World) {
        self.0.apply(system_meta, world);
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self,
        system_meta: &SystemMeta,
        world: &'world World,
        change_tick: u32,
    ) -> Self::Item<'world, 'state> {
        // SAFETY: We properly delegate SystemParamState
        StaticSystemParam(S::get_param(&mut state.0, system_meta, world, change_tick))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        self as bevy_ecs, // Necessary for the `SystemParam` Derive when used inside `bevy_ecs`.
        query::{ReadOnlyWorldQuery, WorldQuery},
        system::Query,
    };

    // Compile test for #2838
    #[derive(SystemParam)]
    pub struct SpecialQuery<
        'w,
        's,
        Q: WorldQuery + Send + Sync + 'static,
        F: ReadOnlyWorldQuery + Send + Sync + 'static = (),
    > {
        _query: Query<'w, 's, Q, F>,
    }

    #[derive(SystemParam)]
    pub struct SpecialRes<'w, T: Resource> {
        _res: Res<'w, T>,
    }

    #[derive(SystemParam)]
    pub struct SpecialLocal<'s, T: FromWorld + Send + 'static> {
        _local: Local<'s, T>,
    }

    #[derive(SystemParam)]
    pub struct UnitParam {}
}
