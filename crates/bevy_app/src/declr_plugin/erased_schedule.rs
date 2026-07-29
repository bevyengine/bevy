use core::{any::TypeId, clone};

use alloc::boxed::Box;
use bevy_ecs::{
    schedule::{self, IntoScheduleConfigs, ScheduleLabel, Schedules},
    system::{IntoSystem, ScheduleSystem},
    world::World,
};

use crate::{metadata_ptr::MetadataPtr, MainScheduleOrder, Update};

pub(crate) struct ErasedScheduleLabel(MetadataPtr);

#[derive(Debug, Clone, Copy)]
pub(crate) enum BeforeOrAfter<S> {
    Before(S),
    After(S),
}

impl<S> BeforeOrAfter<S> {
    fn map<T>(self, f: impl FnOnce(S) -> T) -> BeforeOrAfter<T> {
        match self {
            BeforeOrAfter::Before(label) => BeforeOrAfter::Before(f(label)),
            BeforeOrAfter::After(label) => BeforeOrAfter::After(f(label)),
        }
    }
}

impl ErasedScheduleLabel {
    pub(crate) fn new<S: ScheduleLabel + 'static, O: ScheduleLabel + 'static>(
        label: S,
        relative_to: BeforeOrAfter<O>,
    ) -> Option<Self> {
        Some(Self(MetadataPtr::new((label, relative_to))?))
    }
}

pub(crate) struct StagedScheduleLabel {
    staged: Staged<ErasedScheduleLabel>,
    // we store these to be able to order how new schedule labels are registered later.
    label_id: TypeId,
    // schedules can have data (i.e. `OnEnter(State)`), so we can't rely on this and should instead use DynHash.
    other_id: BeforeOrAfter<TypeId>,
}

impl StagedScheduleLabel {
    pub(crate) fn new<S: ScheduleLabel + 'static, O: ScheduleLabel + 'static>(
        label: S,
        relative_to: BeforeOrAfter<O>,
    ) -> Option<Self> {
        let label_id = TypeId::of::<S>();
        let other_id = match &relative_to {
            BeforeOrAfter::Before(_) => BeforeOrAfter::Before(TypeId::of::<O>()),
            BeforeOrAfter::After(_) => BeforeOrAfter::After(TypeId::of::<O>()),
        };
        Some(Self {
            staged: Staged {
                erased: ErasedScheduleLabel::new(label, relative_to)?,
                unerase_and_apply_to_world: |world, erased| {
                    let mut schedule_order = world.get_resource_or_init::<MainScheduleOrder>();
                    match erased.0.try_reverse_erase::<(S, BeforeOrAfter<O>)>() {
                        Ok((label, BeforeOrAfter::Before(other_label))) => {
                            schedule_order.insert_before(other_label, label);
                        }
                        Ok((label, BeforeOrAfter::After(other_label))) => {
                            schedule_order.insert_after(other_label, label);
                        }
                        Err(_) => {}
                    }
                },
            },
            label_id,
            other_id,
        })
    }
}

pub(crate) struct ErasedScheduleSystemsPair(MetadataPtr);

impl ErasedScheduleSystemsPair {
    pub fn new<P: 'static>(pair: P) -> Option<Self> {
        Some(Self(MetadataPtr::new(pair)?))
    }
}

pub(crate) struct StagedSystem {
    pub(crate) staging: Staged<ErasedScheduleSystemsPair>,
}

impl StagedSystem {
    pub(crate) fn new<L, S, M>(label: L, systems: S) -> Option<Self>
    where
        L: ScheduleLabel + 'static,
        S: IntoScheduleConfigs<ScheduleSystem, M> + 'static,
    {
        let pair: (L, S) = (label, systems);
        Some(Self {
            staging: Staged {
                erased: ErasedScheduleSystemsPair::new(pair)?,
                unerase_and_apply_to_world: |world, erased| match erased
                    .0
                    .try_reverse_erase::<(L, S)>()
                {
                    Ok((label, systems)) => {
                        world
                            .get_resource_or_init::<Schedules>()
                            .add_systems(label, systems);
                    }
                    _ => {}
                },
            },
        })
    }
}

pub(crate) struct Staged<E> {
    pub(crate) erased: E,
    pub(crate) unerase_and_apply_to_world: fn(&mut World, E),
}

impl<E> Staged<E> {
    pub(crate) fn apply_to_world(self, world: &mut World) {
        (self.unerase_and_apply_to_world)(world, self.erased)
    }
}
