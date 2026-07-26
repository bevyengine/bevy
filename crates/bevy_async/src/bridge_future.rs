use crate::bridge_request::BridgeRequest;
use crate::plugin::AsyncWorld;
use crate::system_state::{ErasedSystemStateCell, SystemStateCell};
use crate::wake_signal::WakeSignaler;
use crate::{bridge_request, wake_signal};
use bevy_ecs::schedule::{InternedSystemSet, IntoSystemSet, SystemSet};
use bevy_ecs::system::{SystemParam, SystemParamItem};
use bevy_platform::sync::Arc;
use core::marker::PhantomData;

/// A `FnOnce` that can be run as a bridge system in order to allow for our bridge function to
/// get type inference off the closure.
pub trait AsyncSystemParamFunction<Marker> {
    type Out;
    type Param: SystemParam + 'static;
    fn run(self, param_value: SystemParamItem<Self::Param>) -> Self::Out;
}

impl<Out, Func, F0: SystemParam + 'static> AsyncSystemParamFunction<fn(F0) -> Out> for Func
where
    Func: FnOnce(F0) -> Out + FnOnce(SystemParamItem<F0>) -> Out,
    Out: 'static,
{
    type Out = Out;
    type Param = F0;

    #[inline]
    fn run(self, param_value: SystemParamItem<Self::Param>) -> Self::Out {
        fn call_inner<Out, F0>(f: impl FnOnce(F0) -> Out, f0: F0) -> Out {
            f(f0)
        }
        call_inner(self, param_value)
    }
}

macro_rules! impl_system_param_function {
    ($($F:ident),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func, $($F: SystemParam + 'static),*> AsyncSystemParamFunction<fn($($F),*) -> Out> for Func
        where
            Func: FnOnce($($F),*) -> Out + FnOnce($(SystemParamItem<$F>),*) -> Out,
            Out: 'static,
        {
            type Out = Out;
            type Param = ($($F,)*);

            #[inline]
            fn run(self, param_value: SystemParamItem<Self::Param>) -> Self::Out {
                #[allow(non_snake_case)]
                fn call_inner<Out, $($F),*>(f: impl FnOnce($($F),*) -> Out, $($F: $F),*) -> Out {
                    f($($F),*)
                }
                let ($($F,)*) = param_value;
                call_inner(self, $($F),*)
            }
        }
    };
}

impl_system_param_function!(F0, F1);
impl_system_param_function!(F0, F1, F2);
impl_system_param_function!(F0, F1, F2, F3);
impl_system_param_function!(F0, F1, F2, F3, F4);
impl_system_param_function!(F0, F1, F2, F3, F4, F5);
impl_system_param_function!(F0, F1, F2, F3, F4, F5, F6);
impl_system_param_function!(F0, F1, F2, F3, F4, F5, F6, F7);
impl_system_param_function!(F0, F1, F2, F3, F4, F5, F6, F7, F8);
impl_system_param_function!(F0, F1, F2, F3, F4, F5, F6, F7, F8, F9);
impl_system_param_function!(F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10);
impl_system_param_function!(F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11);
impl_system_param_function!(F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12);
impl_system_param_function!(F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13);
impl_system_param_function!(F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14);
impl_system_param_function!(F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15);

/// Handle that lets an async task request temporary access to an ECS
/// `SystemParam` or a tuple of them.
///
/// `P` is the typed system parameter the caller eventually wants, such as:
/// - [`bevy_ecs::prelude::Commands`]
/// - [`bevy_ecs::prelude::Res`]
/// - [`bevy_ecs::prelude::Query`]
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
    /// Create a new system state from an [`AsyncWorld`] matching the API surface of [`SystemState`]
    /// with [`World`].
    ///
    /// [`SystemState`]: bevy_ecs::system::SystemState
    /// [`World`]: bevy_ecs::world::World
    pub(crate) fn new(world: AsyncWorld) -> Self {
        Self {
            _p: PhantomData,
            world,
            #[cfg(feature = "std")]
            system_state: Arc::new(SystemStateCell::<P>::default()),
            #[cfg(not(feature = "std"))]
            system_state: Arc::from(
                bevy_platform::prelude::Box::new(SystemStateCell::<P>::default())
                    as bevy_platform::prelude::Box<dyn ErasedSystemStateCell>,
            ),
        }
    }

    /// This function allows us to create a bridge between the async task we are in and the ecs
    /// world we want access to, effectively running a system from an async task. The systems run
    /// here are able to take in `&` and `&mut` variables from the surrounding context unlike
    /// standard Bevy systems.
    ///
    /// We bridge *at* the `_sync_point` `SyncPoint` with our `bridge_fn`.
    ///
    pub fn bridge<Marker, BridgeFn, SyncPoint: 'static>(
        &self,
        _sync_point: SyncPoint,
        bridge_fn: BridgeFn,
    ) -> BridgeFuture<BridgeFn, Marker>
    where
        Marker: 'static,
        BridgeFn: AsyncSystemParamFunction<Marker, Param = P>,
    {
        // This function returns the concrete [`BridgeFuture`] rather than being an `async fn` so that the
        // future's `Send`-ness is structural, which keeps multi-parameter closures usable inside
        // `Send` tasks (an `async fn`'s opaque future trips rust's higher-ranked lifetime checks
        // there).
        BridgeFuture {
            _p: PhantomData,
            system_set: bridge_request::async_world_sync_point::<SyncPoint>
                .into_system_set()
                .intern(),
            bridge_fn: Some(bridge_fn),
            wake_signal: None,
            system_state: self.system_state.clone(),
            world: self.world.clone(),
            queued: false,
        }
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
pub struct BridgeFuture<Func, Marker> {
    _p: PhantomData<fn() -> Marker>,
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
    /// Whether the bridge request has already been queued.
    queued: bool,
}

impl<Func, Marker> Unpin for BridgeFuture<Func, Marker> {}

impl<Func, Marker> Future for BridgeFuture<Func, Marker>
where
    Marker: 'static,
    Func: AsyncSystemParamFunction<Marker>,
{
    type Output = Result<Func::Out, BridgeError>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        use core::task::Poll;

        // Try to gain a strong reference to the bridge. If this fails, the world is gone,
        // so further access is impossible.
        let Some(strong_world) = self.world.0.upgrade() else {
            // Make sure we handle the wake signal if we got one.
            let _ = self.wake_signal.take();
            return Poll::Ready(Err(BridgeError::WorldDropped));
        };

        if !self.queued {
            debug_assert!(self.wake_signal.is_none());
            self.queued = true;
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
            strong_world
                .bridge_requests
                .try_send(
                    &self.system_set,
                    BridgeRequest {
                        waker: cx.waker().clone(),
                        wake_waiter,
                    },
                )
                .ok()
                .unwrap();
            Poll::Pending
        } else {
            // If we were previously woken by the sync-point driver, we will have a
            // `WakeSignaler` stored here.
            //
            // Dropping that signal at the end of this poll acts as the
            // acknowledgement that yes, this wake was observed and this task has
            // attempted its run, you may release the waiting on the other side.
            let _drop_at_end_of_scope = self
                .wake_signal
                .take()
                .expect("future is only polled once, and we were woken after queuing");

            strong_world
                .world_scope
                .try_with(|world| {
                    let Self {
                        ref system_state,
                        ref mut bridge_fn,
                        ..
                    } = *self;
                    // Lock the system state. The unwrap is safe since we only try_lock when we have
                    // exclusive world access, so the lock must not be contested.
                    let mut system_state = system_state.try_lock::<Func::Param>(world).expect(
                        "Lock should never be contended since we have exclusive world access",
                    );

                    let param = match system_state.get_mut(world) {
                        Ok(param) => param,
                        Err(system_param_validation_error) => {
                            return Poll::Ready(Err(BridgeError::SystemParamValidation(
                                system_param_validation_error,
                            )));
                        }
                    };
                    // We finally have `P::Item<'w, 's>`, yay!, so consume the stored `FnOnce`, run it,
                    // and complete the future.
                    let out = bridge_fn.take().unwrap().run(param);
                    // Apply any deferred state (e.g. `Commands`) back into the world.
                    system_state.apply(world);
                    Poll::Ready(Ok(out))
                })
                .ok()
                .expect("we have world access since we queued and were then woken")
        }
    }
}
