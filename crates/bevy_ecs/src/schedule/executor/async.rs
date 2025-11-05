use crate::schedule::r#async::keyed_queues::KeyedQueues;
use crate::schedule::{InternedScheduleLabel, ScheduleLabel};
use crate::system::{RunSystemError, SystemParamValidationError};
use crate::world::unsafe_world_cell::UnsafeWorldCell;
use crate::world::FromWorld;
use crate::{
    system::{SystemParam, SystemState},
    world::World,
};
use bevy_ecs::prelude::NonSend;
use bevy_ecs::world::{Mut, WorldId};
use bevy_platform::collections::HashMap;
use bevy_platform::sync::{Arc, Mutex, OnceLock, RwLock};
use concurrent_queue::ConcurrentQueue;
use core::any::TypeId;
use core::marker::PhantomData;
use core::pin::Pin;
use core::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};
use std::thread;

mod keyed_queues {
    use concurrent_queue::ConcurrentQueue;
    use std::sync::Arc;
    use std::{collections::HashMap, hash::Hash, sync::RwLock};

    /// HashMap<K, Arc<ConcurrentQueue<V>>> behind a single RwLock.
    /// - Writers only contend when creating a new key or GC'ing.
    /// - `push` is non-blocking (unbounded queue).
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
        /// ( Only blocks when the (WorldId, Schedule) has never been used before
        #[inline]
        pub fn try_send(&self, key: &K, val: V) -> Result<(), concurrent_queue::PushError<V>> {
            let q = self.get_or_create(key);
            q.push(val)
        }
    }
}

pub(crate) static GLOBAL_WORLD_ACCESS: WorldAccessRegistry = WorldAccessRegistry(OnceLock::new());

pub(crate) static GLOBAL_WAKE_REGISTRY: WakeRegistry = WakeRegistry(OnceLock::new());

#[derive(bevy_ecs_macros::Resource, Clone)]
pub(crate) struct WakeParkBarrier(thread::Thread, Arc<AtomicI64>);

#[derive(bevy_ecs_macros::Resource)]
pub(crate) struct SystemStatePool<T: SystemParam + 'static>(
    RwLock<HashMap<AsyncTaskId, ConcurrentQueue<SystemState<T>>>>,
);

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
                        let mut system_state = match concurrent_queue.pop() {
                            Ok(val) => val,
                            Err(_) => panic!(),
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

#[derive(Clone, Copy, Hash, PartialOrd, PartialEq, Eq)]
struct AsyncTaskId(usize);

/// The next [`AsyncTaskId`].
static MAX_TASK_ID: AtomicUsize = AtomicUsize::new(0);

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

pub(crate) struct WakeRegistry(
    OnceLock<
        KeyedQueues<
            (WorldId, InternedScheduleLabel),
            (Waker, fn(&mut World, AsyncTaskId), AsyncTaskId),
        >,
    >,
);

impl WakeRegistry {
    pub fn wait(&self, schedule: InternedScheduleLabel, world: &mut World) -> Option<()> {
        let world_id = world.id();
        let mut waker_list = std::vec![];
        while let Ok((waker, system_init, task_id)) = GLOBAL_WAKE_REGISTRY
            .0
            .get_or_init(|| KeyedQueues::new())
            .get_or_create(&(world_id, schedule))
            .pop()
        {
            // It's okay to call this every time, because it only *actually* inits the system if the task id is new
            system_init(world, task_id);
            waker_list.push(waker);
        }
        let waker_list_len = waker_list.len();
        if waker_list_len == 0 {
            return None;
        }
        world.insert_resource(WakeParkBarrier(
            thread::current(),
            Arc::new(AtomicI64::new(waker_list_len as i64 - 1)),
        ));
        if let None = GLOBAL_WORLD_ACCESS.set(world, || {
            for waker in waker_list {
                waker.wake();
            }
            thread::park();
        }) {
            return None;
        }
        world.try_resource_scope(|world, mut appliers: Mut<SystemParamAppliers>| {
            appliers.run(world);
        });
        Some(())
    }
}

/// The PhantomData here is just there cause it's a cute way of showing that we have a mutex around our unsafe worldcell and that's what the mutex is 'locking'
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
    pub(crate) fn set(&self, world: &mut World, func: impl FnOnce()) -> Option<()> {
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
                // clear it on the way out, even on panic
                self.slot.write().unwrap().take();
            }
        }
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
                std::mem::transmute(world.as_unsafe_world_cell()),
                Mutex::new(PhantomData),
            ));
            func()
        }
        Some(())
    }
    pub(crate) unsafe fn get<T>(
        &self,
        world_id: WorldId,
        func: impl FnOnce(UnsafeWorldCell) -> Poll<T>,
    ) -> Option<Poll<T>> {
        // it's okay to *not* do the RaiiThing on these early returns, because that means we aren't in a state
        // where a thread is parked because of our world.
        let a = self.0.get()?.read().unwrap();
        let b = a.get(&world_id)?.read().unwrap();
        let Some(our_thing) = b.as_ref() else {
            return None;
        };
        struct UnparkOnDropGuard(WakeParkBarrier);
        impl Drop for UnparkOnDropGuard {
            fn drop(&mut self) {
                let val = self.0 .1.fetch_add(-1, Ordering::SeqCst);
                if val == 0 {
                    self.0 .0.unpark();
                }
            }
        }
        let async_barrier = {
            our_thing
                .0
                .get_resource::<WakeParkBarrier>()
                .unwrap()
                .clone()
        };
        UnparkOnDropGuard(async_barrier.clone());
        // this allows us to effectively yield as if pending if the world doesn't exist rn.
        let _world = our_thing.1.try_lock().ok()?;
        // SAFETY: this is safe because we ensure no one else has access to the world.
        Some(func(our_thing.0))
    }
}

/// Allows you to access the ECS from any arbitrary async runtime.
/// Calls will never return immediately and will always start Pending at least once.
/// Call this with the same `PersistentTask` to persist SystemParams like Local or Changed
/// Just use `world_id` if you do not mind a new SystemParam being initialized every time.
pub fn async_access<P, Func, Out>(
    task_identifier: impl Into<EcsTask<P>>,
    schedule: impl ScheduleLabel,
    ecs_access: Func,
) -> impl Future<Output = Result<Out, RunSystemError>>
where
    P: SystemParam + 'static,
    for<'w, 's> Func: Clone + FnMut(P::Item<'w, 's>) -> Out,
{
    let task_identifier = task_identifier.into();
    PendingEcsCall::<P, Func, Out>(
        PhantomData::<P>,
        PhantomData,
        Some(ecs_access),
        (task_identifier.1, schedule.intern()),
        task_identifier.0,
    )
}

impl<T> From<WorldId> for EcsTask<T> {
    fn from(value: WorldId) -> Self {
        EcsTask::new(value)
    }
}

/// An EcsTask can be re-used in order to persist SystemParams like Local, Changed, or Added
pub struct EcsTask<T>(AsyncTaskId, WorldId, PhantomData<T>);

impl<T> Clone for EcsTask<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for EcsTask<T> {}
impl<T> EcsTask<T> {
    /// Generates a new unique PersistentTask that can be re-used in order to persist SystemParams
    /// like Local, Changed, or Added
    pub fn new(world_id: WorldId) -> Self {
        Self(AsyncTaskId::new().unwrap(), world_id, PhantomData)
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
    type Output = Result<Out, RunSystemError>;

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
        unsafe {
            match GLOBAL_WORLD_ACCESS.get(world_id, |world: UnsafeWorldCell| {
                let system_param_queue = match world.get_resource::<SystemStatePool<P>>() {
                    None => return Poll::Pending,
                    Some(system_param_queue) => system_param_queue,
                };

                let mut system_state = match system_param_queue.0.read().unwrap().get(&task_id) {
                    None => return Poll::Pending,
                    Some(cq) => cq.pop().unwrap(),
                };
                let out;
                // SAFETY: This is safe because we have a mutex around our world cell, so only one thing can have access to it at a time.
                #[expect(unused_unsafe)]
                unsafe {
                    // Obtain params and immediately consume them with the closure,
                    // ensuring the borrow ends before `apply`.
                    if let Err(err) = SystemState::validate_param(&mut system_state, world) {
                        return Poll::Ready(Err(err.into()));
                    }
                    if !system_state.meta().is_send() {
                        return Poll::Ready(Err(
                            SystemParamValidationError::invalid::<NonSend<()>>(
                                "Cannot have your system be non-send / exclusive",
                            )
                            .into(),
                        ));
                    }
                    let state = system_state.get_unchecked(world);
                    out = self.as_mut().2.take().unwrap()(state);
                }
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
                    Err(_) => panic!(),
                }
                Poll::Ready(Ok(out))
            }) {
                Some(awa) => awa,
                _ => {
                    match GLOBAL_WAKE_REGISTRY
                        .0
                        .get_or_init(|| KeyedQueues::new())
                        .try_send(
                            &self.3,
                            (cx.waker().clone(), system_state_init::<P>, task_id),
                        ) {
                        Ok(_) => {}
                        Err(_) => panic!(),
                    }
                    Poll::Pending
                }
            }
        }
    }
}
