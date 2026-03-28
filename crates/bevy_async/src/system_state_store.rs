use bevy_ecs::system::{SystemParam, SystemState};
use bevy_ecs::world::World;
use bevy_platform::sync::{Mutex, MutexGuard, OnceLock};

/// Stores a typed `SystemState<P>` behind a mutex so it can be initialized once
/// and then shared across bridge requests.
///
/// Why this exists:
/// `SystemState<P>` is typed, but the bridge queue needs to store heterogeneous
/// requests without knowing `P` at compile time. So each concrete
/// `SystemStateStore<P>` is later erased behind `dyn ErasedSystemStateStore`.
///
/// The inner `Option` starts as `None` because we cannot construct the
/// `SystemState<P>` until we have a mutable `World`. Furthermore, it is not safe to try to
/// initialize the `SystemState<P>` from a thread *other* than the world-owning thread, so
/// we have to start it as none and have the initialization occur on the world-owning thread before
/// the `SystemState<P>` is ever used.
pub(crate) struct SystemStateCell<P: SystemParam + 'static>(OnceLock<Mutex<SystemState<P>>>);

impl<P: SystemParam + 'static> Default for SystemStateCell<P> {
    fn default() -> Self {
        // Start uninitialized. Initialization is deferred until the request is
        // first driven on the world-owning thread with access to `&mut World`.
        Self(OnceLock::default())
    }
}

/// Allows us to erase the `SystemStateCell` so we can pass it to and from the ecs.
///
/// This lets the bridge store all request state uniformly as `Arc<dyn ErasedSystemStateStore>`.
///
/// This trait exposes the following operations:
/// - initialize the typed `SystemState` if needed,
/// - apply deferred state back into the world,
/// - ask whether initialization has already happened.
pub(crate) trait ErasedSystemStateCell: Send + Sync + core::any::Any + 'static {
    /// Apply deferred operations accumulated by the `SystemState` back into
    /// the world.
    ///
    /// For example, `Commands` buffers are typically flushed during `apply`.
    fn apply(&self, world: &mut World);
}

impl<P: SystemParam> ErasedSystemStateCell for SystemStateCell<P> {
    fn apply(&self, world: &mut World) {
        // We expect initialization to have already occurred before `apply` is
        // ever called. So `unwrap()` here reflects an invariant of the bridge.
        // Completed requests only exist for initialized system states.
        self.0.get().unwrap().lock().unwrap().apply(world);
    }
}

impl dyn ErasedSystemStateCell {
    pub(crate) fn try_lock<'w, 'a, P: SystemParam + 'static>(
        &'a self,
        world: &'w mut World,
    ) -> Option<MutexGuard<'a, SystemState<P>>>
    where
        'a: 'w,
    {
        (self as &dyn core::any::Any)
            .downcast_ref::<SystemStateCell<P>>()
            // Caller must use the same `Params` that created this cell.
            .unwrap()
            .0
            .get_or_init(|| Mutex::new(SystemState::new(world)))
            // Use `try_lock` rather than blocking:
            // if another request currently owns the typed `SystemState<P>`, the
            // caller should yield with `Poll::Pending` instead of stalling a
            // thread. We get ticked optimistically many times so it's okay. We aren't guaranteed to
            // run everytime so we can return Poll::Pending instead of blocking an async task
            // which would be very bad.
            .try_lock()
            .ok()
    }
}
