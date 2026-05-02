use alloc::vec::Vec;
use core::{any::TypeId, fmt};

use bevy_platform::sync::{Arc, Mutex, MutexGuard};
use bevy_utils::DebugName;

use crate::{
    change_detection::{CheckChangeTicks, Tick},
    prelude::World,
    query::FilteredAccessSet,
    schedule::InternedSystemSet,
    system::{IntoSystem, System, SystemInput},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld},
};

use super::{RunSystemError, SystemIn, SystemStateFlags};

/// A type alias for a [`SystemArc<dyn System>`].
pub type SystemArcDyn<I, O> = SystemArc<dyn System<In = I, Out = O>>;

/// A shareable, mutable reference to a [`System`] protected by a [`Mutex`] and
/// reference-counted by an [`Arc`].
pub struct SystemArc<S: System + ?Sized> {
    system: Arc<Mutex<SystemArcInner<S>>>,
}

impl<S: System> SystemArc<S> {
    /// Creates a new [`SystemArc`] by converting the given `system` into a
    /// [`System`] and wrapping it in an [`Arc`] and [`Mutex`].
    pub fn new<M>(system: impl IntoSystem<S::In, S::Out, M, System = S>) -> Self {
        Self {
            system: Arc::new(Mutex::new(SystemArcInner {
                initialized: false,
                system: IntoSystem::into_system(system),
            })),
        }
    }

    /// Erases the concrete type of the system, returning a [`SystemArc<dyn System>`].
    /// Useful for storing systems of different types in a homogeneous collection.
    pub fn erase(self) -> SystemArcDyn<S::In, S::Out> {
        SystemArc {
            system: self.system as Arc<Mutex<SystemArcInner<dyn System<In = S::In, Out = S::Out>>>>,
        }
    }
}

impl<S: System + ?Sized> SystemArc<S> {
    /// Locks the system for mutable access, returning a [`MutexGuard`] to the
    /// inner [`SystemArcInner`].
    pub fn lock(&self) -> MutexGuard<'_, SystemArcInner<S>> {
        self.system.lock().unwrap()
    }
}

impl<I: SystemInput + 'static, O: 'static> SystemArc<dyn System<In = I, Out = O>> {
    /// Creates a new [`SystemArc`] by converting the given `system` into a
    /// [`System`], wrapping it in an [`Arc`] and [`Mutex`], and erasing its concrete type.
    pub fn new_dyn<M>(system: impl IntoSystem<I, O, M>) -> Self {
        Self {
            system: Arc::new(Mutex::new(SystemArcInner {
                initialized: false,
                system: IntoSystem::into_system(system),
            })) as Arc<Mutex<SystemArcInner<dyn System<In = I, Out = O>>>>,
        }
    }
}

impl<S: System + ?Sized> Clone for SystemArc<S> {
    fn clone(&self) -> Self {
        Self {
            system: self.system.clone(),
        }
    }
}

impl<S: System + ?Sized> fmt::Debug for SystemArc<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let system = self.lock();
        f.debug_struct("SystemArc")
            .field("name", &system.name())
            .field("is_exclusive", &system.is_exclusive())
            .field("is_send", &system.is_send())
            .finish_non_exhaustive()
    }
}

impl<S: System> From<S> for SystemArc<S> {
    fn from(system: S) -> Self {
        Self::new(system)
    }
}

impl<S: System> From<SystemArc<S>> for SystemArcDyn<S::In, S::Out> {
    fn from(system_arc: SystemArc<S>) -> Self {
        system_arc.erase()
    }
}

/// The inner data of a [`SystemArc`], containing the actual system and a flag
/// indicating whether the system has been initialized.
pub struct SystemArcInner<S: System + ?Sized> {
    initialized: bool,
    system: S,
}

impl<S: System + ?Sized> SystemArcInner<S> {
    /// Returns `true` if the system has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Initializes the system and sets the initialized flag, if it has not
    /// already been initialized. Does nothing if the system is already initialized.
    pub fn ensure_initialized(&mut self, world: &mut World) {
        if !self.initialized {
            self.system.initialize(world);
            self.initialized = true;
        }
    }
}

impl<S: System + ?Sized> System for SystemArcInner<S> {
    type In = S::In;
    type Out = S::Out;

    fn name(&self) -> DebugName {
        self.system.name()
    }

    fn system_type(&self) -> TypeId {
        TypeId::of::<S>()
    }

    fn flags(&self) -> SystemStateFlags {
        self.system.flags()
    }

    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        // SAFETY: Upheld by caller
        unsafe { self.system.run_unsafe(input, world) }
    }

    #[cfg(feature = "hotpatching")]
    fn refresh_hotpatch(&mut self) {
        self.system.refresh_hotpatch();
    }

    fn apply_deferred(&mut self, world: &mut World) {
        self.system.apply_deferred(world);
    }

    fn queue_deferred(&mut self, world: DeferredWorld) {
        self.system.queue_deferred(world);
    }

    fn initialize(&mut self, world: &mut World) -> FilteredAccessSet {
        let access = self.system.initialize(world);
        self.initialized = true;
        access
    }

    fn check_change_tick(&mut self, check: CheckChangeTicks) {
        self.system.check_change_tick(check);
    }

    fn default_system_sets(&self) -> Vec<InternedSystemSet> {
        self.system.default_system_sets()
    }

    fn get_last_run(&self) -> Tick {
        self.system.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.system.set_last_run(last_run);
    }
}
