//! This modules defins and implements links between worlds.
//! This allows a system to have [`SystemParam`](crate::system::SystemParam)s that link to other worlds besides the world the system lives in.

use core::marker::PhantomData;

use crate::{
    component::ComponentId,
    prelude::DetectChangesMut,
    system::{Res, SystemMeta, SystemParam, SystemParamItem},
};

use super::{unsafe_world_cell::UnsafeWorldCell, World, WorldId};

/// This trait allows a link to be made between a two worlds via a system. One world can hold a nested world via this link.
pub trait WorldLink: Send + Sync + 'static {
    /// gets the linked world mutably
    fn get_world_mut(&mut self) -> &mut World;
    /// gets the linked world as an unsafe cell.
    ///
    /// # Safety
    /// It is up to the caller to ensure the cell is used responsibly and does not conflict with any other access to the link.
    unsafe fn get_unsafe_world(&self) -> UnsafeWorldCell;
}

/// this is purely used to facilitate the resource derive
mod wrapper {
    use super::WorldLink;
    use crate as bevy_ecs;
    use bevy_ecs_macros::Resource;

    /// This resouce links one world to another via a [`WorldLink`].
    #[derive(Resource)]
    pub struct Link<L: WorldLink> {
        /// Safety: since the inner value of `Link` is private, and there is currently only immutably access to the resoruce thanks to using `Res<Link<L>>`,
        /// the only way the link is being accessed is through the interfaces and abstractions provided in this file, which are checked properly via [`SystemMeta`]'s access lists.
        pub(super) inner: L,
    }
}
use derive_more::derive::{Deref, DerefMut};
pub use wrapper::Link;

/// A [`SystemParam`] that is sourced from a linked world via a [`Link`].
#[derive(Deref, DerefMut)]
pub struct Linked<'w, 's, L: WorldLink, P: SystemParam> {
    /// the parameter item requested
    #[deref]
    #[deref_mut]
    item: SystemParamItem<'w, 's, P>,
    marker: PhantomData<L>,
}

// SAFETY: This assumes the Link exists, pannicing otherwise. This assumes that the inner system parameter will report access properly.
unsafe impl<L: WorldLink, P: SystemParam> SystemParam for Linked<'_, '_, L, P> {
    // the [`ComponentId`] is for `Res<Link<L>>`
    type State = (P::State, ComponentId, WorldId);

    type Item<'world, 'state> = Linked<'world, 'state, L, P>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let link_res = Res::<Link<L>>::init_state(world, system_meta);
        let mut link = world
            .get_resource_mut::<Link<L>>()
            .expect("attempting to build a system parameter from a missing link");
        let state = P::init_state(
            link.bypass_change_detection().inner.get_world_mut(),
            system_meta,
        );
        (state, link_res, world.id())
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        _change_tick: crate::component::Tick,
    ) -> Self::Item<'world, 'state> {
        // prevents undefined behavior. If this ever fails, the safety of this system param is comprimised.
        assert_eq!(state.2, world.id());
        // Safety: conflicts are prevented by initing `Res<Link<L>>` in `init_state`.
        let link = unsafe {
            world
                .get_resource::<Link<L>>()
                .expect("attempting to get a system parameter from a missing link")
        };
        // Safety: See [`Link::inner`].
        let link = unsafe { link.inner.get_unsafe_world() };
        // IMPORTANT: We use the link's change tick to keep it fully seperate from the base world. This is what is really relevant.
        let item = P::get_param(&mut state.0, system_meta, link, link.change_tick());
        Linked {
            item,
            marker: PhantomData,
        }
    }

    unsafe fn new_archetype(
        _state: &mut Self::State,
        _archetype: &crate::archetype::Archetype,
        _system_meta: &mut SystemMeta,
    ) {
        // how do we inform the inner parameter of archetype changes to the nested world?
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        let mut link = world
            .get_resource_mut::<Link<L>>()
            .expect("attempting to apply a system parameter from a missing link");
        P::apply(
            &mut state.0,
            system_meta,
            link.bypass_change_detection().inner.get_world_mut(),
        );
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, mut world: super::DeferredWorld) {
        let mut link = world
            .get_resource_mut::<Link<L>>()
            .expect("attempting to queue a system parameter from a missing link");
        P::apply(
            &mut state.0,
            system_meta,
            link.bypass_change_detection().inner.get_world_mut(),
        );
    }

    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        let Some(link) = world.get_resource::<Link<L>>() else {
            return false;
        };
        // Safety: See [`Link::inner`]
        P::validate_param(&state.0, system_meta, unsafe {
            link.inner.get_unsafe_world()
        })
    }
}
