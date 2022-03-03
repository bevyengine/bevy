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
/// Stored systems cannot be chained: they can neither have an [`In`](crate::system::In) nor return any values.
#[derive(Default)]
pub struct SystemRegistry {
    systems: HashMap<TypeId, UnchainedSystem>,
}

// User-facing methods
impl SystemRegistry {
    /// Runs the supplied system on the [`World`] a single time
    ///
    /// Any [`Commands`](crate::system::Commands) generated will also be applied to the world immediately.
    pub fn run_system<Params, S: IntoSystem<(), (), Params> + 'static>(
        &mut self,
        world: &mut World,
        system: S,
    ) {
        // If the system is already registered and initialized, use it
        if let Some(initialized_system) = self.systems.get_mut(&system.type_id()) {
            initialized_system.run((), world);
            initialized_system.apply_buffers(world);
        // Otherwise, register and initialize it first.
        } else {
            let initialized_system = self.register(world, system);
            initialized_system.run((), world);
            initialized_system.apply_buffers(world);
        }
    }

    /// Runs the supplied system on the [`World`] a single time, without flushing [`Commands`](crate::system::Commands)
    #[inline]
    pub fn run_system_without_flushing<Params, S: IntoSystem<(), (), Params> + 'static>(
        &mut self,
        world: &mut World,
        system: S,
    ) {
        // If the system is already registered and initialized, use it
        if let Some(initialized_system) = self.systems.get_mut(&system.type_id()) {
            initialized_system.run((), world);
        // Otherwise, register and initialize it first.
        } else {
            let initialized_system = self.register(world, system);
            initialized_system.run((), world);
        }
    }
}

// Internals
impl SystemRegistry {
    #[inline]
    fn register<Params, S: IntoSystem<(), (), Params> + 'static>(
        &mut self,
        world: &mut World,
        system: S,
    ) -> &mut UnchainedSystem {
        let label = system.type_id();

        let mut unchained_system: UnchainedSystem = Box::new(IntoSystem::into_system(system));
        unchained_system.initialize(world);

        self.systems.insert(label, unchained_system);
        self.systems.get_mut(&label).unwrap()
    }
}

impl World {
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
            registry.run_system(world, system);
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
            registry.run_system_without_flushing(world, system);
        });
    }
}

/// The [`Command`] type for [`SystemRegistry`] [`Commands`]
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
    /// Creates a new [`Command`] struct, which can be addeded to [`Commands`]
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
