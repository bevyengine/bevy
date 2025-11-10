use crate::schedule::async_ecs::keyed_queues::KeyedQueues;
use crate::schedule::{InternedScheduleLabel, ScheduleLabel};
use crate::system::SystemParamValidationError;
use crate::world::unsafe_world_cell::UnsafeWorldCell;
use crate::world::FromWorld;
use crate::{
    system::{SystemParam, SystemState},
    world::World,
};
use bevy_ecs::error::ErrorContext;
use bevy_ecs::prelude::NonSend;
use bevy_ecs::world::{Mut, WorldId};
use bevy_platform::collections::HashMap;
use bevy_platform::sync::{Arc, Mutex, OnceLock, RwLock};
use concurrent_queue::ConcurrentQueue;
use core::any::TypeId;
use core::marker::PhantomData;
use core::pin::Pin;
use core::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use core::task::{Context, Poll, Waker};
use std::thread;

/// Keyed queues is a combination of a hashmap and a concurrent queue which is useful because it
/// allows for non-blocking keyed queues.
/// We want every World's async machinery to be as independent as possible, and this allows us
/// to key our Queues on `(WorldId, Schedule)` so that there is 0 contention on the fast path and
/// arbitrary N number of worlds running in parallel on the same process do not interfere at all
/// except the very first time a new world initializes it's key.
mod keyed_queues {
    use bevy_platform::collections::HashMap;
    use bevy_platform::sync::{Arc, RwLock};
    use concurrent_queue::ConcurrentQueue;
    use core::hash::Hash;
    /// `HashMap<K, Arc<ConcurrentQueue<V>>>` behind a single `RwLock`.
    /// - Writers only contend when creating a new key.
    /// - `push` is almost always non-blocking (unbounded queue).
    pub struct KeyedQueues<K, V> {
        inner: RwLock<HashMap<K, Arc<ConcurrentQueue<V>>>>,
    }

    impl<K, V> KeyedQueues<K, V>
    where
        K: Eq + Hash + Clone,
        V: Send + 'static,
    {
        pub fn new() -> Self {
            Self {
                inner: RwLock::new(HashMap::new()),
            }
        }

        #[inline]
        pub fn get_or_create(&self, key: &K) -> Arc<ConcurrentQueue<V>> {
            // Fast path: try read lock first
            if let Some(q) = self.inner.read().unwrap().get(key).cloned() {
                return q;
            }
            // Slow path: create under write lock if still absent
            let mut write = self.inner.write().unwrap();
            // We intentionally check a second time because of synchronization
            if let Some(q) = write.get(key).cloned() {
                return q;
            }
            let q = Arc::new(ConcurrentQueue::unbounded());
            write.insert(key.clone(), q.clone());
            q
        }

        /// Potentially-blocking send but almost never blocking (unbounded queue => `push` never fails).
        /// ( Only blocks when the `(WorldId, Schedule)` has never been used before
        #[inline]
        pub fn try_send(&self, key: &K, val: V) -> Result<(), concurrent_queue::PushError<V>> {
            let q = self.get_or_create(key);
            q.push(val)
        }
    }
}

/// This is an abstraction that temporarily and soundly stores the `UnsafeWorldCell` in a static so we can access
/// it from any async task, runtime, and thread.
static GLOBAL_WORLD_ACCESS: WorldAccessRegistry = WorldAccessRegistry(OnceLock::new());

/// The entrypoint, stores `Waker`s from `async_access`'s that wish to be polled with world access
/// also stores the generic function pointer to the concrete function that initializes the
/// system state for any set of `SystemParams`
pub(crate) static GLOBAL_WAKE_REGISTRY: WakeRegistry = WakeRegistry(OnceLock::new());

/// Acts as a barrier that is waited on in the `wait` call, and once the `AtomicI64` reaches 0 the
/// thread that `wait` was called on gets woken up and resumes.
#[derive(bevy_ecs_macros::Resource, Clone)]
pub(crate) struct WakeParkBarrier(thread::Thread, Arc<AtomicI64>);

/// Stores the previous system state per task id which allows `Local`, `Changed` and other filters
/// that depend on persistent state to work.
#[derive(bevy_ecs_macros::Resource)]
pub(crate) struct SystemStatePool<T: SystemParam + 'static>(
    RwLock<HashMap<AsyncTaskId, ConcurrentQueue<SystemState<T>>>>,
);

/// Function pointer to a concrete version of a genericized system state being applied to the world.
#[derive(bevy_ecs_macros::Resource, Default)]
pub(crate) struct SystemParamAppliers(HashMap<TypeId, fn(&mut World)>);
impl SystemParamAppliers {
    fn run(&mut self, world: &mut World) {
        for closure in self.0.values_mut() {
            closure(world);
        }
    }
}
impl<T: SystemParam + 'static> FromWorld for SystemStatePool<T> {
    fn from_world(world: &mut World) -> Self {
        let this = Self(RwLock::new(HashMap::default()));
        world.init_resource::<SystemParamAppliers>();
        let mut appliers = world.get_resource_mut::<SystemParamAppliers>().unwrap();
        if !appliers.0.contains_key(&TypeId::of::<T>()) {
            appliers.0.insert(TypeId::of::<T>(), |world: &mut World| {
                world.try_resource_scope(|world, param_pool: Mut<SystemStatePool<T>>| {
                    for concurrent_queue in param_pool.0.read().unwrap().values() {
                        let Ok(mut system_state) = concurrent_queue.pop() else {
                            unreachable!()
                        };
                        system_state.apply(world);
                        match concurrent_queue.push(system_state) {
                            Ok(_) => {}
                            Err(_) => panic!(),
                        }
                    }
                });
            });
        }
        this
    }
}

/// A monotonically increasing global identifier for any particular async task.
/// Is an internal implementation detail and thus not generally accessible
#[derive(Clone, Copy, Hash, PartialOrd, PartialEq, Eq, Debug)]
struct AsyncTaskId(u64);

/// The next [`AsyncTaskId`].
static MAX_TASK_ID: AtomicU64 = AtomicU64::new(0);

impl AsyncTaskId {
    /// Create a new, unique [`AsyncTaskId`]. Returns [`None`] if the supply of unique
    /// IDs has been exhausted.
    ///
    /// Please note that the IDs created from this method are unique across
    /// time - if a given ID is [`Drop`]ped its value still cannot be reused
    pub fn new() -> Option<Self> {
        MAX_TASK_ID
            // We use `Relaxed` here since this atomic only needs to be consistent with itself
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                val.checked_add(1)
            })
            .map(AsyncTaskId)
            .ok()
    }
}

/// Is the `GLOBAL_WAKE_REGISTRY`
pub(crate) struct WakeRegistry(
    OnceLock<
        KeyedQueues<
            (WorldId, InternedScheduleLabel),
            (Waker, fn(&mut World, AsyncTaskId), AsyncTaskId),
        >,
    >,
);

impl WakeRegistry {
    /// This function finds all pending `async_access` calls for a particular `Schedule` and a particular
    /// `WorldId`. It wakes all of them, temporarily and soundly stores a `UnsafeWorldCell` in the
    /// `GLOBAL_WORLD_ACCESS` and parks until the tasks it has awoken either complete their `async_access`
    /// or have returned `Poll::Pending` for a variety of reasons.
    /// The performance implications of this call are entirely dependent on the async runtime
    /// you are using it with, certain poor implementations *could* cause this to take longer
    /// than expect to resolve.
    /// Returns `Some` as long as the last call processed any number of waiting `async_access` calls.
    pub fn wait(&self, schedule: InternedScheduleLabel, world: &mut World) -> Option<()> {
        let world_id = world.id();
        if GLOBAL_WAKE_REGISTRY
            .0
            .get_or_init(KeyedQueues::new)
            .get_or_create(&(world_id, schedule))
            .is_empty()
        {
            return None;
        }
        // Cleanups the garbage first.
        for (cleanup_function, task_to_cleanup) in TASKS_TO_CLEANUP
            .get_or_init(KeyedQueues::new)
            .get_or_create(&world_id)
            .try_iter()
        {
            cleanup_function(world, task_to_cleanup);
        }
        let mut waker_list = bevy_platform::prelude::vec![];
        while let Ok((waker, system_init, task_id)) = GLOBAL_WAKE_REGISTRY
            .0
            .get_or_init(KeyedQueues::new)
            .get_or_create(&(world_id, schedule))
            .pop()
        {
            // It's okay to call this every time, because it only *actually* inits the system if the task id is new
            system_init(world, task_id);
            waker_list.push(waker);
        }
        let waker_list_len = waker_list.len();
        let wake_park_barrier = WakeParkBarrier(
            thread::current(),
            Arc::new(AtomicI64::new(waker_list_len as i64)),
        );
        world.insert_resource(wake_park_barrier.clone());
        GLOBAL_WORLD_ACCESS.set(world, || {
            for waker in waker_list {
                waker.wake();
            }
            // We do this because we can get spurious wakes, but we wanna ensure that
            // we stay parked until we have at least given every poll a chance to happen.
            while wake_park_barrier.1.load(Ordering::SeqCst) > 0 {
                thread::park();
            }
        })?;
        // Applies all the commands stored up to the world
        world.try_resource_scope(|world, mut appliers: Mut<SystemParamAppliers>| {
            appliers.run(world);
        });
        Some(())
    }
}

/// This is a very low contention, no contention in the normal execution path, way of storing and
/// using a `UnsafeWorldCell` from any thread/async task/async runtime.
/// The `Mutex<PhantomData<>>` is used to return `Poll::Pending` early from an `async_access` if
/// another `async_access` is currently using it.
pub(crate) struct WorldAccessRegistry(
    OnceLock<
        RwLock<
            HashMap<
                WorldId,
                RwLock<
                    Option<(
                        UnsafeWorldCell<'static>,
                        Mutex<PhantomData<UnsafeWorldCell<'static>>>,
                    )>,
                >,
            >,
        >,
    >,
);

impl WorldAccessRegistry {
    /// During this `func: FnOnce()` call, calling `get` will access the stored `UnsafeWorldCell`
    fn set(&self, world: &mut World, func: impl FnOnce()) -> Option<()> {
        let this = self.0.get_or_init(|| RwLock::new(HashMap::new()));
        let world_id = world.id();
        if !this.read().unwrap().contains_key(&world_id) {
            // VERY rare only happens the first time we try to do anything async in a new World
            let _ = this.write().unwrap().insert(world_id, RwLock::new(None));
        }

        struct ClearOnDropGuard<'a> {
            slot: &'a RwLock<
                Option<(
                    UnsafeWorldCell<'static>,
                    Mutex<PhantomData<UnsafeWorldCell<'static>>>,
                )>,
            >,
        }
        impl<'a> Drop for ClearOnDropGuard<'a> {
            fn drop(&mut self) {
                // clear it on the way out
                // we can't actually panic here because panicking in a drop is bad
                match self.slot.write() {
                    Ok(mut slot) => {
                        let _ = slot.take();
                    }
                    Err(_) => {
                        // This is okay because the mutex is poisoned so nothing can access the
                        // UnsafeWorldCell now.
                    }
                }
            }
        }
        // SAFETY: This mem transmute is safe only because we drop it after, and our GLOBAL_WORLD_ACCESS is private, and we don't clone it
        // where we do use it, so the lifetime doesn't get propagated anywhere.
        // Lifetimes are not used in any actual code optimization, so turning it into a static does not violate any of rust's rules
        // As *LONG* as we keep it within it's lifetime, which we do here, manually, with our `ClearOnDrop` struct.
        unsafe {
            let binding = this.read().unwrap();
            let world_container = binding.get(&world_id).unwrap();
            // SAFETY this is required in order to make sure that even in the event of a panic, this can't get accessed
            let _clear = ClearOnDropGuard {
                slot: world_container,
            };
            // SAFETY: This mem transmute is safe only because we drop it after, and our GLOBAL_WORLD_ACCESS is private, and we don't clone it
            // where we do use it, so the lifetime doesn't get propagated anywhere.
            // Lifetimes are not used in any actual code optimization, so turning it into a static does not violate any of rust's rules
            // As *LONG* as we keep it within it's lifetime, which we do here, manually, with our `ClearOnDrop` struct.
            world_container.write().unwrap().replace((
                core::mem::transmute::<UnsafeWorldCell, UnsafeWorldCell<'static>>(
                    world.as_unsafe_world_cell(),
                ),
                Mutex::new(PhantomData),
            ));
            func();
        }
        Some(())
    }
    fn get<T>(
        &self,
        world_id: WorldId,
        func: impl FnOnce(UnsafeWorldCell) -> Poll<T>,
    ) -> Option<Poll<T>> {
        // it's okay to *not* do the RaiiThing on these early returns, because that means we aren't in a state
        // where a thread is parked because of our world.
        let a = self.0.get()?.read().unwrap();
        let b = a.get(&world_id)?.read().unwrap();
        let our_thing = b.as_ref()?;
        struct UnparkOnDropGuard(WakeParkBarrier);
        impl Drop for UnparkOnDropGuard {
            fn drop(&mut self) {
                let val = self.0 .1.fetch_sub(1, Ordering::SeqCst) - 1;
                // The runtime can poll us *more* often than when we call wake,
                // this is why we use a AtomicI64 instead
                if val == 0 {
                    self.0 .0.unpark();
                }
            }
        }
        // SAFETY: WakeParkBarrier is only *read* during this section per world, so reading it
        // without an associated mutex is okay.
        // Furthermore the WakeParkBarrier cannot be queried by `async_access` because it's type
        // is not public, `async_access` cannot access `&mut World` to do a dynamic resource
        // modification.
        let async_barrier = unsafe {
            our_thing
                .0
                .get_resource::<WakeParkBarrier>()
                .unwrap()
                .clone()
        };
        let _guard = UnparkOnDropGuard(async_barrier.clone());
        // this allows us to effectively yield as if pending if the world doesn't exist rn.
        let _world = our_thing.1.try_lock().ok()?;
        // SAFETY: this is safe because we ensure no one else has access to the world.
        Some(func(our_thing.0))
    }
}

/// Allows you to access the ECS from any arbitrary async runtime.
/// Calls will never return immediately and will always start Pending at least once.
/// Call this with the same `EcsTask` to persist `SystemParams` like `Local` or `Changed`
/// Just use `world_id` if you do not mind a new `SystemParam` being initialized every time.
pub async fn async_access<P, Func, Out>(
    task_identifier: impl Into<EcsTask<P>>,
    schedule: impl ScheduleLabel,
    ecs_access: Func,
) -> Out
where
    P: SystemParam + 'static,
    for<'w, 's> Func: FnOnce(P::Item<'w, 's>) -> Out,
{
    let task_identifier = task_identifier.into();
    PendingEcsCall::<P, Func, Out>(
        PhantomData::<P>,
        PhantomData,
        Some(ecs_access),
        (task_identifier.0 .1, schedule.intern()),
        task_identifier.0 .0,
    )
    .await
}

static TASKS_TO_CLEANUP: OnceLock<
    KeyedQueues<WorldId, (fn(&mut World, AsyncTaskId), AsyncTaskId)>,
> = OnceLock::new();

/// Pass the `EcsTask` into here after you're done using it
/// This function will mark the `SystemState` for that task for cleanup.
fn cleanup_ecs_task<P: SystemParam + 'static>(task: &InternalEcsTask<P>) {
    fn cleanup_task<P: SystemParam + 'static>(world: &mut World, task_id: AsyncTaskId) {
        world.try_resource_scope(|_world, param_pool: Mut<SystemStatePool<P>>| {
            let mut pool = param_pool.0.write().unwrap();
            pool.remove(&task_id);
            if pool.len() * 2 < pool.capacity() {
                pool.shrink_to_fit();
            }
        });
    }
    // Should never panic cause this is an unbounded queue
    match TASKS_TO_CLEANUP
        .get_or_init(KeyedQueues::new)
        .try_send(&task.1, (cleanup_task::<P>, task.0))
    {
        Ok(_) => {}
        Err(_) => unreachable!(),
    }
}

impl<P: SystemParam + 'static> From<WorldId> for EcsTask<P> {
    fn from(value: WorldId) -> Self {
        EcsTask(Arc::new(InternalEcsTask(
            AsyncTaskId::new().unwrap(),
            value,
            PhantomData,
        )))
    }
}

/// An `EcsTask` can be re-used in order to persist `SystemParams` like `Local`, `Changed`, or `Added`
pub struct EcsTask<P: SystemParam + 'static>(Arc<InternalEcsTask<P>>);

struct InternalEcsTask<P: SystemParam + 'static>(AsyncTaskId, WorldId, PhantomData<P>);

impl<T: SystemParam + 'static> Drop for InternalEcsTask<T> {
    fn drop(&mut self) {
        cleanup_ecs_task(self);
    }
}

impl<P: SystemParam + 'static> Clone for EcsTask<P> {
    fn clone(&self) -> Self {
        EcsTask(self.0.clone())
    }
}
impl<P: SystemParam + 'static> EcsTask<P> {
    /// Generates a new unique `EcsTask` that can be re-used in order to persist `SystemParams`
    /// like `Local`, `Changed`, or `Added`
    pub fn new(world_id: WorldId) -> Self {
        Self(Arc::new(InternalEcsTask(
            AsyncTaskId::new().unwrap(),
            world_id,
            PhantomData,
        )))
    }
}

struct PendingEcsCall<P: SystemParam + 'static, Func, Out>(
    PhantomData<P>,
    PhantomData<Out>,
    Option<Func>,
    (WorldId, InternedScheduleLabel),
    AsyncTaskId,
);

impl<P: SystemParam + 'static, Func, Out> Unpin for PendingEcsCall<P, Func, Out> {}

impl<P, Func, Out> Future for PendingEcsCall<P, Func, Out>
where
    P: SystemParam + 'static,
    for<'w, 's> Func: FnOnce(P::Item<'w, 's>) -> Out,
{
    type Output = Out;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        fn system_state_init<P: SystemParam + 'static>(world: &mut World, task_id: AsyncTaskId) {
            world.init_resource::<SystemStatePool<P>>();
            if !world
                .get_resource::<SystemStatePool<P>>()
                .unwrap()
                .0
                .read()
                .unwrap()
                .contains_key(&task_id)
            {
                let system_state = SystemState::<P>::new(world);
                let cq = ConcurrentQueue::bounded(1);
                match cq.push(system_state) {
                    Ok(_) => {}
                    Err(_) => {
                        panic!()
                    }
                }
                world
                    .get_resource::<SystemStatePool<P>>()
                    .unwrap()
                    .0
                    .write()
                    .unwrap()
                    .insert(task_id, cq);
            }
        }

        let task_id = self.4;
        let world_id = self.3 .0;

        match GLOBAL_WORLD_ACCESS.get(world_id, |world: UnsafeWorldCell| {
            // SAFETY: We have a fake-mutex around our world, so no one else can do mutable access to it.
            let Some(system_param_queue) = (unsafe { world.get_resource::<SystemStatePool<P>>() }) else { return Poll::Pending };
            let mut system_state = match system_param_queue.0.read().unwrap().get(&task_id) {
                None => return Poll::Pending,
                Some(cq) => cq.pop().unwrap(),
            };
            let out;
            // SAFETY: This is safe because we have a fake-mutex around our world cell, so only one thing can have access to it at a time.
            unsafe {
                let default_error_handler = world.default_error_handler();
                // Obtain params and immediately consume them with the closure,
                // ensuring the borrow ends before `apply`.
                if let Err(err) = SystemState::validate_param(&mut system_state, world) {
                    default_error_handler(err.into(), ErrorContext::System {
                        name: system_state.meta.name.clone(),
                        last_run: system_state.meta.last_run,
                    });
                }
                if !system_state.meta().is_send() {
                    default_error_handler(SystemParamValidationError::invalid::<NonSend<()>>(
                        "Cannot have your system be non-send / exclusive",
                    ).into(), ErrorContext::System {
                        name: system_state.meta.name.clone(),
                        last_run: system_state.meta.last_run,
                    });
                }
                let state = system_state.get_unchecked(world);
                out = self.as_mut().2.take().unwrap()(state);
            }
            // SAFETY: We have a fake-mutex around our world, so no one else can do mutable access to it.
            unsafe {
                match world
                    .get_resource::<SystemStatePool<P>>()
                    .unwrap()
                    .0
                    .read()
                    .unwrap()
                    .get(&task_id)
                    .unwrap()
                    .push(system_state)
                {
                    Ok(_) => {}
                    Err(_) => unreachable!("SystemStatePool should not be able to be removed if it previously existed, otherwise an invariant was violated"),
                }
            }
            Poll::Ready(out)
        }) {
            Some(awa) => awa,
            _ => {
                // This must be a static, sadly, because we must always make sure that we can store
                // our pending wakers no matter what. Everything else that we care about can be
                // stored on the world itself, but this must always be accessible, even if another
                // `async_access` is currently running.
                match GLOBAL_WAKE_REGISTRY
                    .0
                    .get_or_init(KeyedQueues::new)
                    .try_send(
                        &self.3,
                        (cx.waker().clone(), system_state_init::<P>, task_id),
                    ) {
                    Ok(_) => {}
                    // This should never panic because we never `close` our concurrent queues and
                    // the concurrent queue here is unbounded.
                    Err(_) => unreachable!(),
                }
                Poll::Pending
            }
        }
    }
}
