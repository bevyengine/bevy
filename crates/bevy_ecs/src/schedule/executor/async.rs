use crate::world::unsafe_world_cell::UnsafeWorldCell;
use bevy_ecs::prelude::World;
use bevy_ecs::system::{IntoSystem, RunSystemError, RunSystemOnce, System, SystemIn, SystemInput, SystemParam, SystemState};
use bevy_ecs::world::WorldId;
use bevy_platform::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::prelude::v1::Vec;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll};
use std::thread;

pub(crate) static ASYNC_ECS_WORLD_ACCESS: LockWrapper = LockWrapper(OnceLock::new());
pub(crate) static ASYNC_ECS_WAKER_LIST: EcsWakerList = EcsWakerList(OnceLock::new());
#[derive(bevy_ecs_macros::Resource)]
pub(crate) struct AsyncBarrier(thread::Thread, AtomicI64);

pub(crate) struct EcsWakerList(OnceLock<Mutex<HashMap<WorldId, Vec<std::task::Waker>>>>);

impl EcsWakerList {
    pub fn wait(&self, world: &mut World) -> Option<()> {
        let world_id = world.id();
        // We intentionally do not hold this lock the whole time because we are emptying the vec, it's gonna be all new wakers next time.
        let waker_list = self
            .0
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .ok()?
            .remove(&world_id)?;
        let waker_list_len = waker_list.len();
        world.insert_resource(AsyncBarrier(
            thread::current(),
            AtomicI64::new(waker_list_len as i64),
        ));
        let drop_wrapper = unsafe { ASYNC_ECS_WORLD_ACCESS.set(world) };
        for waker in waker_list {
            waker.wake();
        }
        if waker_list_len != 0 {
            thread::park();
        }
        drop(drop_wrapper);
        Some(())
    }
}

pub(crate) struct LockWrapper(OnceLock<Arc<Mutex<Option<UnsafeWorldCell<'static>>>>>);

// SAFETY: Because this lockwrapper removes the UnsafeWorldCell when it goes out of scope, we ensure the UnsafeWorlCell inside can't actually escape the lifetime.
impl Drop for LockWrapper {
    fn drop(&mut self) {
        if let Some(awa) = self.0.get() {
            if let Ok(mut awa) = awa.lock() {
                awa.take();
            }
        }
    }
}

// SAFETY: Because this lockwrapper removes the UnsafeWorldCell when it goes out of scope, we ensure the UnsafeWorlCell inside can't actually escape the lifetime.
pub(crate) struct DropWrapper {
    _unread: OnceLock<Arc<Mutex<Option<UnsafeWorldCell<'static>>>>>,
}

impl LockWrapper {
    pub(crate) unsafe fn set(&self, world: &mut World) -> DropWrapper {
        unsafe {
            self.0
                .get_or_init(|| Arc::new(Mutex::new(None)))
                .lock()
                .unwrap()
                // SAFETY: This mem transmute is safe only because we drop it after, and our ASYNC_ECS_WORLD_ACCESS is private, and we don't clone it
                // where we do use it, so the lifetime doesn't get propagated anywhere.
                .replace(std::mem::transmute(world.as_unsafe_world_cell()));
            DropWrapper {
                _unread: self.0.clone(),
            }
        }
    }
    pub(crate) unsafe fn get<T>(&self, func: impl FnOnce(&mut World) -> T) -> Option<T> {
        let uwu = self.0.get()?.try_lock().ok()?.clone()?;
        // SAFETY: this is safe because we ensure no one else has access to the world.
        let out;
        unsafe {
            out = func(uwu.world_mut());
        }
        Some(out)
    }
}

pub async fn async_access<P, Func, Out>(world_id: WorldId, ecs_access: Func) -> Out
where
    P: SystemParam + 'static,
    for<'w, 's> Func: FnOnce(P::Item<'w, 's>) -> Out,
{
    SystemParamThing::<P, Func, Out>(PhantomData::<P>, PhantomData, Some(ecs_access), world_id)
        .await
}

struct SystemParamThing<'a, 'b, P: SystemParam + 'static, Func, Out>(
    PhantomData<P>,
    PhantomData<(Out, &'a (), &'b ())>,
    Option<Func>,
    WorldId,
);

impl<'a, 'b, P: SystemParam + 'static, Func, Out> Unpin for SystemParamThing<'a, 'b, P, Func, Out> {}

impl<'a, 'b, P, Func, Out> Future for SystemParamThing<'a, 'b, P, Func, Out>
where
    P: SystemParam + 'static,
    for<'w, 's> Func: FnOnce(P::Item<'w, 's>) -> Out,
{
    type Output = Out;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match unsafe {
            ASYNC_ECS_WORLD_ACCESS.get(|world: &mut World| {
                let out;
                // SAFETY: This is safe because we have a mutex around our world cell, so only one thing can have access to it at a time.
                let mut system_state: SystemState<P> = SystemState::new(world);
                {
                    // Obtain params and immediately consume them with the closure,
                    // ensuring the borrow ends before `apply`.
                    let state = system_state.get_unchecked(world.as_unsafe_world_cell());
                    out = self.as_mut().2.take().unwrap()(state);
                    system_state.apply(world);
                }
                let async_barrier = world.get_resource::<AsyncBarrier>().unwrap();
                if async_barrier.1.fetch_add(-1, Ordering::Relaxed) == 0 {
                    async_barrier.0.unpark();
                }
                out
            })
        } {
            Some(awa) => Poll::Ready(awa),
            None => {
                let mut hashmap = ASYNC_ECS_WAKER_LIST
                    .0
                    .get_or_init(|| Mutex::new(HashMap::new()))
                    .lock()
                    .unwrap();
                if !hashmap.contains_key(&self.3) {
                    hashmap.insert(self.3.clone(), Vec::new());
                }
                hashmap.get_mut(&self.3).unwrap().push(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
