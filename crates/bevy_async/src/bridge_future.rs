use crate::bridge_request::BridgeRequest;
use crate::plugin::AsyncWorld;
use crate::system_state::{ErasedSystemStateCell, SystemStateCell};
use crate::wake_signal::WakeSignaler;
use crate::{bridge_request, wake_signal};
use bevy_ecs::schedule::{InternedSystemSet, IntoSystemSet, SystemSet};
use bevy_ecs::system::SystemParam;
use bevy_platform::sync::Arc;
use core::marker::PhantomData;

/// Handle that lets an async task request temporary access to an ECS
/// `SystemParam` or a tuple of them.
///
/// `P` is the typed system parameter the caller eventually wants, such as:
/// - [`bevy_ecs::prelude::Commands`]
/// - [`bevy_ecs::prelude::Res`]
/// - [`bevy_ecs::prelude::Query`]
/// - tuples of params
///
/// It is cheap to clone and intended to be passed into async tasks.
/// You can pass it into *multiple* tasks on separate threads and have them work concurrently
/// off of the same state, sharing `Locals`.
pub struct AsyncSystemState<P: SystemParam + 'static> {
    pub(crate) _p: PhantomData<P>,

    /// A `Weak` is used so tasks do not stay alive if the world is dropped.
    /// If the world goes away, upgrading this weak pointer fails and access
    /// returns [`BridgeError::WorldDropped`].
    pub(crate) world: AsyncWorld,

    /// Type-erased storage for the underlying `SystemState<P>`.
    ///
    /// Each `EcsAccess<P>` keeps reusing the same typed system state across
    /// accesses so repeated operations do not rebuild it from scratch.
    ///
    /// This is also important not only to persist params like `Local` but *also* so `Changed` and
    /// `Added` and other filters can work.
    pub(crate) system_state: Arc<dyn ErasedSystemStateCell>,
}

impl<P: SystemParam + 'static> Clone for AsyncSystemState<P> {
    fn clone(&self) -> Self {
        Self {
            _p: PhantomData,
            world: self.world.clone(),
            system_state: self.system_state.clone(),
        }
    }
}

impl<P: SystemParam + 'static> AsyncSystemState<P> {
    /// Create a new `AsyncSystemState` from an `AsyncWorld` matching the Api surface of
    /// `SystemState` with `World`.
    pub fn new(world: AsyncWorld) -> Self {
        Self {
            _p: PhantomData,
            world,
            system_state: Arc::new(SystemStateCell::<P>::default()),
        }
    }

    /// This function allows us to create a bridge between the async task we are in and the ecs
    /// world we want access to, effectively running a system from an async task. The systems run
    /// here are able to take in `&` and `&mut` variables from the surrounding context unlike
    /// standard Bevy systems.
    ///
    /// We bridge *at* the `_sync_point` `SyncPoint` with our `bridge_fn`.
    pub async fn bridge<BridgeFn, Out, SyncPoint: 'static>(
        &self,
        _sync_point: SyncPoint,
        bridge_fn: BridgeFn,
    ) -> Result<Out, BridgeError>
    where
        for<'w, 's> BridgeFn: FnOnce(P::Item<'w, 's>) -> Out,
    {
        BridgeFuture {
            _p: PhantomData,
            system_set: bridge_request::async_world_sync_point::<SyncPoint>
                .into_system_set()
                .intern(),
            bridge_fn: Some(bridge_fn),
            wake_signal: None,
            system_state: self.system_state.clone(),
            world: self.world.clone(),
        }
        .await
    }
}

/// If the bridge cannot run, either because the system params were invalid, or because the world it
/// was referencing no longer exists, we return this error.
#[derive(thiserror::Error, Debug)]
pub enum BridgeError {
    /// The requested `SystemParam` was invalid in the current world context.
    /// for example trying to access a param that fails Bevy's usual validation like a missing
    /// Resource or using `Single` on something that has 0 or multiple instances.
    #[error(transparent)]
    SystemParamValidation(bevy_ecs::system::SystemParamValidationError),
    /// The world has been dropped, so we should just return.
    #[error("World no longer exists")]
    WorldDropped,
}

/// Future representing a single in-flight bridging request between our async task and our `World`.
struct BridgeFuture<P: SystemParam + 'static, Func, Out> {
    _p: PhantomData<(P, Func, Out)>,
    /// Interned system-set key identifying which sync-point queue this future
    /// should be sent to.
    system_set: InternedSystemSet,
    /// This is the pseudo-system that we try to run when we have access to `World`.
    /// This is an option just so we can take it out when we run it so we can use `FnOnce`
    /// instead of `FnMut`, so it's more flexible than true systems.
    bridge_fn: Option<Func>,
    /// Wake signal for the currently queued wake cycle, if any.
    ///
    /// The future drops this at the end of `poll` which acts as acknowledgement that the wake
    /// has been handled.
    wake_signal: Option<WakeSignaler>,
    system_state: Arc<dyn ErasedSystemStateCell>,
    /// Weak bridge pointer so the loss of the world becomes a clean runtime error.
    world: AsyncWorld,
}

impl<P: SystemParam + 'static, Func, Out> Unpin for BridgeFuture<P, Func, Out> {}

impl<P, Func, Out> Future for BridgeFuture<P, Func, Out>
where
    P: SystemParam + 'static,
    for<'w, 's> Func: FnOnce(P::Item<'w, 's>) -> Out,
{
    type Output = Result<Out, BridgeError>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        use core::task::Poll;

        // If we were previously woken by the sync-point driver, we will have a
        // `WakeSignaler` stored here.
        //
        // Dropping that signal at the end of this poll acts as the
        // acknowledgement that yes, this wake was observed and this task has
        // attempted its run, you may release the waiting on the other side.
        let _drop_at_end_of_scope = self.wake_signal.take();

        // Try to gain a strong reference to the bridge. If this fails, the world is gone,
        // so further access is impossible.
        let Some(strong_world) = self.world.0.upgrade() else {
            return Poll::Ready(Err(BridgeError::WorldDropped));
        };
        match strong_world
            .world_scope
            .try_with(|world| {
                let Self {
                    ref system_state,
                    ref mut bridge_fn,
                    ..
                } = *self;
                // Attempt to acquire the typed `SystemState<P>`.
                //
                // We deliberately use `try_lock` rather than blocking. If
                // another bridge request is currently using the same system
                // state, we simply yield and let the sync-point driver try again
                // on a later internal tick.
                let Some(mut system_state) = system_state.try_lock::<P>(world) else {
                    return Poll::Pending;
                };

                if !system_state.meta().is_send() {
                    return Poll::Ready(Err(BridgeError::SystemParamValidation(
                        bevy_ecs::system::SystemParamValidationError::invalid::<
                            bevy_ecs::prelude::NonSend<()>,
                        >("Cannot have your system be non-send / exclusive"),
                    )));
                }

                let param = match system_state.get_mut(world) {
                    Ok(param) => param,
                    Err(system_param_validation_error) => {
                        return Poll::Ready(Err(BridgeError::SystemParamValidation(
                            system_param_validation_error,
                        )))
                    }
                };
                // We finally have `P::Item<'w, 's>`, yay!, so consume the stored `FnOnce`, run it,
                // and complete the future.
                Poll::Ready(Ok(bridge_fn.take().unwrap()(param)))
            })
            .ok()
        {
            Some(out) => out,
            None => {
                // No world is currently exposed. That means we are being polled
                // outside the `async_world_sync_point`, so we cannot access ECS yet.
                //
                // Instead, enqueue ourselves to be revisited when the matching
                // sync-point system runs.
                let (wake_signal, wake_waiter) = wake_signal::pair();
                // Store the wake_signal locally so dropping it at the end of the next
                // poll acknowledges the wake.
                self.wake_signal.replace(wake_signal);
                // Queue the request under this future's target sync point.
                //
                // The queued payload carries the following!
                // 1. The task's waker, so the sync-point driver can wake it.
                // 2. The wake handshake signal, so the driver can wait until the wake has actually
                // been processed.
                // 3. An initialization hint for the typed `SystemState`.
                // 4. The erased `SystemState` storage itself.
                strong_world
                    .bridge_requests
                    .try_send(
                        &self.system_set,
                        BridgeRequest {
                            waker: cx.waker().clone(),
                            wake_waiter,
                            system_state: self.system_state.clone(),
                        },
                    )
                    .ok()
                    .unwrap();
                Poll::Pending
            }
        }
    }
}
