use bevy_ecs::system::{SystemParam, SystemState};
use bevy_ecs::world::World;
use bevy_platform::sync::{Mutex, MutexGuard, OnceLock};

/// Stores a typed `SystemState<P>` behind a `OnceLock<Mutex>` so it can be initialized once
/// and then mutably shared across bridge requests.
///
/// Why this exists:
/// `SystemState<P>` is typed, but the bridge queue needs to store heterogeneous
/// requests without knowing `P` at compile time. So each concrete
/// `SystemStateCell<P>` is later erased behind `dyn ErasedSystemStateCell`.
///
/// We use a `OnceLock` because we cannot construct the `SystemState<P>` until we have a mutable
/// `World`. So we initialize it `SystemStateCell<P>` the first time it is used.
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
/// This lets the bridge store all request state uniformly as `Arc<dyn ErasedSystemStateCell>`.
///
/// The operations on the typed `SystemState` (initialization, access, and applying deferred
/// state) all happen through the `impl dyn` implementation below, inside the bridge future
/// itself.
pub(crate) trait ErasedSystemStateCell: Send + Sync + core::any::Any + 'static {}

impl<P: SystemParam + 'static> ErasedSystemStateCell for SystemStateCell<P> {}

impl dyn ErasedSystemStateCell {
    /// This function initializes the [`SystemStateCell`] if it hasn't already been initialized, and
    /// then returns the [`MutexGuard`] of the `SystemState` if it isn't being used by another thread.
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
