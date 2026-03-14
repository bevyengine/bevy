use bevy_ecs::{
    error::ErrorContext,
    prelude::{IntoSystemSet, NonSend, SystemSet},
    schedule::InternedSystemSet,
    system::{IntoSystem, SystemParam, SystemParamValidationError, SystemState},
    world::{unsafe_world_cell::UnsafeWorldCell, World, WorldId},
};
use bevy_platform::{
    collections::HashMap,
    prelude::Vec,
    sync::{Arc, Mutex, OnceLock, RwLock},
};
use core::{
    any::Any,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use keyed_concurrent_queue::KeyedQueues;
use scoped_static_storage::ScopedStatic;
use std::sync::Condvar; // This is what prevents us from being no_std

#[derive(Clone)]
struct WakeSignal(Arc<(Mutex<bool>, Condvar)>);
impl WakeSignal {
    #[inline]
    pub fn new() -> Self {
        WakeSignal(Arc::new((Mutex::new(false), Condvar::new())))
    }
    #[inline]
    pub fn wait(&self) {
        let (lock, cv) = &*self.0;
        let mut signaled = lock.lock().unwrap();
        while !*signaled {
            signaled = cv.wait(signaled).unwrap();
        }
    }
}
impl Drop for WakeSignal {
    #[inline]
    fn drop(&mut self) {
        let (lock, cv) = &*self.0;
        let mut signaled = lock.lock().unwrap();
        *signaled = true;
        cv.notify_one();
    }
}

/// Run this function inside your system with the system itself as the second parameter.
/// This will pump the async ecs tasks and run them if they are ready.
pub fn run_async_ecs_system<
    Marker1,
    Marker2,
    T: IntoSystem<(), (), Marker1> + IntoSystemSet<Marker2>,
>(
    world: &mut World,
    system: T,
) {
    let interned = system.into_system_set().intern();
    // we limit it here to prevent *unbounded* async calls if we have a loop somewhere
    for _ in 0..100 {
        if GLOBAL_WAKE_REGISTRY.wait(interned, world).is_none() && {
            bevy_tasks::tick_global_task_pools_on_main_thread();
            GLOBAL_WAKE_REGISTRY.wait(interned, world).is_none()
        } {
            return;
        }
    }
}

/// This is an abstraction that temporarily and soundly stores the `UnsafeWorldCell` in a static so we can access
/// it from any async task, runtime, and thread.
static GLOBAL_WORLD_ACCESS: WorldAccessRegistry = WorldAccessRegistry(OnceLock::new());

/// The entrypoint, stores `Waker`s from `async_access`'s that wish to be polled with world access
/// also stores the generic function pointer to the concrete function that initializes the
/// system state for any set of `SystemParams`
static GLOBAL_WAKE_REGISTRY: WakeRegistry = WakeRegistry(OnceLock::new());

/// Is the `GLOBAL_WAKE_REGISTRY`
struct WakeRegistry(
    OnceLock<(
        KeyedQueues<(WorldId, InternedSystemSet), Uninitialized>,
        KeyedQueues<(WorldId, InternedSystemSet), ReadyToWake>,
    )>,
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
    #[inline]
    fn wait(&self, system_set: InternedSystemSet, world: &mut World) -> Option<()> {
        let world_id = world.id();
        let global_wake_registry = GLOBAL_WAKE_REGISTRY
            .0
            .get_or_init(|| (KeyedQueues::new(), KeyedQueues::new()));
        if global_wake_registry
            .0
            .get_or_create(&(world_id, system_set))
            .is_empty()
            && global_wake_registry
                .1
                .get_or_create(&(world_id, system_set))
                .is_empty()
        {
            return None;
        }
        let mut ecs_tasks = bevy_platform::prelude::vec![];
        while let Ok(ecs_task) = global_wake_registry
            .0
            .get_or_create(&(world_id, system_set))
            .pop()
        {
            ecs_tasks.push(ecs_task.initialize(world))
        }
        while let Ok(ecs_task) = global_wake_registry
            .1
            .get_or_create(&(world_id, system_set))
            .pop()
        {
            ecs_tasks.push(ecs_task)
        }
        let mut need_to_apply_system_state = None;
        GLOBAL_WORLD_ACCESS.set(world, || {
            let ecs_tasks = wait_for_async_tasks(ecs_tasks);
            need_to_apply_system_state = Some(ecs_tasks);
        });
        // Applies all the commands stored up to the world and other system state
        for task in need_to_apply_system_state? {
            task.apply_system_params(world);
        }
        Some(())
    }
}

struct Uninitialized {
    system_state_handler: Arc<dyn SystemStateHandler>,
    waker: (Waker, WakeSignal),
}

struct ReadyToWake {
    system_state_handler: Arc<dyn SystemStateHandler>,
    waker: (Waker, WakeSignal),
}

struct Awoken {
    system_state_handler: Arc<dyn SystemStateHandler>,
    barrier: WakeSignal,
}

struct NeedToApplySystemState {
    system_state_handler: Arc<dyn SystemStateHandler>,
}

impl Uninitialized {
    #[inline]
    fn initialize(self, world: &mut World) -> ReadyToWake {
        self.system_state_handler.system_init(world);
        let Self {
            system_state_handler,
            waker,
        } = self;
        ReadyToWake {
            system_state_handler,
            waker,
        }
    }
}

impl NeedToApplySystemState {
    #[inline]
    fn apply_system_params(self, world: &mut World) {
        self.system_state_handler.system_apply(world);
    }
}

#[inline]
fn wait_for_async_tasks(ecs_tasks: Vec<ReadyToWake>) -> Vec<NeedToApplySystemState> {
    let ecs_tasks = ecs_tasks
        .into_iter()
        .map(
            |ReadyToWake {
                 system_state_handler,
                 waker,
             }| {
                waker.0.wake();
                Awoken {
                    system_state_handler,
                    barrier: waker.1,
                }
            },
        )
        // we re-collect to ensure we fully exhaust the prior iterator
        // we want to have all the wakers call .wake() before the first barrier calls .wait()
        .collect::<Vec<_>>();
    bevy_tasks::tick_global_task_pools_on_main_thread();
    ecs_tasks
        .into_iter()
        .map(
            |Awoken {
                 system_state_handler,
                 barrier,
             }| {
                barrier.wait();
                NeedToApplySystemState {
                    system_state_handler,
                }
            },
        )
        .collect()
}

/// This is a very low contention, no contention in the normal execution path, way of storing and
/// using a `UnsafeWorldCell` from any thread/async task/async runtime.
struct WorldAccessRegistry(OnceLock<RwLock<HashMap<WorldId, Arc<ScopedStatic<World>>>>>);

impl WorldAccessRegistry {
    /// During this `func: FnOnce()` call, calling `get` will access the stored `UnsafeWorldCell`
    #[inline]
    fn set(&self, world: &mut World, func: impl FnOnce()) -> () {
        let this = self.0.get_or_init(|| RwLock::new(HashMap::new()));
        let world_id = world.id();
        if !this.read().unwrap().contains_key(&world_id) {
            this.write()
                .unwrap()
                .insert(world_id, Arc::new(ScopedStatic::new()));
        }
        let world_container = this.read().unwrap().get(&world_id).unwrap().clone();
        world_container.scope(world, func)
    }

    #[inline]
    fn get<T>(
        &self,
        world_id: WorldId,
        func: impl FnOnce(UnsafeWorldCell) -> Poll<T>,
    ) -> Option<Poll<T>> {
        let scoped_static = self.0.get()?.read().unwrap().get(&world_id)?.clone();
        Some(
            scoped_static
                .try_with(|world| func(world.as_unsafe_world_cell()))
                .ok()?,
        )
    }
}

impl<P: SystemParam + 'static> EcsTask<P> {
    /// Allows you to access the ECS from any arbitrary async runtime.
    #[inline]
    pub async fn run_system<Func, Out, M>(
        &self,
        system: impl IntoSystemSet<M>,
        ecs_access: Func,
    ) -> Out
    where
        for<'w, 's> Func: FnOnce(P::Item<'w, 's>) -> Out,
    {
        PendingEcsCall::<P, Func, Out> {
            phantom_data: Default::default(),
            ecs_func: Some(ecs_access),
            world_id_schedule: (self.world_id, system.into_system_set().intern()),
            barrier: None,
            system_state_handler: self.system_state_handler.clone(),
        }
        .await
    }
}

impl WorldId {
    /// Creates a new `EcsTask` with `P` SystemParam that can be cloned and re-referenced to
    /// persist system parameters like `Changed`, `Added` or `Local`.
    #[inline]
    pub fn ecs_task<P: SystemParam + 'static>(self) -> EcsTask<P> {
        EcsTask {
            phantom_data: Default::default(),
            world_id: self,
            system_state_handler: Arc::new(SystemStateHandlerStruct::<P>(Mutex::new(None))),
        }
    }
}

#[derive(PartialOrd, PartialEq, Eq, Ord, Hash, Debug, Copy, Clone)]
enum FutureState {
    Initialized,
    Uninitialized,
}

struct PendingEcsCall<P: SystemParam + 'static, Func, Out> {
    phantom_data: PhantomData<(P, Out)>,
    ecs_func: Option<Func>,
    world_id_schedule: (WorldId, InternedSystemSet),
    barrier: Option<WakeSignal>,
    system_state_handler: Arc<dyn SystemStateHandler>,
}

/// An `EcsTask` can be re-used in order to persist `SystemParams` like `Local`, `Changed`, or `Added`
pub struct EcsTask<P: SystemParam + 'static> {
    phantom_data: PhantomData<P>,
    world_id: WorldId,
    system_state_handler: Arc<dyn SystemStateHandler>,
}
impl<P: SystemParam + 'static> Clone for EcsTask<P> {
    fn clone(&self) -> Self {
        Self {
            phantom_data: Default::default(),
            world_id: self.world_id,
            system_state_handler: self.system_state_handler.clone(),
        }
    }
}

trait SystemStateHandler: Send + Sync {
    fn system_init(&self, world: &mut World);

    fn system_apply(&self, world: &mut World);

    fn as_any(&self) -> &dyn Any;

    fn future_state(&self) -> FutureState;
}

struct SystemStateHandlerStruct<P: SystemParam + 'static>(Mutex<Option<SystemState<P>>>);

impl<P: SystemParam + 'static> SystemStateHandler for SystemStateHandlerStruct<P> {
    fn system_init(&self, world: &mut World) {
        let mut maybe_system_state = self.0.lock().unwrap();
        if maybe_system_state.is_some() {
            return;
        }
        maybe_system_state.replace(SystemState::<P>::new(world));
    }
    fn system_apply(&self, world: &mut World) {
        self.0.lock().unwrap().as_mut().unwrap().apply(world);
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn future_state(&self) -> FutureState {
        match self.0.try_lock() {
            Err(_) => FutureState::Initialized,
            Ok(value) => match value.is_some() {
                true => FutureState::Initialized,
                false => FutureState::Uninitialized,
            },
        }
    }
}

impl<P: SystemParam + 'static, Func, Out> Unpin for PendingEcsCall<P, Func, Out> {}

impl<P, Func, Out> Future for PendingEcsCall<P, Func, Out>
where
    P: SystemParam + 'static,
    for<'w, 's> Func: FnOnce(P::Item<'w, 's>) -> Out,
{
    type Output = Out;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match GLOBAL_WORLD_ACCESS.get(self.world_id_schedule.0, |world: UnsafeWorldCell| {
            let ecs_func = self.ecs_func.take().unwrap();
            let Ok(mut system_state_guard) = self
                .system_state_handler
                .as_any()
                .downcast_ref::<SystemStateHandlerStruct<P>>()
                .unwrap()
                .0
                .try_lock()
            else {
                self.ecs_func.replace(ecs_func);
                return Poll::Pending;
            };

            let Some(system_state) = system_state_guard.as_mut() else {
                drop(system_state_guard);
                self.ecs_func.replace(ecs_func);
                return Poll::Pending;
            };
            let out;
            // SAFETY: This is safe because we have a fake-mutex around our world cell, so only one thing can have access to it at a time.
            unsafe {
                let default_error_handler = world.default_error_handler();
                // Obtain params and immediately consume them with the closure,
                // ensuring the borrow ends before `apply`.
                if let Err(err) = SystemState::validate_param(system_state, world) {
                    default_error_handler(
                        err.into(),
                        ErrorContext::System {
                            name: system_state.meta().name().clone(),
                            last_run: system_state.meta().last_run,
                        },
                    );
                }
                if !system_state.meta().is_send() {
                    default_error_handler(
                        SystemParamValidationError::invalid::<NonSend<()>>(
                            "Cannot have your system be non-send / exclusive",
                        )
                        .into(),
                        ErrorContext::System {
                            name: system_state.meta().name().clone(),
                            last_run: system_state.meta.last_run,
                        },
                    );
                }
                let state = system_state.get_unchecked(world);
                out = ecs_func(state);
            }
            drop(system_state_guard);
            self.barrier.take();
            Poll::Ready(out)
        }) {
            Some(Poll::Ready(out)) => Poll::Ready(out),
            _ => {
                // This must be a static, sadly, because we must always make sure that we can store
                // our pending wakers no matter what. Everything else that we care about can be
                // stored on the world itself, but this must always be accessible, even if another
                // `async_access` is currently running.
                let global_wake_registry = GLOBAL_WAKE_REGISTRY
                    .0
                    .get_or_init(|| (KeyedQueues::new(), KeyedQueues::new()));
                let wait_barrier = WakeSignal::new();
                self.barrier.replace(wait_barrier.clone());
                match self.system_state_handler.future_state() {
                    FutureState::Initialized => global_wake_registry
                        .1
                        .try_send(
                            &self.world_id_schedule,
                            ReadyToWake {
                                system_state_handler: self.system_state_handler.clone(),
                                waker: (cx.waker().clone(), wait_barrier),
                            },
                        )
                        .ok(),
                    FutureState::Uninitialized => global_wake_registry
                        .0
                        .try_send(
                            &self.world_id_schedule,
                            Uninitialized {
                                system_state_handler: self.system_state_handler.clone(),
                                waker: (cx.waker().clone(), wait_barrier),
                            },
                        )
                        .ok(),
                }
                .unwrap();
                // The above should never panic because we never `close` our concurrent queues and
                // the concurrent queue here is unbounded.
                Poll::Pending
            }
        }
    }
}
