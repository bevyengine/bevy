use bevy_utils::tracing::warn;
use core::fmt::Debug;

use crate::component::Tick;
use crate::world::unsafe_world_cell::UnsafeWorldCell;
use crate::{archetype::ArchetypeComponentId, component::ComponentId, query::Access, world::World};

use std::any::TypeId;
use std::borrow::Cow;

use super::IntoSystem;

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
/// see [`IntoSystemConfigs`](crate::schedule::IntoSystemConfigs).
pub trait System: Send + Sync + 'static {
    /// The system's input. See [`In`](crate::system::In) for
    /// [`FunctionSystem`](crate::system::FunctionSystem)s.
    type In;
    /// The system's output.
    type Out;
    /// Returns the system's name.
    fn name(&self) -> Cow<'static, str>;
    /// Returns the [`TypeId`] of the underlying system type.
    fn type_id(&self) -> TypeId;
    /// Returns the system's component [`Access`].
    fn component_access(&self) -> &Access<ComponentId>;
    /// Returns the system's archetype component [`Access`].
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId>;
    /// Returns true if the system is [`Send`].
    fn is_send(&self) -> bool;

    /// Returns true if the system must be run exclusively.
    fn is_exclusive(&self) -> bool;

    /// Runs the system with the given input in the world. Unlike [`System::run`], this function
    /// can be called in parallel with other systems and may break Rust's aliasing rules
    /// if used incorrectly, making it unsafe to call.
    ///
    /// # Safety
    ///
    /// - The caller must ensure that `world` has permission to access any world data
    ///   registered in [`Self::archetype_component_access`]. There must be no conflicting
    ///   simultaneous accesses while the system is running.
    /// - The method [`Self::update_archetype_component_access`] must be called at some
    ///   point before this one, with the same exact [`World`]. If `update_archetype_component_access`
    ///   panics (or otherwise does not return for any reason), this method must not be called.
    unsafe fn run_unsafe(&mut self, input: Self::In, world: UnsafeWorldCell) -> Self::Out;

    /// Runs the system with the given input in the world.
    ///
    /// For [read-only](ReadOnlySystem) systems, see [`run_readonly`], which can be called using `&World`.
    ///
    /// [`run_readonly`]: ReadOnlySystem::run_readonly
    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        let world = world.as_unsafe_world_cell();
        self.update_archetype_component_access(world);
        // SAFETY:
        // - We have exclusive access to the entire world.
        // - `update_archetype_component_access` has been called.
        unsafe { self.run_unsafe(input, world) }
    }

    /// Applies any [`Deferred`](crate::system::Deferred) system parameters (or other system buffers) of this system to the world.
    ///
    /// This is where [`Commands`](crate::system::Commands) get applied.
    fn apply_deferred(&mut self, world: &mut World);

    /// Initialize the system.
    fn initialize(&mut self, _world: &mut World);

    /// Update the system's archetype component [`Access`].
    ///
    /// ## Note for implementors
    /// `world` may only be used to access metadata. This can be done in safe code
    /// via functions such as [`UnsafeWorldCell::archetypes`].
    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell);

    /// Checks any [`Tick`]s stored on this system and wraps their value if they get too old.
    ///
    /// This method must be called periodically to ensure that change detection behaves correctly.
    /// When using bevy's default configuration, this will be called for you as needed.
    fn check_change_tick(&mut self, change_tick: Tick);

    /// Returns the system's default [system sets](crate::schedule::SystemSet).
    fn default_system_sets(&self) -> Vec<Box<dyn crate::schedule::SystemSet>> {
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
pub unsafe trait ReadOnlySystem: System {
    /// Runs this system with the given input in the world.
    ///
    /// Unlike [`System::run`], this can be called with a shared reference to the world,
    /// since this system is known not to modify the world.
    fn run_readonly(&mut self, input: Self::In, world: &World) -> Self::Out {
        let world = world.as_unsafe_world_cell_readonly();
        self.update_archetype_component_access(world);
        // SAFETY:
        // - We have read-only access to the entire world.
        // - `update_archetype_component_access` has been called.
        unsafe { self.run_unsafe(input, world) }
    }
}

/// A convenience type alias for a boxed [`System`] trait object.
pub type BoxedSystem<In = (), Out = ()> = Box<dyn System<In = In, Out = Out>>;

pub(crate) fn check_system_change_tick(last_run: &mut Tick, this_run: Tick, system_name: &str) {
    if last_run.check_tick(this_run) {
        let age = this_run.relative_to(*last_run).get();
        warn!(
            "System '{system_name}' has not run for {age} ticks. \
            Changes older than {} ticks will not be detected.",
            Tick::MAX.get() - 1,
        );
    }
}

impl<In: 'static, Out: 'static> Debug for dyn System<In = In, Out = Out> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
/// This function is not an efficient method of running systems and its meant to be used as a utility
/// for testing and/or diagnostics.
///
/// Systems called through [`run_system_once`](crate::system::RunSystemOnce::run_system_once) do not hold onto any state,
/// as they are created and destroyed every time [`run_system_once`](crate::system::RunSystemOnce::run_system_once) is called.
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
/// If you do need systems to hold onto state between runs, use the [`World::run_system`](crate::system::World::run_system)
/// and run the system by their [`SystemId`](crate::system::SystemId).
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
/// });
/// # assert!(world.get_entity(entity).is_some());
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
/// });
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
/// let count = world.run_system_once(count);
///
/// # assert_eq!(count, 2);
/// ```
pub trait RunSystemOnce: Sized {
    /// Runs a system and applies its deferred parameters.
    fn run_system_once<T: IntoSystem<(), Out, Marker>, Out, Marker>(self, system: T) -> Out {
        self.run_system_once_with((), system)
    }

    /// Runs a system with given input and applies its deferred parameters.
    fn run_system_once_with<T: IntoSystem<In, Out, Marker>, In, Out, Marker>(
        self,
        input: In,
        system: T,
    ) -> Out;
}

impl RunSystemOnce for &mut World {
    fn run_system_once_with<T: IntoSystem<In, Out, Marker>, In, Out, Marker>(
        self,
        input: In,
        system: T,
    ) -> Out {
        let mut system: T::System = IntoSystem::into_system(system);
        system.initialize(self);
        let out = system.run(input, self);
        system.apply_deferred(self);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_ecs;
    use crate::prelude::*;

    #[test]
    fn run_system_once() {
        struct T(usize);

        impl Resource for T {}

        fn system(In(n): In<usize>, mut commands: Commands) -> usize {
            commands.insert_resource(T(n));
            n + 1
        }

        let mut world = World::default();
        let n = world.run_system_once_with(1, system);
        assert_eq!(n, 2);
        assert_eq!(world.resource::<T>().0, 1);
    }

    #[derive(Resource, Default, PartialEq, Debug)]
    struct Counter(u8);

    #[allow(dead_code)]
    fn count_up(mut counter: ResMut<Counter>) {
        counter.0 += 1;
    }

    #[test]
    fn run_two_systems() {
        let mut world = World::new();
        world.init_resource::<Counter>();
        assert_eq!(*world.resource::<Counter>(), Counter(0));
        world.run_system_once(count_up);
        assert_eq!(*world.resource::<Counter>(), Counter(1));
        world.run_system_once(count_up);
        assert_eq!(*world.resource::<Counter>(), Counter(2));
    }

    #[allow(dead_code)]
    fn spawn_entity(mut commands: Commands) {
        commands.spawn_empty();
    }

    #[test]
    fn command_processing() {
        let mut world = World::new();
        assert_eq!(world.entities.len(), 0);
        world.run_system_once(spawn_entity);
        assert_eq!(world.entities.len(), 1);
    }

    #[test]
    fn non_send_resources() {
        fn non_send_count_down(mut ns: NonSendMut<Counter>) {
            ns.0 -= 1;
        }

        let mut world = World::new();
        world.insert_non_send_resource(Counter(10));
        assert_eq!(*world.non_send_resource::<Counter>(), Counter(10));
        world.run_system_once(non_send_count_down);
        assert_eq!(*world.non_send_resource::<Counter>(), Counter(9));
    }
}
