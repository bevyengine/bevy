use std::sync::Arc;
use thread_local_object::ThreadLocal;

use crate::prelude::World;
use crate::{self as bevy_ecs, prelude::Component};

use super::{Resource, SystemMeta, SystemParam, SystemParamFetch, SystemParamState};

pub struct MainThread;
impl SystemParam for MainThread {
    type Fetch = MainThreadState;
}

pub struct MainThreadState;

// SAFETY: this impl defers to `MainThreadState`, which initializes
// and validates the correct world access
unsafe impl SystemParamState for MainThreadState {
    fn init(_world: &mut World, system_meta: &mut SystemMeta) -> Self {
        system_meta.set_non_send();
        MainThreadState
    }
}

impl<'w, 's> SystemParamFetch<'w, 's> for MainThreadState {
    type Item = MainThread;

    #[inline]
    unsafe fn get_param(
        _state: &'s mut Self,
        _system_meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        MainThread
    }
}

#[derive(Resource, Component)]
pub struct Tls<T: 'static>(Arc<ThreadLocal<T>>);

impl<T> Tls<T> {
    pub fn new(value: T) -> Self {
        let tls = Arc::new(ThreadLocal::new());
        tls.set(value);
        Tls(tls)
    }

    pub fn set(&self, value: T) -> Option<T> {
        self.0.set(value)
    }

    pub fn remove(&mut self) -> Option<T> {
        self.0.remove()
    }

    pub fn get<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.0.get(|t|
                // TODO: add typename to error message. possibly add reference to NonSend System param
                f(t.unwrap_or_else(||
                    panic!(
                        "Requested non-send resource {} does not exist on this thread.
                        You may be on the wrong thread or need to call .set on the resource.",
                        std::any::type_name::<R>()
                    )
                )))
    }

    // this takes an &mut self to trigger change detection when we get a mutable value out of the tls
    pub fn get_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        self.0.get_mut(|t|
                // TODO: add typename to error message. possibly add reference to NonSend System param
                f(t.unwrap_or_else(||
                    panic!(
                    "Requested non-send resource {} does not exist on this thread.
                        You may be on the wrong thread or need to call .set on the resource.",
                        std::any::type_name::<R>()
                    )
                )))
    }
}

// TODO: This drop impl is needed because AudioOutput was panicking when
// it was being dropped when the thread local storage was being dropped.
// This tries to drop the resource on the current thread, which fixes
// things for when the world is on the main thread, but would probably
// break if the world is moved to a different thread. We should figure
// out a more robust way of dropping the resource instead.
impl<T: 'static> Drop for Tls<T> {
    fn drop(&mut self) {
        self.remove();
    }
}

// SAFETY: pretty sure this is safe as ThreadLocal just wraps a usize and a phantom data
// and the usize is only written to on the call to ThreadLocal::new()
unsafe impl<T> Send for Tls<T> {}
// SAFETY: pretty sure this is safe as ThreadLocal just wraps a usize and a phantom data
// and the usize is only written to on the call to ThreadLocal::new()
unsafe impl<T> Sync for Tls<T> {}
