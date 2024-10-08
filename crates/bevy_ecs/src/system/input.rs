use core::ops::{Deref, DerefMut};

use crate::{
    archetype::Archetype,
    bundle::Bundle,
    component::Tick,
    entity::Entity,
    prelude::Trigger,
    query::{QueryData, QueryState},
    system::{init_query_param, System, SystemMeta},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

/// Trait for types that can be used as input to [`System`]s.
///
/// Provided implementations are:
/// - `()`: No input
/// - [`In<T>`]: For values
/// - [`InRef<T>`]: For read-only references to values
/// - [`InMut<T>`]: For mutable references to values
/// - [`Trigger<E, B>`]: For [`ObserverSystem`]s
/// - [`StaticSystemInput<I>`]: For arbitrary [`SystemInput`]s in generic contexts
///
/// # Safety
///
/// Implementors must ensure the following is true:
/// - [`SystemInput::init_state`] correctly registers all [`World`] accesses used
///   by [`SystemInput::get_input`] with the provided [`system_meta`](SystemMeta).
/// - None of the world accesses may conflict with any prior accesses registered
///   on `system_meta`.
///
/// [`ObserverSystem`]: crate::system::ObserverSystem
pub unsafe trait SystemInput: Sized {
    /// Used to store data which persists across invocations of a system.
    type State: Send + Sync + 'static;

    /// The wrapper input type that is defined as the first argument to
    /// [`FunctionSystem`]s.
    ///
    /// [`FunctionSystem`]: crate::system::FunctionSystem
    type Item<'world, 'state, 'input>: SystemInput<
        State = Self::State,
        Inner<'input> = Self::Inner<'input>,
    >;

    /// The inner input type that is passed to functions that run systems,
    /// such as [`System::run`].
    ///
    /// [`System::run`]: crate::system::System::run
    type Inner<'input>;

    /// Registers any [`World`] access used by this [`SystemInput`]
    /// and creates a new instance of this param's [`State`](SystemInput::State).
    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State;

    /// For the specified [`Archetype`], registers the components accessed by
    /// this [`SystemInput`] (if applicable).
    ///
    /// # Safety
    ///
    /// `archetype` must be from the [`World`] used to [initialize `state`](SystemInput::init_state).
    #[inline]
    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        let _ = (state, archetype, system_meta);
    }

    /// Validates that the input can be acquired by the
    /// [`get_input`](SystemInput::get_input) function. Built-in executors use
    /// this to prevent systems with invalid params from running.
    ///
    /// However calling and respecting [`SystemInput::validate_input`] is not a
    /// strict requirement, [`SystemInput::get_input`] should provide its own
    /// safety mechanism to prevent undefined behavior.
    ///
    /// The [`world`](UnsafeWorldCell) can only be used to read the input's
    /// queried data and world metadata. No data can be written.
    ///
    /// When using system input that require `change_tick`, you can use
    /// [`UnsafeWorldCell::change_tick`]. Even if this isn't the exact
    /// same tick used for [`SystemInput::get_input`], the world access
    /// ensures that the queried data will be the same in both calls.
    ///
    /// This method has to be called directly before [`SystemInput::get_input`]
    /// with no other (relevant) world mutations in-between. Otherwise, while
    /// it won't lead to any undefined behavior, the validity of the param may
    /// change.
    ///
    /// # Safety
    ///
    /// - The passed [`UnsafeWorldCell`] must have read-only access to world
    ///   data registered in [`init_state`](SystemInput::init_state).
    /// - `world` must the same [`World`] that was used to
    ///   [initialize `state`](SystemInput::init_state).
    /// - All `world` archetypes have been processed by
    ///   [`new_archetype`](SystemInput::new_archetype).
    unsafe fn validate_input(
        input: &Self::Inner<'_>,
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        let _ = (input, state, system_meta, world);
        // By default we allow panics in [`SystemInput::get_input`] and return `true`.
        // Preventing panics is an optional feature.
        true
    }

    /// Creates an input value to be passed into a
    /// [`SystemParamFunction`](super::SystemParamFunction).
    ///
    /// # Safety
    ///
    /// - The passed [`UnsafeWorldCell`] must have access to any world data
    ///   registered in [`init_state`](SystemInput::init_state).
    /// - `world` must be the same [`World`] that was used to [initialize `state`](SystemInput::init_state).
    /// - All `world` archetypes have been processed by [`new_archetype`](SystemInput::new_archetype).
    unsafe fn get_input<'world, 'state, 'input>(
        input: Self::Inner<'input>,
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state, 'input>;
}

/// Trait for types that can be used as input to exclusive [`System`]s.
///
/// Provided implementations are:
/// - `()`: No input
/// - [`In<T>`]: For values
/// - [`InRef<T>`]: For read-only references to values
/// - [`InMut<T>`]: For mutable references to values
/// - [`StaticSystemInput<I>`]: For arbitrary [`SystemInput`]s in generic contexts
pub trait ExclusiveSystemInput: SystemInput {
    /// Creates an input value to be passed into an
    /// [`ExclusiveSystemParamFunction`].
    ///
    /// [`ExclusiveSystemParamFunction`]: crate::system::ExclusiveSystemParamFunction
    fn get_einput<'state, 'input>(
        input: Self::Inner<'input>,
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
    ) -> Self::Item<'static, 'state, 'input>;
}

/// Shorthand way to get the [`System::In`] for a [`System`] as a
/// [`SystemInput::Inner`].
pub type SystemIn<'a, S> = <<S as System>::In as SystemInput>::Inner<'a>;

/// Shorthand way of accessing the associated type [`SystemInput::Item`] for a
/// given [`SystemInput`].
pub type SystemInputItem<'w, 's, 'i, S> = <S as SystemInput>::Item<'w, 's, 'i>;

/// [`SystemInput`] type for systems that take no input.
// SAFETY: Doesn't access any world data.
unsafe impl SystemInput for () {
    type State = ();
    type Item<'world, 'state, 'input> = ();
    type Inner<'input> = ();

    fn init_state(_world: &mut World, _system_metaa: &mut SystemMeta) -> Self::State {}

    unsafe fn get_input<'world, 'state, 'input>(
        _input: Self::Inner<'input>,
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state, 'input> {
    }
}

impl ExclusiveSystemInput for () {
    fn get_einput<'state, 'input>(
        _input: Self::Inner<'input>,
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
    ) -> Self::Item<'static, 'state, 'input> {
    }
}

/// A [`SystemInput`] type which denotes that a [`System`] receives
/// an input value of type `T` from its caller.
///
/// [`System`]s may take an optional input which they require to be passed to them when they
/// are being [`run`](System::run). For [`FunctionSystem`]s the input may be marked
/// with this `In` type, but only the first param of a function may be tagged as an input. This also
/// means a system can only have one or zero input parameters.
///
/// # Examples
///
/// Here is a simple example of a system that takes a [`usize`] and returns the square of it.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// fn square(In(input): In<usize>) -> usize {
///     input * input
/// }
///
/// let mut world = World::new();
/// let mut square_system = IntoSystem::into_system(square);
/// square_system.initialize(&mut world);
///
/// assert_eq!(square_system.run(12, &mut world), 144);
/// ```
///
/// [`SystemParam`]: crate::system::SystemParam
/// [`FunctionSystem`]: crate::system::FunctionSystem
#[derive(Debug)]
pub struct In<T>(pub T);

// SAFETY: Doesn't access any world data.
unsafe impl<T: 'static> SystemInput for In<T> {
    type State = ();
    type Item<'world, 'state, 'input> = In<T>;
    type Inner<'input> = T;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    unsafe fn get_input<'world, 'state, 'input>(
        input: Self::Inner<'input>,
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state, 'input> {
        In(input)
    }
}

impl<T: 'static> ExclusiveSystemInput for In<T> {
    fn get_einput<'state, 'input>(
        input: Self::Inner<'input>,
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
    ) -> Self::Item<'static, 'state, 'input> {
        In(input)
    }
}

impl<T> Deref for In<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for In<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A [`SystemInput`] type which denotes that a [`System`] receives
/// a read-only reference to a value of type `T` from its caller.
///
/// This is similar to [`In`] but takes a reference to a value instead of the value itself.
/// See [`InMut`] for the mutable version.
///
/// # Examples
///
/// Here is a simple example of a system that logs the passed in message.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use std::fmt::Write as _;
/// #
/// #[derive(Resource, Default)]
/// struct Log(String);
///
/// fn log(InRef(msg): InRef<str>, mut log: ResMut<Log>) {
///     writeln!(log.0, "{}", msg).unwrap();
/// }
///
/// let mut world = World::new();
/// world.init_resource::<Log>();
/// let mut log_system = IntoSystem::into_system(log);
/// log_system.initialize(&mut world);
///
/// log_system.run("Hello, world!", &mut world);
/// # assert_eq!(world.get_resource::<Log>().unwrap().0, "Hello, world!\n");
/// ```
///
/// [`SystemParam`]: crate::system::SystemParam
#[derive(Debug)]
pub struct InRef<'i, T: ?Sized>(pub &'i T);

// SAFETY: Doesn't access any world data.
unsafe impl<T: ?Sized + 'static> SystemInput for InRef<'_, T> {
    type State = ();
    type Item<'world, 'state, 'input> = InRef<'input, T>;
    type Inner<'input> = &'input T;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    unsafe fn get_input<'world, 'state, 'input>(
        input: Self::Inner<'input>,
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state, 'input> {
        InRef(input)
    }
}

impl<T: ?Sized + 'static> ExclusiveSystemInput for InRef<'_, T> {
    fn get_einput<'state, 'input>(
        input: Self::Inner<'input>,
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
    ) -> Self::Item<'static, 'state, 'input> {
        InRef(input)
    }
}

impl<'i, T: ?Sized> Deref for InRef<'i, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// A [`SystemInput`] type which denotes that a [`System`] receives
/// a mutable reference to a value of type `T` from its caller.
///
/// This is similar to [`In`] but takes a mutable reference to a value instead of the value itself.
/// See [`InRef`] for the read-only version.
///
/// # Examples
///
/// Here is a simple example of a system that takes a `&mut usize` and squares it.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// fn square(InMut(input): InMut<usize>) {
///     *input *= *input;
/// }
///
/// let mut world = World::new();
/// let mut square_system = IntoSystem::into_system(square);
/// square_system.initialize(&mut world);
///     
/// let mut value = 12;
/// square_system.run(&mut value, &mut world);
/// assert_eq!(value, 144);
/// ```
///
/// [`SystemParam`]: crate::system::SystemParam
#[derive(Debug)]
pub struct InMut<'a, T: ?Sized>(pub &'a mut T);

// SAFETY: Doesn't access any world data.
unsafe impl<T: ?Sized + 'static> SystemInput for InMut<'_, T> {
    type State = ();

    type Item<'world, 'state, 'input> = InMut<'input, T>;

    type Inner<'input> = &'input mut T;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    unsafe fn get_input<'world, 'state, 'input>(
        input: Self::Inner<'input>,
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state, 'input> {
        InMut(input)
    }
}

impl<T: ?Sized + 'static> ExclusiveSystemInput for InMut<'_, T> {
    fn get_einput<'state, 'input>(
        input: Self::Inner<'input>,
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
    ) -> Self::Item<'static, 'state, 'input> {
        InMut(input)
    }
}

impl<'i, T: ?Sized> Deref for InMut<'i, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'i, T: ?Sized> DerefMut for InMut<'i, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

/// Used for [`ObserverSystem`]s.
///
/// [`ObserverSystem`]: crate::system::ObserverSystem
// SAFETY: All world access is registered in `init_state`.
unsafe impl<E: 'static, B: Bundle, D: QueryData + 'static> SystemInput
    for Trigger<'_, '_, E, B, D>
{
    type State = QueryState<D>;
    type Item<'world, 'state, 'input> = Trigger<'world, 'input, E, B, D>;
    type Inner<'input> = Trigger<'static, 'input, E, B, ()>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let state = QueryState::new_with_access(world, &mut system_meta.archetype_component_access);
        init_query_param(world, system_meta, &state);
        state
    }

    unsafe fn validate_input(
        input: &Self::Inner<'_>,
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        if input.entity() == Entity::PLACEHOLDER {
            return true;
        }

        state.validate_world(world.id());
        // SAFETY: We registered access to the components in `init_state`.
        let result = unsafe {
            state.as_readonly().get_unchecked_manual(
                world,
                input.entity(),
                system_meta.last_run,
                world.change_tick(),
            )
        };
        result
            .inspect_err(|e| {
                // TODO system warn
                eprintln!("{}", e);
            })
            .is_ok()
    }

    unsafe fn get_input<'world, 'state, 'input>(
        input: Self::Inner<'input>,
        state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state, 'input> {
        let entity = input.entity();

        // SAFETY: We registered access to the components in `init_state`.
        let data = unsafe { state.get_unchecked(world, entity) };
        input.with_data(data.ok())
    }
}

/// A helper for using [`SystemInput`]s in generic contexts.
///
/// This type is a [`SystemInput`] adapter which always has
/// `Self::Param == Self` (ignoring lifetimes for brevity),
/// no matter the argument [`SystemInput`] (`I`).
///
/// This makes it useful for having arbitrary [`SystemInput`]s in
/// function systems.
pub struct StaticSystemInput<'w, 's, 'i, I: SystemInput>(pub I::Item<'w, 's, 'i>);

// SAFETY: All safety requirements are delegated to the inner `SystemInput`.
unsafe impl<I: SystemInput> SystemInput for StaticSystemInput<'_, '_, '_, I> {
    type State = I::State;

    type Item<'world, 'state, 'input> = StaticSystemInput<'world, 'state, 'input, I>;

    type Inner<'input> = I::Inner<'input>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        I::init_state(world, system_meta)
    }

    unsafe fn validate_input(
        input: &Self::Inner<'_>,
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        I::validate_input(input, state, system_meta, world)
    }

    unsafe fn get_input<'world, 'state, 'input>(
        input: Self::Inner<'input>,
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state, 'input> {
        StaticSystemInput(I::get_input(input, state, system_meta, world, change_tick))
    }
}

impl<I: ExclusiveSystemInput> ExclusiveSystemInput for StaticSystemInput<'static, '_, '_, I> {
    fn get_einput<'state, 'input>(
        input: Self::Inner<'input>,
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
    ) -> Self::Item<'static, 'state, 'input> {
        StaticSystemInput(I::get_einput(input, state, system_meta))
    }
}
