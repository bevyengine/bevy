use alloc::boxed::Box;
use bevy_ecs::{
    schedule::{self, IntoScheduleConfigs, ScheduleLabel, Schedules},
    system::{IntoSystem, ScheduleSystem},
    world::World,
};

use crate::metadata_ptr::MetadataPtr;

pub(crate) struct ErasedSchedule(MetadataPtr);

impl ErasedSchedule {
    pub fn new<F: 'static>(schedule: F) -> Option<Self> {
        Some(Self(MetadataPtr::new(schedule)?))
    }
}

pub(crate) struct StagedSystem {
    pub(crate) staging: Staged<ErasedSchedule>,
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
                erased: ErasedSchedule::new(pair)?,
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
