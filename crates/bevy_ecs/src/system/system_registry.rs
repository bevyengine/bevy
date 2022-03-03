use bevy_utils::HashMap;
use std::any::{Any, TypeId};
use std::marker::PhantomData;

use crate::system::{Command, IntoSystem, System};
use crate::world::{Mut, World};

/// A [`System`] that cannot be chained.
///
/// [`BoxedSystem`](crate::system::BoxedSystem) is the equivalent type alias for arbitrary `In` and `Out` types.
pub type UnchainedSystem = Box<dyn System<In = (), Out = ()>>;

/// Stores initialized [`Systems`](crate::system::System), so they can quickly be reused and run
///
/// Systems are keyed by their [`TypeId`]: repeated calls with the same function type will reuse cached state,
/// including for change detection.
///
/// Stored systems cannot be chained: they can neither have an [`In`](crate::system::In) nor return any values.
#[derive(Default)]
pub struct SystemRegistry {
    systems: HashMap<TypeId, UnchainedSystem>,
}

// User-facing methods
impl SystemRegistry {
    /// Registers a system in the [`SystemRegistry`], so then it can be later run.
    ///
    /// This only needs to be called manually whenn using [`run_system_by_type_id`](SystemRegistry).
    #[inline]
    pub fn register_system<Params, S: IntoSystem<(), (), Params> + 'static>(
        &mut self,
        world: &mut World,
        system: S,
    ) {
        let label = system.type_id();

        let mut unchained_system: UnchainedSystem = Box::new(IntoSystem::into_system(system));
        unchained_system.initialize(world);

        self.systems.insert(label, unchained_system);
    }

    /// Is the provided `type_id` registered?
    pub fn type_id_registered(&self, type_id: &TypeId) -> bool {
        self.systems.contains_key(type_id)
    }

    /// Runs the system corresponding to the provided [`TypeId`] on the [`World`] a single time
    ///
    /// If `flush_commands` is true, any [`Commands`](crate::system::Commands) generated will also be applied to the world immediately
    pub fn run_system_by_type_id(
        &mut self,
        world: &mut World,
        type_id: TypeId,
        flush_commands: bool,
    ) {
        let initialized_system = self.systems.get_mut(&type_id).unwrap_or_else(||{panic!{"No system with the `TypeId` {type_id:?} was found. Did you forget to register it?"}});
        initialized_system.run((), world);
        if flush_commands {
            initialized_system.apply_buffers(world);
        }
    }

    /// Runs the supplied system on the [`World`] a single time
    ///
    /// If `flush_commands` is true, any [`Commands`](crate::system::Commands) generated will also be applied to the world immediately
    pub fn run_system<Params, S: IntoSystem<(), (), Params> + 'static>(
        &mut self,
        world: &mut World,
        system: S,
        flush_commands: bool,
    ) {
        let type_id = TypeId::of::<S>();
        if self.type_id_registered(&type_id) {
            self.run_system_by_type_id(world, type_id, flush_commands);
        } else {
            self.register_system(world, system);
            self.run_system_by_type_id(world, type_id, flush_commands);
        }
    }
}

impl World {
    /// Registers the supplied system in the [`SystemRegistry`] resource
    ///
    /// This allows the system to be run by [`TypeId`] using [`World::run_system_by_type_id`]
    #[inline]
    pub fn register_system<Params, S: IntoSystem<(), (), Params> + 'static>(&mut self, system: S) {
        self.resource_scope(|world, mut registry: Mut<SystemRegistry>| {
            registry.register_system(world, system);
        });
    }

    /// Runs the supplied system on the [`World`] a single time
    ///
    /// Any [`Commands`](crate::system::Commands) generated will also be applied to the world immediately.
    ///
    /// The system's state will be cached: any future calls using the same type will use this state,
    /// improving performance and ensuring that change detection works properly.
    ///
    /// Unsurprisingly, this is evaluated in a sequential, single-threaded fashion.
    /// Consider creating and running a [`Schedule`](crate::schedule::Schedule) if you need to execute large groups of systems
    /// at once, and want parallel execution of these systems.
    #[inline]
    pub fn run_system<Params, S: IntoSystem<(), (), Params> + 'static>(&mut self, system: S) {
        self.resource_scope(|world, mut registry: Mut<SystemRegistry>| {
            registry.run_system(world, system, true);
        });
    }

    /// Runs the system corresponding to the supplied `type_id` on the [`World`] a single time
    ///
    /// Systems must be registered before they can be run by `type_id`.
    ///
    /// Any [`Commands`](crate::system::Commands) generated will also be applied to the world immediately.
    ///
    /// The system's state will be cached: any future calls using the same type will use this state,
    /// improving performance and ensuring that change detection works properly.
    ///
    /// Unsurprisingly, this is evaluated in a sequential, single-threaded fashion.
    /// Consider creating and running a [`Schedule`](crate::schedule::Schedule) if you need to execute large groups of systems
    /// at once, and want parallel execution of these systems.
    #[inline]
    pub fn run_system_by_type_id(&mut self, type_id: TypeId) {
        self.resource_scope(|world, mut registry: Mut<SystemRegistry>| {
            registry.run_system_by_type_id(world, type_id, true);
        });
    }

    /// Runs the supplied system on the [`World`] a single time, wihthout flushing [`Commands`](crate::system::Commands)
    ///
    /// The system's state will be cached: any future calls using the same type will use this state,
    /// improving performance and ensuring that change detection works properly.
    ///
    /// Unsurprisingly, this is evaluated in a sequential, single-threaded fashion.
    /// Consider creating and running a [`Schedule`](crate::schedule::Schedule) if you need to execute large groups of systems
    /// at once, and want parallel execution of these systems.
    #[inline]
    pub fn run_system_without_flushing<Params, S: IntoSystem<(), (), Params> + 'static>(
        &mut self,
        system: S,
    ) {
        self.resource_scope(|world, mut registry: Mut<SystemRegistry>| {
            registry.run_system(world, system, false);
        });
    }
}

/// The [`Command`] type for [`SystemRegistry::run_system`]
#[derive(Debug, Clone)]
pub struct RunSystemCommand<
    Params: Send + Sync + 'static,
    S: IntoSystem<(), (), Params> + Send + Sync + 'static,
> {
    _phantom_params: PhantomData<Params>,
    system: S,
    flush: bool,
}

impl<Params: Send + Sync + 'static, S: IntoSystem<(), (), Params> + Send + Sync + 'static>
    RunSystemCommand<Params, S>
{
    /// Creates a new [`Command`] struct, which can be addeded to [`Commands`](crate::system::Commands)
    #[inline]
    #[must_use]
    pub fn new(system: S, flush: bool) -> Self {
        Self {
            _phantom_params: PhantomData::default(),
            system,
            flush,
        }
    }
}

impl<Params: Send + Sync + 'static, S: IntoSystem<(), (), Params> + Send + Sync + 'static> Command
    for RunSystemCommand<Params, S>
{
    #[inline]
    fn apply(self, world: &mut World) {
        if self.flush {
            world.run_system(self.system);
        } else {
            world.run_system_without_flushing(self.system);
        }
    }
}

/// The [`Command`] type for [`SystemRegistry::run_system_by_type_id`]
#[derive(Debug, Clone)]
pub struct RunSystemByTypeIdCommand {
    pub type_id: TypeId,
}

impl Command for RunSystemByTypeIdCommand {
    #[inline]
    fn write(self, world: &mut World) {
        world.run_system_by_type_id(self.type_id)
    }
}
