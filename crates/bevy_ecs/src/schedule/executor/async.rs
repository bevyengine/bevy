use crate::schedule::{InternedScheduleLabel, ScheduleLabel};
use crate::system::{RunSystemError, SystemMeta};
use crate::world::unsafe_world_cell::UnsafeWorldCell;
use crate::{
    system::{SystemParam, SystemState},
    world::World,
};
use bevy_ecs::world::WorldId;
use bevy_platform::collections::HashMap;
use std::any::Any;
use std::marker::PhantomData;
use std::pin::Pin;
use std::prelude::v1::{Box, Vec};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll};
use std::thread;

pub(crate) static ASYNC_ECS_WORLD_ACCESS: LockWrapper = LockWrapper(OnceLock::new());
pub(crate) static ASYNC_ECS_WAKER_LIST: EcsWakerList = EcsWakerList(OnceLock::new());
#[derive(bevy_ecs_macros::Resource, Clone)]
pub(crate) struct AsyncBarrier(
    thread::Thread,
    Arc<AtomicI64>,
    std::sync::mpsc::Sender<Box<dyn FnOnce(&mut World) + Send + Sync + 'static>>,
);

enum MyThing {
    FnClosure(Box<dyn FnOnce(&mut World) -> Box<dyn Any + Send + Sync> + Send + Sync>),
    SystemState(Box<dyn Any + Send + Sync>),
}

impl MyThing {
    pub fn into_state(mut self, world: &mut World) -> MyThing {
        match self {
            MyThing::FnClosure(mut f) => {
                let state = f(world);
                MyThing::SystemState(state)
            }
            MyThing::SystemState(_) => self,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialOrd, PartialEq, Eq)]
pub struct TaskId(usize);

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
    OnceLock<Mutex<(HashMap<(WorldId, InternedScheduleLabel), Vec<(std::task::Waker, TaskId)>>, HashMap<TaskId, MyThing>)>>,
);


impl EcsWakerList {
    pub fn wait(&self, schedule: InternedScheduleLabel, world: &mut World) -> Option<()> {
        let this = self.0.get_or_init(|| Mutex::new((HashMap::new(), HashMap::new())));
        let world_id = world.id();
        // We intentionally do not hold this lock the whole time because we are emptying the vec, it's gonna be all new wakers next time.
        let mut waker_list = this
            .try_lock()
            .ok()?
            .0
            .remove(&(world_id, schedule))?;
        let waker_list_len = waker_list.len();
        let (tx, rx) = std::sync::mpsc::channel();
        world.insert_resource(AsyncBarrier(
            thread::current(),
            Arc::new(AtomicI64::new(waker_list_len as i64 - 1)),
            tx,
        ));
        for (_, task_id) in waker_list.iter() {
            let mut uwu = this.lock()
                .unwrap();
            let task = uwu.1.remove(task_id).unwrap();
            uwu.1.insert(*task_id, task.into_state(world));
            drop(uwu);
        }
        if let None = ASYNC_ECS_WORLD_ACCESS.set(world, || {
            for (waker, _) in waker_list {
                waker.wake();
            }
            if waker_list_len != 0 {
                std::println!("thread is parking");
                thread::park();
                std::println!("thread is unparked");
            } else {
                panic!("AWA");
            }
        }) {
            return None
        }
        for thing in rx.try_iter() {
            thing(world);
        }
        Some(())
    }
}

pub(crate) struct LockWrapper(OnceLock<Arc<Mutex<Option<UnsafeWorldCell<'static>>>>>);

impl LockWrapper {
    pub(crate) fn set(&self, world: &mut World, func: impl FnOnce()) -> Option<()>{
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
            let mut awa = self.0
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

pub async fn async_access<P, Func, Out>(
    world_id: WorldId,
    schedule: impl ScheduleLabel,
    ecs_access: Func,
) -> Result<Out, RunSystemError>
where
    P: SystemParam + 'static,
    for<'w, 's> Func: FnOnce(P::Item<'w, 's>) -> Out,
{
    SystemParamThing::<P, Func, Out>(
        PhantomData::<P>,
        PhantomData,
        Some(ecs_access),
        (world_id, schedule.intern()),
        TaskId::new().unwrap(),
    )
    .await
}

struct SystemParamThing<'a, 'b, P: SystemParam + 'static, Func, Out>(
    PhantomData<P>,
    PhantomData<(Out, &'a (), &'b ())>,
    Option<Func>,
    (WorldId, InternedScheduleLabel),
    TaskId,
);

impl<'a, 'b, P: SystemParam + 'static, Func, Out> Unpin for SystemParamThing<'a, 'b, P, Func, Out> {}

impl<'a, 'b, P, Func, Out> Future for SystemParamThing<'a, 'b, P, Func, Out>
where
    P: SystemParam + 'static,
    for<'w, 's> Func: FnOnce(P::Item<'w, 's>) -> Out,
{
    type Output = Result<Out, RunSystemError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            match ASYNC_ECS_WORLD_ACCESS.get(|world: UnsafeWorldCell| {
                let async_barrier = { world.get_resource::<AsyncBarrier>().unwrap().clone() };
                let our_thing = async_barrier.1.load(Ordering::SeqCst);
                std::println!("A: {}", our_thing);
                struct RaiiThing(AsyncBarrier);
                impl Drop for RaiiThing {
                    fn drop(&mut self) {
                        std::println!("ready to drop uwu");
                        let val = self.0.1.fetch_add(-1, Ordering::SeqCst);
                        std::println!("counter is: {val}");
                        if val == 0 {
                            self.0.0.unpark();
                        }
                    }
                }
                RaiiThing(async_barrier.clone());
                let out;
                std::println!("B: {}", our_thing);
                let mut hashmap = ASYNC_ECS_WAKER_LIST
                    .0
                    .get_or_init(|| Mutex::new((HashMap::new(), HashMap::new())))
                    .lock()
                    .unwrap();
                let Some(awa) = hashmap.1.remove(&self.4)  else {
                    return Poll::Pending
                };
                drop(hashmap);
                let mut uwu = match awa {
                    MyThing::FnClosure(_) => panic!(),
                    MyThing::SystemState(state) => *state.downcast::<SystemState<P>>().unwrap()
                };
                let mut system_state = uwu;
                //let mut system_state = SystemState::<P>::new(world.world_mut());
                // SAFETY: This is safe because we have a mutex around our world cell, so only one thing can have access to it at a time.
                unsafe {
                    // Obtain params and immediately consume them with the closure,
                    // ensuring the borrow ends before `apply`.
                    if let Err(err) = SystemState::validate_param(&mut system_state, world) {
                        panic!();
                        return Poll::Ready(Err(err.into()));
                    }
                    std::println!("C: {}", our_thing);
                    let state = system_state.get_unchecked(world);
                    std::println!("D: {}", our_thing);
                    out = self.as_mut().2.take().unwrap()(state);
                    std::println!("E: {}", our_thing);
                }
                //system_state.apply(world.world_mut());
                std::println!("F: {}", our_thing);
                if let Err(err) = async_barrier.2.send(Box::new(move |world: &mut World| {
                    system_state.apply(world);
                })) {
                    return Poll::Ready(Err(err.into()));
                }
                Poll::Ready(Ok(out))
            }) {
                Some(Poll::Pending) => {
                    let mut hashmap = ASYNC_ECS_WAKER_LIST
                        .0
                        .get_or_init(|| Mutex::new((HashMap::new(), HashMap::new())))
                        .lock()
                        .unwrap();
                    if !hashmap.0.contains_key(&self.3) {
                        hashmap.0.insert(self.3.clone(), Vec::new());
                    }
                    hashmap.0
                        .get_mut(&self.3)
                        .unwrap()
                        .push((cx.waker().clone(), self.4));
                    if !hashmap.1.contains_key(&self.4) {
                        hashmap.1
                            .insert(
                                self.4,
                                MyThing::FnClosure(Box::new(
                                    |world: &mut World| -> Box<dyn Any + Send + Sync> {
                                        Box::new(SystemState::<P>::new(world))
                                    },
                                )),
                            );
                    }
                    Poll::Pending
                }
                None => {
                    let mut hashmap = ASYNC_ECS_WAKER_LIST
                        .0
                        .get_or_init(|| Mutex::new((HashMap::new(), HashMap::new())))
                        .lock()
                        .unwrap();
                    if !hashmap.0.contains_key(&self.3) {
                        hashmap.0.insert(self.3.clone(), Vec::new());
                    }
                    hashmap.0
                        .get_mut(&self.3)
                        .unwrap()
                        .push((cx.waker().clone(), self.4));
                    hashmap.1
                        .insert(
                            self.4,
                            MyThing::FnClosure(Box::new(
                                |world: &mut World| -> Box<dyn Any + Send + Sync> {
                                    Box::new(SystemState::<P>::new(world))
                                },
                            )),
                        );
                    Poll::Pending
                }
                Some(awa) => awa,
            }
        }
    }
}
