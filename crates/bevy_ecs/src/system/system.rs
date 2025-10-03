#![expect(
    clippy::module_inception,
    reason = "This instance of module inception is being discussed; see #17353."
)]
use bevy_utils::prelude::DebugName;
use bitflags::bitflags;
use core::fmt::{Debug, Display};
use log::warn;

use crate::{
    component::{CheckChangeTicks, Tick},
    error::BevyError,
    query::FilteredAccessSet,
    schedule::InternedSystemSet,
    system::{input::SystemInput, SystemIn},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World},
};

use alloc::{boxed::Box, vec::Vec};
use core::any::{Any, TypeId};

use super::{IntoSystem, SystemParamValidationError};

bitflags! {
    /// Bitflags representing system states and requirements.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SystemStateFlags: u8 {
        /// Set if system cannot be sent across threads
        const NON_SEND       = 1 << 0;
        /// Set if system requires exclusive World access
        const EXCLUSIVE      = 1 << 1;
        /// Set if system has deferred buffers.
        const DEFERRED       = 1 << 2;
    }
}
/// An ECS system that can be added to a [`Schedule`](crate::schedule::Schedule)
///
/// Systems are functions with all arguments implementing
/// [`SystemParam`](crate::system::SystemParam).
///
/// Systems are added to an application using `App::add_systems(Update, my_system)`
/// or similar methods, and will generally run once per pass of the main loop.
///
/// Systems are executed in parallel, in opportunistic order; data access is managed automatically.
/// It's possible to specify explicit execution order between specific systems,
/// see [`IntoScheduleConfigs`](crate::schedule::IntoScheduleConfigs).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a system", label = "invalid system")]
pub trait System: Send + Sync + 'static {
    /// The system's input.
    type In: SystemInput;
    /// The system's output.
    type Out;

    /// Returns the system's name.
    fn name(&self) -> DebugName;
    /// Returns the [`TypeId`] of the underlying system type.
    #[inline]
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// Returns the [`SystemStateFlags`] of the system.
    fn flags(&self) -> SystemStateFlags;

    /// Returns true if the system is [`Send`].
    #[inline]
    fn is_send(&self) -> bool {
        !self.flags().intersects(SystemStateFlags::NON_SEND)
    }

    /// Returns true if the system must be run exclusively.
    #[inline]
    fn is_exclusive(&self) -> bool {
        self.flags().intersects(SystemStateFlags::EXCLUSIVE)
    }

    /// Returns true if system has deferred buffers.
    #[inline]
    fn has_deferred(&self) -> bool {
        self.flags().intersects(SystemStateFlags::DEFERRED)
    }

    /// Runs the system with the given input in the world. Unlike [`System::run`], this function
    /// can be called in parallel with other systems and may break Rust's aliasing rules
    /// if used incorrectly, making it unsafe to call.
    ///
    /// Unlike [`System::run`], this will not apply deferred parameters, which must be independently
    /// applied by calling [`System::apply_deferred`] at later point in time.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that [`world`](UnsafeWorldCell) has permission to access any world data
    ///   registered in the access returned from [`System::initialize`]. There must be no conflicting
    ///   simultaneous accesses while the system is running.
    /// - If [`System::is_exclusive`] returns `true`, then it must be valid to call
    ///   [`UnsafeWorldCell::world_mut`] on `world`.
    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError>;

    /// Refresh the inner pointer based on the latest hot patch jump table
    #[cfg(feature = "hotpatching")]
    fn refresh_hotpatch(&mut self);

    /// Runs the system with the given input in the world.
    ///
    /// For [read-only](ReadOnlySystem) systems, see [`run_readonly`], which can be called using `&World`.
    ///
    /// Unlike [`System::run_unsafe`], this will apply deferred parameters *immediately*.
    ///
    /// [`run_readonly`]: ReadOnlySystem::run_readonly
    fn run(
        &mut self,
        input: SystemIn<'_, Self>,
        world: &mut World,
    ) -> Result<Self::Out, RunSystemError> {
        let ret = self.run_without_applying_deferred(input, world)?;
        self.apply_deferred(world);
        Ok(ret)
    }

    /// Runs the system with the given input in the world.
    ///
    /// [`run_readonly`]: ReadOnlySystem::run_readonly
    fn run_without_applying_deferred(
        &mut self,
        input: SystemIn<'_, Self>,
        world: &mut World,
    ) -> Result<Self::Out, RunSystemError> {
        let world_cell = world.as_unsafe_world_cell();
        // SAFETY:
        // - We have exclusive access to the entire world.
        unsafe { self.validate_param_unsafe(world_cell) }?;
        // SAFETY:
        // - We have exclusive access to the entire world.
        unsafe { self.run_unsafe(input, world_cell) }
    }

    /// Applies any [`Deferred`](crate::system::Deferred) system parameters (or other system buffers) of this system to the world.
    ///
    /// This is where [`Commands`](crate::system::Commands) get applied.
    fn apply_deferred(&mut self, world: &mut World);

    /// Enqueues any [`Deferred`](crate::system::Deferred) system parameters (or other system buffers)
    /// of this system into the world's command buffer.
    fn queue_deferred(&mut self, world: DeferredWorld);

    /// Validates that all parameters can be acquired and that system can run without panic.
    /// Built-in executors use this to prevent invalid systems from running.
    ///
    /// However calling and respecting [`System::validate_param_unsafe`] or its safe variant
    /// is not a strict requirement, both [`System::run`] and [`System::run_unsafe`]
    /// should provide their own safety mechanism to prevent undefined behavior.
    ///
    /// This method has to be called directly before [`System::run_unsafe`] with no other (relevant)
    /// world mutations in between. Otherwise, while it won't lead to any undefined behavior,
    /// the validity of the param may change.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that [`world`](UnsafeWorldCell) has permission to access any world data
    ///   registered in the access returned from [`System::initialize`]. There must be no conflicting
    ///   simultaneous accesses while the system is running.
    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError>;

    /// Safe version of [`System::validate_param_unsafe`].
    /// that runs on exclusive, single-threaded `world` pointer.
    fn validate_param(&mut self, world: &World) -> Result<(), SystemParamValidationError> {
        let world_cell = world.as_unsafe_world_cell_readonly();
        // SAFETY:
        // - We have exclusive access to the entire world.
        unsafe { self.validate_param_unsafe(world_cell) }
    }

    /// Initialize the system.
    ///
    /// Returns a [`FilteredAccessSet`] with the access required to run the system.
    fn initialize(&mut self, _world: &mut World) -> FilteredAccessSet;

    /// Checks any [`Tick`]s stored on this system and wraps their value if they get too old.
    ///
    /// This method must be called periodically to ensure that change detection behaves correctly.
    /// When using bevy's default configuration, this will be called for you as needed.
    fn check_change_tick(&mut self, check: CheckChangeTicks);

    /// Returns the system's default [system sets](crate::schedule::SystemSet).
    ///
    /// Each system will create a default system set that contains the system.
    fn default_system_sets(&self) -> Vec<InternedSystemSet> {
        Vec::new()
    }

    /// Gets the tick indicating the last time this system ran.
    fn get_last_run(&self) -> Tick;

    /// Overwrites the tick indicating the last time this system ran.
    ///
    /// # Warning
    /// This is a complex and error-prone operation, that can have unexpected consequences on any system relying on this code.
    /// However, it can be an essential escape hatch when, for example,
    /// you are trying to synchronize representations using change detection and need to avoid infinite recursion.
    fn set_last_run(&mut self, last_run: Tick);
}

/// [`System`] types that do not modify the [`World`] when run.
/// This is implemented for any systems whose parameters all implement [`ReadOnlySystemParam`].
///
/// Note that systems which perform [deferred](System::apply_deferred) mutations (such as with [`Commands`])
/// may implement this trait.
///
/// [`ReadOnlySystemParam`]: crate::system::ReadOnlySystemParam
/// [`Commands`]: crate::system::Commands
///
/// # Safety
///
/// This must only be implemented for system types which do not mutate the `World`
/// when [`System::run_unsafe`] is called.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a read-only system",
    label = "invalid read-only system"
)]
pub unsafe trait ReadOnlySystem: System {
    /// Runs this system with the given input in the world.
    ///
    /// Unlike [`System::run`], this can be called with a shared reference to the world,
    /// since this system is known not to modify the world.
    fn run_readonly(
        &mut self,
        input: SystemIn<'_, Self>,
        world: &World,
    ) -> Result<Self::Out, RunSystemError> {
        let world = world.as_unsafe_world_cell_readonly();
        // SAFETY:
        // - We have read-only access to the entire world.
        unsafe { self.validate_param_unsafe(world) }?;
        // SAFETY:
        // - We have read-only access to the entire world.
        unsafe { self.run_unsafe(input, world) }
    }
}

/// A convenience type alias for a boxed [`System`] trait object.
pub type BoxedSystem<In = (), Out = ()> = Box<dyn System<In = In, Out = Out>>;

/// A convenience type alias for a boxed [`ReadOnlySystem`] trait object.
pub type BoxedReadOnlySystem<In = (), Out = ()> = Box<dyn ReadOnlySystem<In = In, Out = Out>>;

pub(crate) fn check_system_change_tick(
    last_run: &mut Tick,
    check: CheckChangeTicks,
    system_name: DebugName,
) {
    if last_run.check_tick(check) {
        let age = check.present_tick().relative_to(*last_run).get();
        warn!(
            "System '{system_name}' has not run for {age} ticks. \
            Changes older than {} ticks will not be detected.",
            Tick::MAX.get() - 1,
        );
    }
}

impl<In, Out> Debug for dyn System<In = In, Out = Out>
where
    In: SystemInput + 'static,
    Out: 'static,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("System")
            .field("name", &self.name())
            .field("is_exclusive", &self.is_exclusive())
            .field("is_send", &self.is_send())
            .finish_non_exhaustive()
    }
}

/// Trait used to run a system immediately on a [`World`].
///
/// # Warning
/// This function is not an efficient method of running systems and it's meant to be used as a utility
/// for testing and/or diagnostics.
///
/// Systems called through [`run_system_once`](RunSystemOnce::run_system_once) do not hold onto any state,
/// as they are created and destroyed every time [`run_system_once`](RunSystemOnce::run_system_once) is called.
/// Practically, this means that [`Local`](crate::system::Local) variables are
/// reset on every run and change detection does not work.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::RunSystemOnce;
/// #[derive(Resource, Default)]
/// struct Counter(u8);
///
/// fn increment(mut counter: Local<Counter>) {
///    counter.0 += 1;
///    println!("{}", counter.0);
/// }
///
/// let mut world = World::default();
/// world.run_system_once(increment); // prints 1
/// world.run_system_once(increment); // still prints 1
/// ```
///
/// If you do need systems to hold onto state between runs, use [`World::run_system_cached`](World::run_system_cached)
/// or [`World::run_system`](World::run_system).
///
/// # Usage
/// Typically, to test a system, or to extract specific diagnostics information from a world,
/// you'd need a [`Schedule`](crate::schedule::Schedule) to run the system. This can create redundant boilerplate code
/// when writing tests or trying to quickly iterate on debug specific systems.
///
/// For these situations, this function can be useful because it allows you to execute a system
/// immediately with some custom input and retrieve its output without requiring the necessary boilerplate.
///
/// # Examples
///
/// ## Immediate Command Execution
///
/// This usage is helpful when trying to test systems or functions that operate on [`Commands`](crate::system::Commands):
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::RunSystemOnce;
/// let mut world = World::default();
/// let entity = world.run_system_once(|mut commands: Commands| {
///     commands.spawn_empty().id()
/// }).unwrap();
/// # assert!(world.get_entity(entity).is_ok());
/// ```
///
/// ## Immediate Queries
///
/// This usage is helpful when trying to run an arbitrary query on a world for testing or debugging purposes:
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::RunSystemOnce;
///
/// #[derive(Component)]
/// struct T(usize);
///
/// let mut world = World::default();
/// world.spawn(T(0));
/// world.spawn(T(1));
/// world.spawn(T(1));
/// let count = world.run_system_once(|query: Query<&T>| {
///     query.iter().filter(|t| t.0 == 1).count()
/// }).unwrap();
///
/// # assert_eq!(count, 2);
/// ```
///
/// Note that instead of closures you can also pass in regular functions as systems:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::RunSystemOnce;
///
/// #[derive(Component)]
/// struct T(usize);
///
/// fn count(query: Query<&T>) -> usize {
///     query.iter().filter(|t| t.0 == 1).count()
/// }
///
/// let mut world = World::default();
/// world.spawn(T(0));
/// world.spawn(T(1));
/// world.spawn(T(1));
/// let count = world.run_system_once(count).unwrap();
///
/// # assert_eq!(count, 2);
/// ```
pub trait RunSystemOnce: Sized {
    /// Tries to run a system and apply its deferred parameters.
    fn run_system_once<T, Out, Marker>(self, system: T) -> Result<Out, RunSystemError>
    where
        T: IntoSystem<(), Out, Marker>,
    {
        self.run_system_once_with(system, ())
    }

    /// Tries to run a system with given input and apply deferred parameters.
    fn run_system_once_with<T, In, Out, Marker>(
        self,
        system: T,
        input: SystemIn<'_, T::System>,
    ) -> Result<Out, RunSystemError>
    where
        T: IntoSystem<In, Out, Marker>,
        In: SystemInput;
}

impl RunSystemOnce for &mut World {
    fn run_system_once_with<T, In, Out, Marker>(
        self,
        system: T,
        input: SystemIn<'_, T::System>,
    ) -> Result<Out, RunSystemError>
    where
        T: IntoSystem<In, Out, Marker>,
        In: SystemInput,
    {
        let mut system: T::System = IntoSystem::into_system(system);
        system.initialize(self);
        system.run(input, self)
    }
}

/// Running system failed.
#[derive(Debug)]
pub enum RunSystemError {
    /// System could not be run due to parameters that failed validation.
    /// This is not considered an error.
    Skipped(SystemParamValidationError),
    /// System returned an error or failed required parameter validation.
    Failed(BevyError),
}

impl Display for RunSystemError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Skipped(err) => write!(
                f,
                "System did not run due to failed parameter validation: {err}"
            ),
            Self::Failed(err) => write!(f, "{err}"),
        }
    }
}

impl<E: Any> From<E> for RunSystemError
where
    BevyError: From<E>,
{
    fn from(mut value: E) -> Self {
        // Specialize the impl so that a skipped `SystemParamValidationError`
        // is converted to `Skipped` instead of `Failed`.
        // Note that the `downcast_mut` check is based on the static type,
        // and can be optimized out after monomorphization.
        let any: &mut dyn Any = &mut value;
        if let Some(err) = any.downcast_mut::<SystemParamValidationError>() {
            if err.skipped {
                return Self::Skipped(core::mem::replace(err, SystemParamValidationError::EMPTY));
            }
        }
        Self::Failed(From::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use alloc::string::ToString;

    #[test]
    fn run_system_once() {
        struct T(usize);

        impl Resource for T {}

        fn system(In(n): In<usize>, mut commands: Commands) -> usize {
            commands.insert_resource(T(n));
            n + 1
        }

        let mut world = World::default();
        let n = world.run_system_once_with(system, 1).unwrap();
        assert_eq!(n, 2);
        assert_eq!(world.resource::<T>().0, 1);
    }

    #[derive(Resource, Default, PartialEq, Debug)]
    struct Counter(u8);

    fn count_up(mut counter: ResMut<Counter>) {
        counter.0 += 1;
    }

    #[test]
    fn run_two_systems() {
        let mut world = World::new();
        world.init_resource::<Counter>();
        assert_eq!(*world.resource::<Counter>(), Counter(0));
        world.run_system_once(count_up).unwrap();
        assert_eq!(*world.resource::<Counter>(), Counter(1));
        world.run_system_once(count_up).unwrap();
        assert_eq!(*world.resource::<Counter>(), Counter(2));
    }

    #[derive(Component)]
    struct A;

    fn spawn_entity(mut commands: Commands) {
        commands.spawn(A);
    }

    #[test]
    fn command_processing() {
        let mut world = World::new();
        assert_eq!(world.query::<&A>().query(&world).count(), 0);
        world.run_system_once(spawn_entity).unwrap();
        assert_eq!(world.query::<&A>().query(&world).count(), 1);
    }

    #[test]
    fn non_send_resources() {
        fn non_send_count_down(mut ns: NonSendMut<Counter>) {
            ns.0 -= 1;
        }

        let mut world = World::new();
        world.insert_non_send_resource(Counter(10));
        assert_eq!(*world.non_send_resource::<Counter>(), Counter(10));
        world.run_system_once(non_send_count_down).unwrap();
        assert_eq!(*world.non_send_resource::<Counter>(), Counter(9));
    }

    #[test]
    fn run_system_once_invalid_params() {
        struct T;
        impl Resource for T {}
        fn system(_: Res<T>) {}

        let mut world = World::default();
        // This fails because `T` has not been added to the world yet.
        let result = world.run_system_once(system);

        assert!(matches!(result, Err(RunSystemError::Failed { .. })));

        let expected = "Resource does not exist";
        let actual = result.unwrap_err().to_string();

        assert!(
            actual.contains(expected),
            "Expected error message to contain `{}` but got `{}`",
            expected,
            actual
        );
    }
}
