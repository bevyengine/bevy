pub use crate::change_detection::{NonSendMut, Res, ResMut};
use crate::{
    archetype::{Archetype, Archetypes},
    bundle::Bundles,
    change_detection::{Ticks, TicksMut},
    component::{ComponentId, ComponentTicks, Components, Tick},
    entity::Entities,
    prelude::QueryBuilder,
    query::{
        Access, FilteredAccess, FilteredAccessSet, QueryData, QueryFilter, QueryState,
        ReadOnlyQueryData,
    },
    system::{Query, SystemMeta},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, FromWorld, World},
};
use bevy_ecs_macros::impl_param_set;
pub use bevy_ecs_macros::Resource;
pub use bevy_ecs_macros::SystemParam;
use bevy_ptr::UnsafeCellDeref;
use bevy_utils::{all_tuples, synccell::SyncCell};
use std::{
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
/// The following list shows the most common [`SystemParam`]s and which lifetime they require
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Resource)]
/// # struct SomeResource;
/// # #[derive(Event)]
/// # struct SomeEvent;
/// # #[derive(Resource)]
/// # struct SomeOtherResource;
/// # use bevy_ecs::system::SystemParam;
/// # #[derive(SystemParam)]
/// # struct ParamsExample<'w, 's> {
/// #    query:
/// Query<'w, 's, Entity>,
/// #    res:
/// Res<'w, SomeResource>,
/// #    res_mut:
/// ResMut<'w, SomeOtherResource>,
/// #    local:
/// Local<'s, u8>,
/// #    commands:
/// Commands<'w, 's>,
/// #    eventreader:
/// EventReader<'w, 's, SomeEvent>,
/// #    eventwriter:
/// EventWriter<'w, SomeEvent>
/// # }
///```
/// ## `PhantomData`
///
/// [`PhantomData`] is a special type of `SystemParam` that does nothing.
/// This is useful for constraining generic types or lifetimes.
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
/// found associated type `<[ParamType] as SystemParam>::Item<'_, '_>`
/// ```
/// where `[ParamType]` is the type of one of your fields.
/// To solve this error, you can wrap the field of type `[ParamType]` with [`StaticSystemParam`]
/// (i.e. `StaticSystemParam<[ParamType]>`).
///
/// ## Details
///
/// The derive macro requires that the [`SystemParam`] implementation of
/// each field `F`'s [`Item`](`SystemParam::Item`)'s is itself `F`
/// (ignoring lifetimes for simplicity).
/// This assumption is due to type inference reasons, so that the derived [`SystemParam`] can be
/// used as an argument to a function system.
/// If the compiler cannot validate this property for `[ParamType]`, it will error in the form shown above.
///
/// This will most commonly occur when working with `SystemParam`s generically, as the requirement
/// has not been proven to the compiler.
///
/// # Safety
///
/// The implementor must ensure the following is true.
/// - [`SystemParam::init_state`] correctly registers all [`World`] accesses used
///   by [`SystemParam::get_param`] with the provided [`system_meta`](SystemMeta).
/// - None of the world accesses may conflict with any prior accesses registered
///   on `system_meta`.
pub unsafe trait SystemParam: Sized {
    /// Used to store data which persists across invocations of a system.
    type State: Send + Sync + 'static;

    /// The item type returned when constructing this system param.
    /// The value of this associated type should be `Self`, instantiated with new lifetimes.
    ///
    /// You could think of `SystemParam::Item<'w, 's>` as being an *operation* that changes the lifetimes bound to `Self`.
    type Item<'world, 'state>: SystemParam<State = Self::State>;

    /// Registers any [`World`] access used by this [`SystemParam`]
    /// and creates a new instance of this param's [`State`](Self::State).
    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State;

    /// For the specified [`Archetype`], registers the components accessed by this [`SystemParam`] (if applicable).a
    ///
    /// # Safety
    /// `archetype` must be from the [`World`] used to initialize `state` in `init_state`.
    #[inline]
    #[allow(unused_variables)]
    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
    }

    /// Applies any deferred mutations stored in this [`SystemParam`]'s state.
    /// This is used to apply [`Commands`] during [`apply_deferred`](crate::prelude::apply_deferred).
    ///
    /// [`Commands`]: crate::prelude::Commands
    #[inline]
    #[allow(unused_variables)]
    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {}

    /// Queues any deferred mutations to be applied at the next [`apply_deferred`](crate::prelude::apply_deferred).
    #[inline]
    #[allow(unused_variables)]
    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {}

    /// Creates a parameter to be passed into a [`SystemParamFunction`].
    ///
    /// [`SystemParamFunction`]: super::SystemParamFunction
    ///
    /// # Safety
    ///
    /// - The passed [`UnsafeWorldCell`] must have access to any world data
    ///   registered in [`init_state`](SystemParam::init_state).
    /// - `world` must be the same `World` that was used to initialize [`state`](SystemParam::init_state).
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state>;
}

/// A parameter that can be built with [`SystemBuilder`](crate::system::builder::SystemBuilder)
pub trait BuildableSystemParam: SystemParam {
    /// A mutable reference to this type will be passed to the builder function
    type Builder<'b>;

    /// Constructs [`SystemParam::State`] for `Self` using a given builder function
    fn build(
        world: &mut World,
        meta: &mut SystemMeta,
        func: impl FnOnce(&mut Self::Builder<'_>),
    ) -> Self::State;
}

/// A [`SystemParam`] that only reads a given [`World`].
///
/// # Safety
/// This must only be implemented for [`SystemParam`] impls that exclusively read the World passed in to [`SystemParam::get_param`]
pub unsafe trait ReadOnlySystemParam: SystemParam {}

/// Shorthand way of accessing the associated type [`SystemParam::Item`] for a given [`SystemParam`].
pub type SystemParamItem<'w, 's, P> = <P as SystemParam>::Item<'w, 's>;

// SAFETY: QueryState is constrained to read-only fetches, so it only reads World.
unsafe impl<'w, 's, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> ReadOnlySystemParam
    for Query<'w, 's, D, F>
{
}

// SAFETY: Relevant query ComponentId and ArchetypeComponentId access is applied to SystemMeta. If
// this Query conflicts with any prior access, a panic will occur.
unsafe impl<D: QueryData + 'static, F: QueryFilter + 'static> SystemParam for Query<'_, '_, D, F> {
    type State = QueryState<D, F>;
    type Item<'w, 's> = Query<'w, 's, D, F>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let state = QueryState::new_with_access(world, &mut system_meta.archetype_component_access);
        assert_component_access_compatibility(
            &system_meta.name,
            std::any::type_name::<D>(),
            std::any::type_name::<F>(),
            &system_meta.component_access_set,
            &state.component_access,
            world,
        );
        system_meta
            .component_access_set
            .add(state.component_access.clone());
        state
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        state.new_archetype(archetype, &mut system_meta.archetype_component_access);
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        // SAFETY: We have registered all of the query's world accesses,
        // so the caller ensures that `world` has permission to access any
        // world data that the query needs.
        unsafe { Query::new(world, state, system_meta.last_run, change_tick) }
    }
}

impl<'w, 's, D: QueryData + 'static, F: QueryFilter + 'static> BuildableSystemParam
    for Query<'w, 's, D, F>
{
    type Builder<'b> = QueryBuilder<'b, D, F>;

    #[inline]
    fn build(
        world: &mut World,
        system_meta: &mut SystemMeta,
        build: impl FnOnce(&mut Self::Builder<'_>),
    ) -> Self::State {
        let mut builder = QueryBuilder::new(world);
        build(&mut builder);
        let state = builder.build();
        assert_component_access_compatibility(
            &system_meta.name,
            std::any::type_name::<D>(),
            std::any::type_name::<F>(),
            &system_meta.component_access_set,
            &state.component_access,
            world,
        );
        system_meta
            .component_access_set
            .add(state.component_access.clone());
        state
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
    panic!("error[B0001]: Query<{query_type}, {filter_type}> in system {system_name} accesses component(s) {accesses} in a way that conflicts with a previous system parameter. Consider using `Without<T>` to create disjoint Queries or merging conflicting Queries into a `ParamSet`. See: https://bevyengine.org/learn/errors/#b0001");
}

/// A collection of potentially conflicting [`SystemParam`]s allowed by disjoint access.
///
/// Allows systems to safely access and interact with up to 8 mutually exclusive [`SystemParam`]s, such as
/// two queries that reference the same mutable data or an event reader and writer of the same type.
///
/// Each individual [`SystemParam`] can be accessed by using the functions `p0()`, `p1()`, ..., `p7()`,
/// according to the order they are defined in the `ParamSet`. This ensures that there's either
/// only one mutable reference to a parameter at a time or any number of immutable references.
///
/// # Examples
///
/// The following system mutably accesses the same component two times,
/// which is not allowed due to rust's mutability rules.
///
/// ```should_panic
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct Health;
/// #
/// # #[derive(Component)]
/// # struct Enemy;
/// #
/// # #[derive(Component)]
/// # struct Ally;
/// #
/// // This will panic at runtime when the system gets initialized.
/// fn bad_system(
///     mut enemies: Query<&mut Health, With<Enemy>>,
///     mut allies: Query<&mut Health, With<Ally>>,
/// ) {
///     // ...
/// }
/// #
/// # let mut bad_system_system = IntoSystem::into_system(bad_system);
/// # let mut world = World::new();
/// # bad_system_system.initialize(&mut world);
/// # bad_system_system.run((), &mut world);
/// ```
///
/// Conflicting `SystemParam`s like these can be placed in a `ParamSet`,
/// which leverages the borrow checker to ensure that only one of the contained parameters are accessed at a given time.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct Health;
/// #
/// # #[derive(Component)]
/// # struct Enemy;
/// #
/// # #[derive(Component)]
/// # struct Ally;
/// #
/// // Given the following system
/// fn fancy_system(
///     mut set: ParamSet<(
///         Query<&mut Health, With<Enemy>>,
///         Query<&mut Health, With<Ally>>,
///     )>
/// ) {
///     // This will access the first `SystemParam`.
///     for mut health in set.p0().iter_mut() {
///         // Do your fancy stuff here...
///     }
///
///     // The second `SystemParam`.
///     // This would fail to compile if the previous parameter was still borrowed.
///     for mut health in set.p1().iter_mut() {
///         // Do even fancier stuff here...
///     }
/// }
/// # bevy_ecs::system::assert_is_system(fancy_system);
/// ```
///
/// Of course, `ParamSet`s can be used with any kind of `SystemParam`, not just [queries](Query).
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Event)]
/// # struct MyEvent;
/// # impl MyEvent {
/// #   pub fn new() -> Self { Self }
/// # }
/// fn event_system(
///     mut set: ParamSet<(
///         // `EventReader`s and `EventWriter`s conflict with each other,
///         // since they both access the event queue resource for `MyEvent`.
///         EventReader<MyEvent>,
///         EventWriter<MyEvent>,
///         // `&World` reads the entire world, so a `ParamSet` is the only way
///         // that it can be used in the same system as any mutable accesses.
///         &World,
///     )>,
/// ) {
///     for event in set.p0().read() {
///         // ...
///         # let _event = event;
///     }
///     set.p1().send(MyEvent::new());
///     
///     let entities = set.p2().entities();
///     // ...
///     # let _entities = entities;
/// }
/// # bevy_ecs::system::assert_is_system(event_system);
/// ```
pub struct ParamSet<'w, 's, T: SystemParam> {
    param_states: &'s mut T::State,
    world: UnsafeWorldCell<'w>,
    system_meta: SystemMeta,
    change_tick: Tick,
}

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
/// # schedule.add_systems((read_resource_system, write_resource_system).chain());
/// # schedule.run(&mut world);
/// ```
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
/// [`Exclusive`]: https://doc.rust-lang.org/nightly/std/sync/struct.Exclusive.html
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Resource`",
    label = "invalid `Resource`",
    note = "consider annotating `{Self}` with `#[derive(Resource)]`"
)]
pub trait Resource: Send + Sync + 'static {}

// SAFETY: Res only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for Res<'a, T> {}

// SAFETY: Res ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: Resource> SystemParam for Res<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = Res<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let component_id = world.components.init_resource::<T>();
        world.initialize_resource_internal(component_id);

        let combined_access = system_meta.component_access_set.combined_access();
        assert!(
            !combined_access.has_write(component_id),
            "error[B0002]: Res<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/#b0002",
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

        component_id
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks) = world
            .get_resource_with_ticks(component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        Res {
            value: ptr.deref(),
            ticks: Ticks {
                added: ticks.added.deref(),
                changed: ticks.changed.deref(),
                last_run: system_meta.last_run,
                this_run: change_tick,
            },
        }
    }
}

// SAFETY: Only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for Option<Res<'a, T>> {}

// SAFETY: this impl defers to `Res`, which initializes and validates the correct world access.
unsafe impl<'a, T: Resource> SystemParam for Option<Res<'a, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<Res<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        Res::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_resource_with_ticks(component_id)
            .map(|(ptr, ticks)| Res {
                value: ptr.deref(),
                ticks: Ticks {
                    added: ticks.added.deref(),
                    changed: ticks.changed.deref(),
                    last_run: system_meta.last_run,
                    this_run: change_tick,
                },
            })
    }
}

// SAFETY: Res ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: Resource> SystemParam for ResMut<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = ResMut<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let component_id = world.components.init_resource::<T>();
        world.initialize_resource_internal(component_id);

        let combined_access = system_meta.component_access_set.combined_access();
        if combined_access.has_write(component_id) {
            panic!(
                "error[B0002]: ResMut<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/#b0002",
                std::any::type_name::<T>(), system_meta.name);
        } else if combined_access.has_read(component_id) {
            panic!(
                "error[B0002]: ResMut<{}> in system {} conflicts with a previous Res<{0}> access. Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/#b0002",
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

        component_id
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let value = world
            .get_resource_mut_by_id(component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        ResMut {
            value: value.value.deref_mut::<T>(),
            ticks: TicksMut {
                added: value.ticks.added,
                changed: value.ticks.changed,
                last_run: system_meta.last_run,
                this_run: change_tick,
            },
        }
    }
}

// SAFETY: this impl defers to `ResMut`, which initializes and validates the correct world access.
unsafe impl<'a, T: Resource> SystemParam for Option<ResMut<'a, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<ResMut<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        ResMut::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_resource_mut_by_id(component_id)
            .map(|value| ResMut {
                value: value.value.deref_mut::<T>(),
                ticks: TicksMut {
                    added: value.ticks.added,
                    changed: value.ticks.changed,
                    last_run: system_meta.last_run,
                    this_run: change_tick,
                },
            })
    }
}

/// SAFETY: only reads world
unsafe impl<'w> ReadOnlySystemParam for &'w World {}

// SAFETY: `read_all` access is set and conflicts result in a panic
unsafe impl SystemParam for &'_ World {
    type State = ();
    type Item<'w, 's> = &'w World;

    fn init_state(_world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
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
    }

    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        // SAFETY: Read-only access to the entire world was registered in `init_state`.
        unsafe { world.world() }
    }
}

/// SAFETY: `DeferredWorld` can read all components and resources but cannot be used to gain any other mutable references.
unsafe impl<'w> SystemParam for DeferredWorld<'w> {
    type State = ();
    type Item<'world, 'state> = DeferredWorld<'world>;

    fn init_state(_world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.component_access_set.read_all();
        system_meta.component_access_set.write_all();
        system_meta.set_has_deferred();
    }

    unsafe fn get_param<'world, 'state>(
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        world.into_deferred()
    }
}

/// A system local [`SystemParam`].
///
/// A local may only be accessed by the system itself and is therefore not visible to other systems.
/// If two or more systems specify the same local type each will have their own unique local.
/// If multiple [`SystemParam`]s within the same system each specify the same local type
/// each will get their own distinct data storage.
///
/// The supplied lifetime parameter is the [`SystemParam`]s `'s` lifetime.
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
/// struct MyU32Wrapper(u32);
/// fn reset_to_system(value: Config) -> impl FnMut(ResMut<MyU32Wrapper>) {
///     move |mut val| val.0 = value.0
/// }
///
/// // .add_systems(reset_to_system(my_config))
/// # assert_is_system(reset_to_system(Config(10)));
/// ```
#[derive(Debug)]
pub struct Local<'s, T: FromWorld + Send + 'static>(pub(crate) &'s mut T);

// SAFETY: Local only accesses internal state
unsafe impl<'s, T: FromWorld + Send + 'static> ReadOnlySystemParam for Local<'s, T> {}

impl<'s, T: FromWorld + Send + 'static> Deref for Local<'s, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'s, T: FromWorld + Send + 'static> DerefMut for Local<'s, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'s, 'a, T: FromWorld + Send + 'static> IntoIterator for &'a Local<'s, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'s, 'a, T: FromWorld + Send + 'static> IntoIterator for &'a mut Local<'s, T>
where
    &'a mut T: IntoIterator,
{
    type Item = <&'a mut T as IntoIterator>::Item;
    type IntoIter = <&'a mut T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// SAFETY: only local state is accessed
unsafe impl<'a, T: FromWorld + Send + 'static> SystemParam for Local<'a, T> {
    type State = SyncCell<T>;
    type Item<'w, 's> = Local<'s, T>;

    fn init_state(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(T::from_world(world))
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        Local(state.get())
    }
}

impl<'w, T: FromWorld + Send + 'static> BuildableSystemParam for Local<'w, T> {
    type Builder<'b> = T;

    fn build(
        world: &mut World,
        _meta: &mut SystemMeta,
        func: impl FnOnce(&mut Self::Builder<'_>),
    ) -> Self::State {
        let mut value = T::from_world(world);
        func(&mut value);
        SyncCell::new(value)
    }
}

/// Types that can be used with [`Deferred<T>`] in systems.
/// This allows storing system-local data which is used to defer [`World`] mutations.
///
/// Types that implement `SystemBuffer` should take care to perform as many
/// computations up-front as possible. Buffers cannot be applied in parallel,
/// so you should try to minimize the time spent in [`SystemBuffer::apply`].
pub trait SystemBuffer: FromWorld + Send + 'static {
    /// Applies any deferred mutations to the [`World`].
    fn apply(&mut self, system_meta: &SystemMeta, world: &mut World);
    /// Queues any deferred mutations to be applied at the next [`apply_deferred`](crate::prelude::apply_deferred).
    fn queue(&mut self, _system_meta: &SystemMeta, _world: DeferredWorld) {}
}

/// A [`SystemParam`] that stores a buffer which gets applied to the [`World`] during
/// [`apply_deferred`](crate::schedule::apply_deferred).
/// This is used internally by [`Commands`] to defer `World` mutations.
///
/// [`Commands`]: crate::system::Commands
///
/// # Examples
///
/// By using this type to defer mutations, you can avoid mutable `World` access within
/// a system, which allows it to run in parallel with more systems.
///
/// Note that deferring mutations is *not* free, and should only be used if
/// the gains in parallelization outweigh the time it takes to apply deferred mutations.
/// In general, [`Deferred`] should only be used for mutations that are infrequent,
/// or which otherwise take up a small portion of a system's run-time.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// // Tracks whether or not there is a threat the player should be aware of.
/// #[derive(Resource, Default)]
/// pub struct Alarm(bool);
///
/// #[derive(Component)]
/// pub struct Settlement {
///     // ...
/// }
///
/// // A threat from inside the settlement.
/// #[derive(Component)]
/// pub struct Criminal;
///
/// // A threat from outside the settlement.
/// #[derive(Component)]
/// pub struct Monster;
///
/// # impl Criminal { pub fn is_threat(&self, _: &Settlement) -> bool { true } }
///
/// use bevy_ecs::system::{Deferred, SystemBuffer, SystemMeta};
///
/// // Uses deferred mutations to allow signalling the alarm from multiple systems in parallel.
/// #[derive(Resource, Default)]
/// struct AlarmFlag(bool);
///
/// impl AlarmFlag {
///     /// Sounds the alarm the next time buffers are applied via apply_deferred.
///     pub fn flag(&mut self) {
///         self.0 = true;
///     }
/// }
///
/// impl SystemBuffer for AlarmFlag {
///     // When `AlarmFlag` is used in a system, this function will get
///     // called the next time buffers are applied via apply_deferred.
///     fn apply(&mut self, system_meta: &SystemMeta, world: &mut World) {
///         if self.0 {
///             world.resource_mut::<Alarm>().0 = true;
///             self.0 = false;
///         }
///     }
/// }
///
/// // Sound the alarm if there are any criminals who pose a threat.
/// fn alert_criminal(
///     settlements: Query<&Settlement>,
///     criminals: Query<&Criminal>,
///     mut alarm: Deferred<AlarmFlag>
/// ) {
///     let settlement = settlements.single();
///     for criminal in &criminals {
///         // Only sound the alarm if the criminal is a threat.
///         // For this example, assume that this check is expensive to run.
///         // Since the majority of this system's run-time is dominated
///         // by calling `is_threat()`, we defer sounding the alarm to
///         // allow this system to run in parallel with other alarm systems.
///         if criminal.is_threat(settlement) {
///             alarm.flag();
///         }
///     }
/// }
///
/// // Sound the alarm if there is a monster.
/// fn alert_monster(
///     monsters: Query<&Monster>,
///     mut alarm: ResMut<Alarm>
/// ) {
///     if monsters.iter().next().is_some() {
///         // Since this system does nothing except for sounding the alarm,
///         // it would be pointless to defer it, so we sound the alarm directly.
///         alarm.0 = true;
///     }
/// }
///
/// let mut world = World::new();
/// world.init_resource::<Alarm>();
/// world.spawn(Settlement {
///     // ...
/// });
///
/// let mut schedule = Schedule::default();
/// // These two systems have no conflicts and will run in parallel.
/// schedule.add_systems((alert_criminal, alert_monster));
///
/// // There are no criminals or monsters, so the alarm is not sounded.
/// schedule.run(&mut world);
/// assert_eq!(world.resource::<Alarm>().0, false);
///
/// // Spawn a monster, which will cause the alarm to be sounded.
/// let m_id = world.spawn(Monster).id();
/// schedule.run(&mut world);
/// assert_eq!(world.resource::<Alarm>().0, true);
///
/// // Remove the monster and reset the alarm.
/// world.entity_mut(m_id).despawn();
/// world.resource_mut::<Alarm>().0 = false;
///
/// // Spawn a criminal, which will cause the alarm to be sounded.
/// world.spawn(Criminal);
/// schedule.run(&mut world);
/// assert_eq!(world.resource::<Alarm>().0, true);
/// ```
pub struct Deferred<'a, T: SystemBuffer>(pub(crate) &'a mut T);

impl<'a, T: SystemBuffer> Deref for Deferred<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, T: SystemBuffer> DerefMut for Deferred<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<T: SystemBuffer> Deferred<'_, T> {
    /// Returns a [`Deferred<T>`] with a smaller lifetime.
    /// This is useful if you have `&mut Deferred<T>` but need `Deferred<T>`.
    pub fn reborrow(&mut self) -> Deferred<T> {
        Deferred(self.0)
    }
}

// SAFETY: Only local state is accessed.
unsafe impl<T: SystemBuffer> ReadOnlySystemParam for Deferred<'_, T> {}

// SAFETY: Only local state is accessed.
unsafe impl<T: SystemBuffer> SystemParam for Deferred<'_, T> {
    type State = SyncCell<T>;
    type Item<'w, 's> = Deferred<'s, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.set_has_deferred();
        SyncCell::new(T::from_world(world))
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        state.get().apply(system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {
        state.get().queue(system_meta, world);
    }

    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        Deferred(state.get())
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
    last_run: Tick,
    this_run: Tick,
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
        self.ticks.is_added(self.last_run, self.this_run)
    }

    /// Returns `true` if the resource was added or mutably dereferenced after the system last ran.
    pub fn is_changed(&self) -> bool {
        self.ticks.is_changed(self.last_run, self.this_run)
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
            this_run: nsm.ticks.this_run,
            last_run: nsm.ticks.last_run,
        }
    }
}

// SAFETY: NonSendComponentId and ArchetypeComponentId access is applied to SystemMeta. If this
// NonSend conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: 'static> SystemParam for NonSend<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = NonSend<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.set_non_send();

        let component_id = world.components.init_non_send::<T>();
        world.initialize_non_send_internal(component_id);

        let combined_access = system_meta.component_access_set.combined_access();
        assert!(
            !combined_access.has_write(component_id),
            "error[B0002]: NonSend<{}> in system {} conflicts with a previous mutable resource access ({0}). Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/#b0002",
            std::any::type_name::<T>(),
            system_meta.name,
        );
        system_meta
            .component_access_set
            .add_unfiltered_read(component_id);

        let archetype_component_id = world
            .get_non_send_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_read(archetype_component_id);

        component_id
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks) = world
            .get_non_send_with_ticks(component_id)
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
            last_run: system_meta.last_run,
            this_run: change_tick,
        }
    }
}

// SAFETY: Only reads a single World non-send resource
unsafe impl<T: 'static> ReadOnlySystemParam for Option<NonSend<'_, T>> {}

// SAFETY: this impl defers to `NonSend`, which initializes and validates the correct world access.
unsafe impl<T: 'static> SystemParam for Option<NonSend<'_, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<NonSend<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        NonSend::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_non_send_with_ticks(component_id)
            .map(|(ptr, ticks)| NonSend {
                value: ptr.deref(),
                ticks: ticks.read(),
                last_run: system_meta.last_run,
                this_run: change_tick,
            })
    }
}

// SAFETY: NonSendMut ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this
// NonSendMut conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: 'static> SystemParam for NonSendMut<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = NonSendMut<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.set_non_send();

        let component_id = world.components.init_non_send::<T>();
        world.initialize_non_send_internal(component_id);

        let combined_access = system_meta.component_access_set.combined_access();
        if combined_access.has_write(component_id) {
            panic!(
                "error[B0002]: NonSendMut<{}> in system {} conflicts with a previous mutable resource access ({0}). Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/#b0002",
                std::any::type_name::<T>(), system_meta.name);
        } else if combined_access.has_read(component_id) {
            panic!(
                "error[B0002]: NonSendMut<{}> in system {} conflicts with a previous immutable resource access ({0}). Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/#b0002",
                std::any::type_name::<T>(), system_meta.name);
        }
        system_meta
            .component_access_set
            .add_unfiltered_write(component_id);

        let archetype_component_id = world
            .get_non_send_archetype_component_id(component_id)
            .unwrap();
        system_meta
            .archetype_component_access
            .add_write(archetype_component_id);

        component_id
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks) = world
            .get_non_send_with_ticks(component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Non-send resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        NonSendMut {
            value: ptr.assert_unique().deref_mut(),
            ticks: TicksMut::from_tick_cells(ticks, system_meta.last_run, change_tick),
        }
    }
}

// SAFETY: this impl defers to `NonSendMut`, which initializes and validates the correct world access.
unsafe impl<'a, T: 'static> SystemParam for Option<NonSendMut<'a, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<NonSendMut<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        NonSendMut::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_non_send_with_ticks(component_id)
            .map(|(ptr, ticks)| NonSendMut {
                value: ptr.assert_unique().deref_mut(),
                ticks: TicksMut::from_tick_cells(ticks, system_meta.last_run, change_tick),
            })
    }
}

// SAFETY: Only reads World archetypes
unsafe impl<'a> ReadOnlySystemParam for &'a Archetypes {}

// SAFETY: no component value access
unsafe impl<'a> SystemParam for &'a Archetypes {
    type State = ();
    type Item<'w, 's> = &'w Archetypes;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world.archetypes()
    }
}

// SAFETY: Only reads World components
unsafe impl<'a> ReadOnlySystemParam for &'a Components {}

// SAFETY: no component value access
unsafe impl<'a> SystemParam for &'a Components {
    type State = ();
    type Item<'w, 's> = &'w Components;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world.components()
    }
}

// SAFETY: Only reads World entities
unsafe impl<'a> ReadOnlySystemParam for &'a Entities {}

// SAFETY: no component value access
unsafe impl<'a> SystemParam for &'a Entities {
    type State = ();
    type Item<'w, 's> = &'w Entities;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world.entities()
    }
}

// SAFETY: Only reads World bundles
unsafe impl<'a> ReadOnlySystemParam for &'a Bundles {}

// SAFETY: no component value access
unsafe impl<'a> SystemParam for &'a Bundles {
    type State = ();
    type Item<'w, 's> = &'w Bundles;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world.bundles()
    }
}

/// A [`SystemParam`] that reads the previous and current change ticks of the system.
///
/// A system's change ticks are updated each time it runs:
/// - `last_run` copies the previous value of `change_tick`
/// - `this_run` copies the current value of [`World::read_change_tick`]
///
/// Component change ticks that are more recent than `last_run` will be detected by the system.
/// Those can be read by calling [`last_changed`](crate::change_detection::DetectChanges::last_changed)
/// on a [`Mut<T>`](crate::change_detection::Mut) or [`ResMut<T>`](ResMut).
#[derive(Debug)]
pub struct SystemChangeTick {
    last_run: Tick,
    this_run: Tick,
}

impl SystemChangeTick {
    /// Returns the current [`World`] change tick seen by the system.
    #[inline]
    pub fn this_run(&self) -> Tick {
        self.this_run
    }

    /// Returns the [`World`] change tick seen by the system the previous time it ran.
    #[inline]
    pub fn last_run(&self) -> Tick {
        self.last_run
    }
}

// SAFETY: Only reads internal system state
unsafe impl ReadOnlySystemParam for SystemChangeTick {}

// SAFETY: `SystemChangeTick` doesn't require any world access
unsafe impl SystemParam for SystemChangeTick {
    type State = ();
    type Item<'w, 's> = SystemChangeTick;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        SystemChangeTick {
            last_run: system_meta.last_run,
            this_run: change_tick,
        }
    }
}

macro_rules! impl_system_param_tuple {
    ($($param: ident),*) => {
        // SAFETY: tuple consists only of ReadOnlySystemParams
        unsafe impl<$($param: ReadOnlySystemParam),*> ReadOnlySystemParam for ($($param,)*) {}

        // SAFETY: implementors of each `SystemParam` in the tuple have validated their impls
        #[allow(clippy::undocumented_unsafe_blocks)] // false positive by clippy
        #[allow(non_snake_case)]
        unsafe impl<$($param: SystemParam),*> SystemParam for ($($param,)*) {
            type State = ($($param::State,)*);
            type Item<'w, 's> = ($($param::Item::<'w, 's>,)*);

            #[inline]
            fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
                (($($param::init_state(_world, _system_meta),)*))
            }

            #[inline]
            #[allow(unused_unsafe)]
            unsafe fn new_archetype(($($param,)*): &mut Self::State, _archetype: &Archetype, _system_meta: &mut SystemMeta) {
                // SAFETY: The caller ensures that `archetype` is from the World the state was initialized from in `init_state`.
                unsafe { $($param::new_archetype($param, _archetype, _system_meta);)* }
            }

            #[inline]
            fn apply(($($param,)*): &mut Self::State, _system_meta: &SystemMeta, _world: &mut World) {
                $($param::apply($param, _system_meta, _world);)*
            }

            #[inline]
            fn queue(($($param,)*): &mut Self::State, _system_meta: &SystemMeta, mut _world: DeferredWorld) {
                $($param::queue($param, _system_meta, _world.reborrow());)*
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            unsafe fn get_param<'w, 's>(
                state: &'s mut Self::State,
                _system_meta: &SystemMeta,
                _world: UnsafeWorldCell<'w>,
                _change_tick: Tick,
            ) -> Self::Item<'w, 's> {

                let ($($param,)*) = state;
                ($($param::get_param($param, _system_meta, _world, _change_tick),)*)
            }
        }
    };
}

all_tuples!(impl_system_param_tuple, 0, 16, P);

/// Contains type aliases for built-in [`SystemParam`]s with `'static` lifetimes.
/// This makes it more convenient to refer to these types in contexts where
/// explicit lifetime annotations are required.
///
/// Note that this is entirely safe and tracks lifetimes correctly.
/// This purely exists for convenience.
///
/// You can't instantiate a static `SystemParam`, you'll always end up with
/// `Res<'w, T>`, `ResMut<'w, T>` or `&'w T` bound to the lifetime of the provided
/// `&'w World`.
///
/// [`SystemParam`]: super::SystemParam
pub mod lifetimeless {
    /// A [`Query`](super::Query) with `'static` lifetimes.
    pub type SQuery<D, F = ()> = super::Query<'static, 'static, D, F>;
    /// A shorthand for writing `&'static T`.
    pub type Read<T> = &'static T;
    /// A shorthand for writing `&'static mut T`.
    pub type Write<T> = &'static mut T;
    /// A [`Res`](super::Res) with `'static` lifetimes.
    pub type SRes<T> = super::Res<'static, T>;
    /// A [`ResMut`](super::ResMut) with `'static` lifetimes.
    pub type SResMut<T> = super::ResMut<'static, T>;
    /// [`Commands`](crate::system::Commands) with `'static` lifetimes.
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

// SAFETY: This doesn't add any more reads, and the delegated fetch confirms it
unsafe impl<'w, 's, P: ReadOnlySystemParam + 'static> ReadOnlySystemParam
    for StaticSystemParam<'w, 's, P>
{
}

// SAFETY: all methods are just delegated to `P`'s `SystemParam` implementation
unsafe impl<P: SystemParam + 'static> SystemParam for StaticSystemParam<'_, '_, P> {
    type State = P::State;
    type Item<'world, 'state> = StaticSystemParam<'world, 'state, P>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        P::init_state(world, system_meta)
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        // SAFETY: The caller guarantees that the provided `archetype` matches the World used to initialize `state`.
        unsafe { P::new_archetype(state, archetype, system_meta) };
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        P::apply(state, system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {
        P::queue(state, system_meta, world);
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY: Defer to the safety of P::SystemParam
        StaticSystemParam(unsafe { P::get_param(state, system_meta, world, change_tick) })
    }
}

// SAFETY: No world access.
unsafe impl<T: ?Sized> SystemParam for PhantomData<T> {
    type State = ();
    type Item<'world, 'state> = Self;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    unsafe fn get_param<'world, 'state>(
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        PhantomData
    }
}

// SAFETY: No world access.
unsafe impl<T: ?Sized> ReadOnlySystemParam for PhantomData<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        self as bevy_ecs, // Necessary for the `SystemParam` Derive when used inside `bevy_ecs`.
        system::assert_is_system,
    };
    use std::cell::RefCell;

    // Compile test for https://github.com/bevyengine/bevy/pull/2838.
    #[test]
    fn system_param_generic_bounds() {
        #[derive(SystemParam)]
        pub struct SpecialQuery<
            'w,
            's,
            D: QueryData + Send + Sync + 'static,
            F: QueryFilter + Send + Sync + 'static = (),
        > {
            _query: Query<'w, 's, D, F>,
        }

        fn my_system(_: SpecialQuery<(), ()>) {}
        assert_is_system(my_system);
    }

    // Compile tests for https://github.com/bevyengine/bevy/pull/6694.
    #[test]
    fn system_param_flexibility() {
        #[derive(SystemParam)]
        pub struct SpecialRes<'w, T: Resource> {
            _res: Res<'w, T>,
        }

        #[derive(SystemParam)]
        pub struct SpecialLocal<'s, T: FromWorld + Send + 'static> {
            _local: Local<'s, T>,
        }

        #[derive(Resource)]
        struct R;

        fn my_system(_: SpecialRes<R>, _: SpecialLocal<u32>) {}
        assert_is_system(my_system);
    }

    #[derive(Resource)]
    pub struct R<const I: usize>;

    // Compile test for https://github.com/bevyengine/bevy/pull/7001.
    #[test]
    fn system_param_const_generics() {
        #[allow(dead_code)]
        #[derive(SystemParam)]
        pub struct ConstGenericParam<'w, const I: usize>(Res<'w, R<I>>);

        fn my_system(_: ConstGenericParam<0>, _: ConstGenericParam<1000>) {}
        assert_is_system(my_system);
    }

    // Compile test for https://github.com/bevyengine/bevy/pull/6867.
    #[test]
    fn system_param_field_limit() {
        #[derive(SystemParam)]
        pub struct LongParam<'w> {
            // Each field should be a distinct type so there will
            // be an error if the derive messes up the field order.
            _r0: Res<'w, R<0>>,
            _r1: Res<'w, R<1>>,
            _r2: Res<'w, R<2>>,
            _r3: Res<'w, R<3>>,
            _r4: Res<'w, R<4>>,
            _r5: Res<'w, R<5>>,
            _r6: Res<'w, R<6>>,
            _r7: Res<'w, R<7>>,
            _r8: Res<'w, R<8>>,
            _r9: Res<'w, R<9>>,
            _r10: Res<'w, R<10>>,
            _r11: Res<'w, R<11>>,
            _r12: Res<'w, R<12>>,
            _r13: Res<'w, R<13>>,
            _r14: Res<'w, R<14>>,
            _r15: Res<'w, R<15>>,
            _r16: Res<'w, R<16>>,
        }

        fn long_system(_: LongParam) {}
        assert_is_system(long_system);
    }

    // Compile test for https://github.com/bevyengine/bevy/pull/6919.
    // Regression test for https://github.com/bevyengine/bevy/issues/7447.
    #[test]
    fn system_param_phantom_data() {
        #[derive(SystemParam)]
        struct PhantomParam<'w, T: Resource, Marker: 'static> {
            _foo: Res<'w, T>,
            marker: PhantomData<&'w Marker>,
        }

        fn my_system(_: PhantomParam<R<0>, ()>) {}
        assert_is_system(my_system);
    }

    // Compile tests for https://github.com/bevyengine/bevy/pull/6957.
    #[test]
    fn system_param_struct_variants() {
        #[derive(SystemParam)]
        pub struct UnitParam;

        #[allow(dead_code)]
        #[derive(SystemParam)]
        pub struct TupleParam<'w, 's, R: Resource, L: FromWorld + Send + 'static>(
            Res<'w, R>,
            Local<'s, L>,
        );

        fn my_system(_: UnitParam, _: TupleParam<R<0>, u32>) {}
        assert_is_system(my_system);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/4200.
    #[test]
    fn system_param_private_fields() {
        #[derive(Resource)]
        struct PrivateResource;

        #[allow(dead_code)]
        #[derive(SystemParam)]
        pub struct EncapsulatedParam<'w>(Res<'w, PrivateResource>);

        fn my_system(_: EncapsulatedParam) {}
        assert_is_system(my_system);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/7103.
    #[test]
    fn system_param_where_clause() {
        #[derive(SystemParam)]
        pub struct WhereParam<'w, 's, D>
        where
            D: 'static + QueryData,
        {
            _q: Query<'w, 's, D, ()>,
        }

        fn my_system(_: WhereParam<()>) {}
        assert_is_system(my_system);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/1727.
    #[test]
    fn system_param_name_collision() {
        #[derive(Resource)]
        pub struct FetchState;

        #[derive(SystemParam)]
        pub struct Collide<'w> {
            _x: Res<'w, FetchState>,
        }

        fn my_system(_: Collide) {}
        assert_is_system(my_system);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/8192.
    #[test]
    fn system_param_invariant_lifetime() {
        #[derive(SystemParam)]
        pub struct InvariantParam<'w, 's> {
            _set: ParamSet<'w, 's, (Query<'w, 's, ()>,)>,
        }

        fn my_system(_: InvariantParam) {}
        assert_is_system(my_system);
    }

    // Compile test for https://github.com/bevyengine/bevy/pull/9589.
    #[test]
    fn non_sync_local() {
        fn non_sync_system(cell: Local<RefCell<u8>>) {
            assert_eq!(*cell.borrow(), 0);
        }

        let mut world = World::new();
        let mut schedule = crate::schedule::Schedule::default();
        schedule.add_systems(non_sync_system);
        schedule.run(&mut world);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/10207.
    #[test]
    fn param_set_non_send_first() {
        fn non_send_param_set(mut p: ParamSet<(NonSend<*mut u8>, ())>) {
            let _ = p.p0();
            p.p1();
        }

        let mut world = World::new();
        world.insert_non_send_resource(std::ptr::null_mut::<u8>());
        let mut schedule = crate::schedule::Schedule::default();
        schedule.add_systems((non_send_param_set, non_send_param_set, non_send_param_set));
        schedule.run(&mut world);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/10207.
    #[test]
    fn param_set_non_send_second() {
        fn non_send_param_set(mut p: ParamSet<((), NonSendMut<*mut u8>)>) {
            p.p0();
            let _ = p.p1();
        }

        let mut world = World::new();
        world.insert_non_send_resource(std::ptr::null_mut::<u8>());
        let mut schedule = crate::schedule::Schedule::default();
        schedule.add_systems((non_send_param_set, non_send_param_set, non_send_param_set));
        schedule.run(&mut world);
    }
}
