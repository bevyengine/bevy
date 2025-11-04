use crate::schedule::r#async::keyed_queues::KeyedQueues;
use crate::schedule::{InternedScheduleLabel, ScheduleLabel};
use crate::system::{RunSystemError, SystemMeta};
use crate::world::unsafe_world_cell::UnsafeWorldCell;
use crate::world::FromWorld;
use crate::{
    system::{SystemParam, SystemState},
    world::World,
};
use bevy_ecs::world::{Mut, WorldId};
use bevy_platform::collections::HashMap;
use concurrent_queue::ConcurrentQueue;
use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::pin::Pin;
use std::prelude::v1::{Box, Vec};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::task::{Context, Poll, Waker};
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

pub(crate) static ASYNC_ECS_WORLD_ACCESS: LockWrapper = LockWrapper(OnceLock::new());

pub(crate) static ASYNC_ECS_WAKER_LIST: EcsWakerList = EcsWakerList(OnceLock::new());

#[derive(bevy_ecs_macros::Resource, Clone)]
pub(crate) struct AsyncBarrier(thread::Thread, Arc<AtomicI64>);

#[derive(bevy_ecs_macros::Resource)]
pub(crate) struct SystemParamQueue<T: SystemParam + 'static>(
    RwLock<HashMap<TaskId, ConcurrentQueue<SystemState<T>>>>,
);

#[derive(bevy_ecs_macros::Resource, Default)]
pub(crate) struct SystemParamApplications(
    HashMap<TypeId, Box<dyn FnMut(&mut World) + Send + Sync + 'static>>,
);
impl SystemParamApplications {
    fn run(&mut self, world: &mut World) {
        for closure in self.0.values_mut() {
            closure(world);
        }
    }
}
impl<T: SystemParam + 'static> FromWorld for SystemParamQueue<T> {
    fn from_world(world: &mut World) -> Self {
        let this = Self(RwLock::new(HashMap::default()));
        world.init_resource::<SystemParamApplications>();
        let mut system_param_applications =
            world.get_resource_mut::<SystemParamApplications>().unwrap();
        if !system_param_applications.0.contains_key(&TypeId::of::<T>()) {
            system_param_applications.0.insert(
                TypeId::of::<T>(),
                Box::new(|world: &mut World| {
                    world.try_resource_scope(
                        |world, system_param_queue: Mut<SystemParamQueue<T>>| {
                            for concurrent_queue in system_param_queue.0.read().unwrap().values() {
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
                        },
                    );
                }),
            );
        }
        this
    }
}

#[derive(Clone, Copy, Hash, PartialOrd, PartialEq, Eq)]
struct TaskId(usize);

/// The next [`TaskId`].
static MAX_TASK_ID: AtomicUsize = AtomicUsize::new(0);

impl TaskId {
    /// Create a new, unique [`TaskId`]. Returns [`None`] if the supply of unique
    /// [`TaskId`]s has been exhausted
    ///
    /// Please note that the [`TaskId`]s created from this method are unique across
    /// time - if a given [`TaskId`] is [`Drop`]ped its value still cannot be reused
    pub fn new() -> Option<Self> {
        MAX_TASK_ID
            // We use `Relaxed` here since this atomic only needs to be consistent with itself
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                val.checked_add(1)
            })
            .map(TaskId)
            .ok()
    }
}

pub(crate) struct EcsWakerList(
    OnceLock<
        KeyedQueues<(WorldId, InternedScheduleLabel), (Waker, fn(&mut World, TaskId), TaskId)>,
    >,
);

impl EcsWakerList {
    pub fn wait(&self, schedule: InternedScheduleLabel, world: &mut World) -> Option<()> {
        let world_id = world.id();
        let mut waker_list = std::vec![];
        while let Ok((waker, system_init, task_id)) = ASYNC_ECS_WAKER_LIST
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
        world.insert_resource(AsyncBarrier(
            thread::current(),
            Arc::new(AtomicI64::new(waker_list_len as i64 - 1)),
        ));
        if let None = ASYNC_ECS_WORLD_ACCESS.set(world, || {
            for waker in waker_list {
                waker.wake();
            }
            thread::park();
        }) {
            return None;
        }
        world.try_resource_scope(
            |world, mut system_param_applications: Mut<SystemParamApplications>| {
                system_param_applications.run(world);
            },
        );
        Some(())
    }
}

pub(crate) struct LockWrapper(OnceLock<Arc<Mutex<Option<UnsafeWorldCell<'static>>>>>);

impl LockWrapper {
    pub(crate) fn set(&self, world: &mut World, func: impl FnOnce()) -> Option<()> {
        // local RAII type
        struct ClearOnDrop<'a> {
            slot: &'a Mutex<Option<UnsafeWorldCell<'static>>>,
        }

        impl<'a> Drop for ClearOnDrop<'a> {
            fn drop(&mut self) {
                // clear it on the way out, even on panic
                self.slot.lock().unwrap().take();
            }
        }

        unsafe {
            let mut awa = self
                .0
                .get_or_init(|| Arc::new(Mutex::new(None)))
                .try_lock()
                .ok()?;
            // this guard lives until the end of the function
            let _clear = ClearOnDrop {
                slot: self.0.get().unwrap(),
            };
            // SAFETY: This mem transmute is safe only because we drop it after, and our ASYNC_ECS_WORLD_ACCESS is private, and we don't clone it
            // where we do use it, so the lifetime doesn't get propagated anywhere.
            awa.replace(std::mem::transmute(world.as_unsafe_world_cell()));
            drop(awa);
            func()
        }
        Some(())
    }
    pub(crate) unsafe fn get<T>(
        &self,
        func: impl FnOnce(UnsafeWorldCell) -> Poll<T>,
    ) -> Option<Poll<T>> {
        let mut uwu = self.0.get()?.lock().ok()?;
        if let Some(inner) = uwu.clone() {
            // SAFETY: this is safe because we ensure no one else has access to the world.
            let out;
            unsafe {
                out = func(inner);
            }
            return Some(out);
        }
        None
    }
}

struct TypeMap(
    HashMap<
        TypeId,
        (
            Box<dyn Any + 'static + Send + Sync>,
            Mutex<Box<dyn FnMut(&mut World) + 'static + Send>>,
        ),
    >,
);
impl TypeMap {
    pub fn set<T: SystemParam + Any + 'static>(&mut self, _t: &SystemState<T>) {
        let (tx, rx) = std::sync::mpsc::channel::<SystemState<T>>();

        self.0.insert(
            TypeId::of::<T>(),
            (
                Box::new(tx),
                Mutex::new(Box::new(move |world: &mut World| {
                    for mut thing in rx.try_iter() {
                        thing.apply(world);
                    }
                })),
            ),
        );
    }
    pub fn has<T: SystemParam + 'static + Any>(&self, _t: &SystemState<T>) -> bool {
        self.0.contains_key(&TypeId::of::<T>())
    }
    pub fn send<T: SystemParam + Any + 'static>(&self, t: SystemState<T>) -> Option<()> {
        self.0
            .get(&TypeId::of::<T>())?
            .0
            .downcast_ref::<std::sync::mpsc::Sender<SystemState<T>>>()
            .unwrap()
            .send(t)
            .ok()?;
        Some(())
    }
    pub fn run_all(&mut self, world: &mut World) {
        for (_, closure) in self.0.values_mut() {
            closure.lock().unwrap()(world);
        }
    }
}

pub fn async_access<P, Func, Out>(
    task_identifier: impl Into<TaskIdentifier<P>>,
    schedule: impl ScheduleLabel,
    ecs_access: Func,
) -> impl Future<Output = Result<Out, RunSystemError>>
where
    P: SystemParam + 'static,
    for<'w, 's> Func: Clone + FnMut(P::Item<'w, 's>) -> Out,
{
    let task_identifier = task_identifier.into();
    SystemParamThing::<P, Func, Out>(
        PhantomData::<P>,
        PhantomData,
        Some(ecs_access),
        (task_identifier.1, schedule.intern()),
        task_identifier.0,
    )
}

impl<T> From<WorldId> for TaskIdentifier<T> {
    fn from(value: WorldId) -> Self {
        TaskIdentifier::new(value)
    }
}

pub struct TaskIdentifier<T>(TaskId, WorldId, PhantomData<T>);

impl<T> Clone for TaskIdentifier<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for TaskIdentifier<T> {}
impl<T> TaskIdentifier<T> {
    pub fn new(world_id: WorldId) -> Self {
        Self(TaskId::new().unwrap(), world_id, PhantomData)
    }
}

struct SystemParamThing<P: SystemParam + 'static, Func, Out>(
    PhantomData<P>,
    PhantomData<Out>,
    Option<Func>,
    (WorldId, InternedScheduleLabel),
    TaskId,
);

impl<P: SystemParam + 'static, Func, Out> Unpin for SystemParamThing<P, Func, Out> {}

impl<P, Func, Out> Future for SystemParamThing<P, Func, Out>
where
    P: SystemParam + 'static,
    for<'w, 's> Func: FnOnce(P::Item<'w, 's>) -> Out,
{
    type Output = Result<Out, RunSystemError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        fn system_state_init<P: SystemParam + 'static>(world: &mut World, task_id: TaskId) {
            world.init_resource::<SystemParamQueue<P>>();
            if !world
                .get_resource::<SystemParamQueue<P>>()
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
                    .get_resource::<SystemParamQueue<P>>()
                    .unwrap()
                    .0
                    .write()
                    .unwrap()
                    .insert(task_id, cq);
            }
        }

        let task_id = self.4;
        unsafe {
            match ASYNC_ECS_WORLD_ACCESS.get(|world: UnsafeWorldCell| {
                struct RaiiThing(AsyncBarrier);
                impl Drop for RaiiThing {
                    fn drop(&mut self) {
                        let val = self.0 .1.fetch_add(-1, Ordering::SeqCst);
                        if val == 0 {
                            self.0 .0.unpark();
                        }
                    }
                }
                let async_barrier = { world.get_resource::<AsyncBarrier>().unwrap().clone() };
                RaiiThing(async_barrier.clone());

                let system_param_queue = match world.get_resource::<SystemParamQueue<P>>() {
                    None => return Poll::Pending,
                    Some(system_param_queue) => system_param_queue,
                };

                let mut system_state = match system_param_queue.0.read().unwrap().get(&task_id) {
                    None => return Poll::Pending,
                    Some(cq) => cq.pop().unwrap(),
                };
                let out;
                // SAFETY: This is safe because we have a mutex around our world cell, so only one thing can have access to it at a time.
                unsafe {
                    // Obtain params and immediately consume them with the closure,
                    // ensuring the borrow ends before `apply`.
                    if let Err(err) = SystemState::validate_param(&mut system_state, world) {
                        return Poll::Ready(Err(err.into()));
                    }
                    let state = system_state.get_unchecked(world);
                    out = self.as_mut().2.take().unwrap()(state);
                }
                match world
                    .get_resource::<SystemParamQueue<P>>()
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
                    match ASYNC_ECS_WAKER_LIST
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
