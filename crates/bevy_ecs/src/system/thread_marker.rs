use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use thread_local_object::ThreadLocal;

use crate as bevy_ecs;
use crate::prelude::World;

use super::{
    Resource, SystemMeta, SystemParam, SystemParamFetch, SystemParamItem, SystemParamState,
};

pub struct MainThread<'w, 's, T: SystemParam>(pub SystemParamItem<'w, 's, T>);
impl<'w, 's, T: SystemParam + Send + Sync + 'static> SystemParam for MainThread<'w, 's, T> {
    type Fetch = MainThreadState<T>;
}

impl<'w, 's, T: SystemParam + Send + Sync> Deref for MainThread<'w, 's, T> {
    type Target = SystemParamItem<'w, 's, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'w, 's, T: SystemParam + Send + Sync> DerefMut for MainThread<'w, 's, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'w, 's, T: SystemParam + Send + Sync> AsRef<SystemParamItem<'w, 's, T>>
    for MainThread<'w, 's, T>
{
    #[inline]
    fn as_ref(&self) -> &SystemParamItem<'w, 's, T> {
        self.deref()
    }
}

pub struct MainThreadState<T: SystemParam + Send + Sync>(T::Fetch);

// SAFETY: this impl defers to `NonSendMutState`, which initializes
// and validates the correct world access
unsafe impl<T: SystemParam + Send + Sync + 'static> SystemParamState for MainThreadState<T> {
    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        system_meta.set_non_send();
        Self(T::Fetch::init(world, system_meta))
    }
}

impl<'w, 's, T: SystemParam + Send + Sync + 'static> SystemParamFetch<'w, 's> for MainThreadState<T>
where
    T::Fetch: SystemParamState,
{
    type Item = MainThread<'w, 's, T>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        MainThread(T::Fetch::get_param(
            &mut state.0,
            system_meta,
            world,
            change_tick,
        ))
    }
}

#[derive(Resource)]
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

// pretty sure this is safe as ThreadLocal just wraps a usize and a phatom data
// the usize is only written to on the call to ThreadLocal::new()
unsafe impl<T> Send for Tls<T> {}
unsafe impl<T> Sync for Tls<T> {}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::prelude::World;
    use crate::schedule::{Stage, SystemStage};
    use crate::system::{MainThread, ResMut, Resource};

    #[derive(Resource)]
    struct A(pub usize);

    #[test]
    fn test() {
        fn system(mut non_send_res: MainThread<ResMut<A>>) {
            (*non_send_res).0 = 1;
        }
        let mut world = World::new();
        world.insert_resource(A(0));
        let mut stage = SystemStage::parallel();
        stage.add_system(system);
        stage.run(&mut world);
        let res = world.get_resource::<A>().unwrap();
        assert_eq!(res.0, 1);
    }
}
