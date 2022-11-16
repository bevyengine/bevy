use std::ops::{Deref, DerefMut};

use crate::prelude::World;

use super::{SystemMeta, SystemParam, SystemParamFetch, SystemParamItem, SystemParamState};

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
