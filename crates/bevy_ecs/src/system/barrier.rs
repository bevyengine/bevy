use bevy_utils::all_tuples;

use crate::component::Tick;
use crate::prelude::World;
use crate::schedule::{InternedSystemSet, SystemSet};
use crate::system::{SystemMeta, SystemParam};
use crate::world::unsafe_world_cell::UnsafeWorldCell;
use std::marker::PhantomData;

/// List of barrier dependencies for the system.
pub trait BarrierList {
    /// System sets before this system.
    fn before_list() -> Vec<InternedSystemSet>;
    /// System sets after this system.
    fn after_list() -> Vec<InternedSystemSet>;
}

/// System param to mark current system to run before a given system set.
pub struct Before<S: SystemSet + Default> {
    marker: PhantomData<S>,
}

/// System param to mark current system to run after a given system set.
pub struct After<S: SystemSet + Default> {
    marker: PhantomData<S>,
}

unsafe impl<S: SystemSet + Default> SystemParam for Before<S> {
    type State = ();
    type Item<'world, 'state> = Self;
    type BarrierList = BeforeBarrierList<S>;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        ()
    }

    unsafe fn get_param<'world, 'state>(
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        Before {
            marker: PhantomData,
        }
    }
}

unsafe impl<S: SystemSet + Default> SystemParam for After<S> {
    type State = ();
    type Item<'world, 'state> = Self;

    type BarrierList = AfterBarrierList<S>;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        ()
    }

    unsafe fn get_param<'world, 'state>(
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        After {
            marker: PhantomData,
        }
    }
}

#[doc(hidden)]
pub struct BeforeBarrierList<S: SystemSet + Default>(PhantomData<S>);
#[doc(hidden)]
pub struct AfterBarrierList<S: SystemSet + Default>(PhantomData<S>);

impl<S: SystemSet + Default> BarrierList for BeforeBarrierList<S> {
    fn before_list() -> Vec<InternedSystemSet> {
        vec![S::default().intern()]
    }

    fn after_list() -> Vec<InternedSystemSet> {
        Vec::new()
    }
}

impl<S: SystemSet + Default> BarrierList for AfterBarrierList<S> {
    fn before_list() -> Vec<InternedSystemSet> {
        Vec::new()
    }

    fn after_list() -> Vec<InternedSystemSet> {
        vec![S::default().intern()]
    }
}

macro_rules! barrier_list_for_tuple {
    ($($barrier: ident),*) => {
        impl<$($barrier),*> BarrierList for ($($barrier,)*)
        where
            $($barrier: BarrierList),*
        {
            fn before_list() -> Vec<InternedSystemSet> {
                let mut list = Vec::new();
                let _ignore_empty = &mut list;
                $(list.extend($barrier::before_list());)*
                list
            }

            fn after_list() -> Vec<InternedSystemSet> {
                let mut list = Vec::new();
                let _ignore_empty = &mut list;
                $(list.extend($barrier::after_list());)*
                list
            }
        }
    }
}

all_tuples!(barrier_list_for_tuple, 0, 16, P);
