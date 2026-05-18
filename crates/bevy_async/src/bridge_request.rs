use crate::plugin::{AsyncTickBudget, StrongAsyncWorld};
use crate::system_state::ErasedSystemStateCell;
use bevy_ecs::prelude::{IntoSystemSet, SystemSet, World};
use bevy_ecs::schedule::InternedSystemSet;
use bevy_platform::sync::Arc;

/// Drives the queued bridge work for `SyncPoint`.
///
/// Every queued bridge request is guaranteed to be *woken*. That wake guarantees the corresponding
/// async future gets a chance to poll.
/// It does *not* however guarantee the poll will finish its ECS work, because that
/// poll may still fail to finish it's work for a *variety* of reasons, i.e. it is unable to acquire
/// the typed `SystemState` lock and returns `Poll::Pending`.
///
/// For [`bevy_tasks::TaskPool::spawn_local`] we *are* actually guaranteed that the poll will finish
/// it's ECS work, because it's single threaded, so you can use `spawn_local` if you want
/// determinism.
///
/// This function attempts to tick queued work several times, up to `MaxAsyncTicksPerSyncPoint`.
/// If one internal tick finds no work, we opportunistically tick the local global task pool and
/// try once more before returning early.
///
/// We tick queued work multiple times for two reasons. The first is that serial `.await` calls
/// should try to all be completed within the same `SyncPoint` such as
/// ```rust,ignore
/// let health = task_1.run(|health: Single<&Health, With<Player>>| {
///     health.0
/// }).await;
/// if health == 0 {
///     return;
/// }
/// task_1.run(|commands: Commands| {
///     commands.trigger(PlayerDoesAttack);
/// }).await;
/// ```
/// The second reason is spoken of prior. Poll may fail to finish for a variety of reasons and
/// should be given several chances before giving up.
pub fn async_world_sync_point<SyncPoint: 'static>(world: &mut World) {
    // Derive the stable interned system-set key used to look up requests queued
    // for this exact sync point type.
    let sync_point = async_world_sync_point::<SyncPoint>
        .into_system_set()
        .intern();
    let async_world = world.get_resource::<StrongAsyncWorld>().unwrap().clone();
    // Read the configured maximum number of internal attempts we are willing to
    // perform during this `SyncPoint`.
    let max_ticks = world.get_resource::<AsyncTickBudget>().unwrap().0;
    for _ in 0..max_ticks {
        // Drive once. If no work was found, we may truly be done.
        // but we should give external task pools one more opportunity to make newly-woken
        // tasks runnable.
        if async_world.0.tick_sync_point(sync_point, world) == TickResult::NoWork {
            #[cfg(feature = "bevy_tasks")]
            bevy_tasks::cfg::web! {
                if {} else {
                    bevy_tasks::tick_global_task_pools_on_main_thread();
                }
            }
            // Retry once after ticking the global pool. If we are still idle,
            // there is no more immediately available progress to make.
            if async_world.0.tick_sync_point(sync_point, world) == TickResult::NoWork {
                return;
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct AsyncWorldInner {
    pub(crate) bridge_requests:
        keyed_concurrent_queue::KeyedQueues<InternedSystemSet, BridgeRequest>,
    pub(crate) world_scope: scoped_static_storage::ScopedStatic<World>,
}

impl AsyncWorldInner {
    /// This ticks a single sync point, requesting the poll of all tasks in that sync point.
    /// None of the tasks are guaranteed to actually return `Poll::Ready`, but all are guaranteed to
    /// at least do a `Poll::Pending`
    ///
    /// The flow of logic is the following:
    /// 1. We first drain the queue for our `SyncPoint`.
    /// 2. Expose our `World` through `world_scope`.
    /// 3. Wake all our `BridgeFuture`s.
    /// 4. Apply our `SystemState` back into the `World`. (Things like `Commands`).
    fn tick_sync_point(&self, sync_point: InternedSystemSet, world: &mut World) -> TickResult {
        let mut queued_requests = bevy_platform::prelude::vec![];
        while let Ok(queued_task_bridge) = self.bridge_requests.get_or_create(&sync_point).pop() {
            queued_requests.push(queued_task_bridge);
        }
        // If no requests were waiting then report idle so the caller can decide whether to stop
        // or attempt one more task-pool tick.
        if queued_requests.is_empty() {
            return TickResult::NoWork;
        }
        // Make this `World` temporarily visible to our waking futures. Wake them all and wait
        // until they all have at least *attempted* to poll.
        // This is contractually obligated by the contract of `.wake()`. We are guaranteed one wake
        // per call to our `.wake()`.
        let completed_tasks = self
            .world_scope
            .scope(world, || wake_requests_and_wait(queued_requests));
        for task in completed_tasks {
            task.apply(world);
        }
        TickResult::DidWork
    }
}

/// We need to notify all our Wakers that have queued that we've dropped so they can error
impl Drop for AsyncWorldInner {
    fn drop(&mut self) {
        for bridge_requests in self.bridge_requests.inner().read().unwrap().values() {
            while let Ok(request) = bridge_requests.pop() {
                request.waker.wake();
            }
        }
    }
}

/// Whether a tick attempt made any progress.
#[derive(PartialEq)]
enum TickResult {
    /// We found and processed at least one queued bridge request.
    DidWork,
    /// There was no queued work available for the `SyncPoint`.
    NoWork,
}

/// A queued access request bridging an async task into ECS.
pub(crate) struct BridgeRequest {
    /// Waker for the async future (`crate::bridge_future::BridgeFuture`) that wants ECS access.
    /// When the `SyncPoint` is driven, this waker is fired so the future can
    /// poll while `world_scope` exposes the current `World`.
    pub(crate) waker: core::task::Waker,
    /// Our custom primitive that lets us wait until all the futures have tried to run before
    /// continuing.
    pub(crate) wake_waiter: crate::wake_signal::WakeWaiter,
    pub(crate) system_state: Arc<dyn ErasedSystemStateCell>,
}

/// A queued bridge request whose waker has already been fired.
struct WokenBridgeRequest {
    wake_signal: crate::wake_signal::WakeWaiter,
    system_state: Arc<dyn ErasedSystemStateCell>,
}

/// A request that has finished its attempted poll and may need to apply deferred world state.
struct CompletedBridgeRequest {
    system_state: Arc<dyn ErasedSystemStateCell>,
}

impl CompletedBridgeRequest {
    #[inline]
    fn apply(self, world: &mut World) {
        self.system_state.apply(world);
    }
}

#[inline]
fn wake_requests_and_wait(
    queued_requests: bevy_platform::prelude::Vec<BridgeRequest>,
) -> bevy_platform::prelude::Vec<CompletedBridgeRequest> {
    let bridged_futures = queued_requests
        .into_iter()
        .map(
            |BridgeRequest {
                 system_state,
                 waker,
                 wake_waiter: wake_signal,
                 ..
             }| {
                // Trigger the `BridgeFuture` so it can poll while `world_scope`
                // is active.
                waker.wake();
                WokenBridgeRequest {
                    system_state,
                    wake_signal,
                }
            },
        )
        // we re-collect to ensure we fully exhaust the prior iterator
        // we want to have all the wakers call .wake() before the first barrier calls .wait()
        .collect::<bevy_platform::prelude::Vec<_>>();

    #[cfg(feature = "bevy_tasks")]
    bevy_tasks::cfg::web! {
        if {} else {
            bevy_tasks::tick_global_task_pools_on_main_thread();
        }
    }

    bridged_futures
        .into_iter()
        .map(
            |WokenBridgeRequest {
                 system_state,
                 wake_signal,
             }| {
                wake_signal.wait();
                CompletedBridgeRequest { system_state }
            },
        )
        .collect()
}
