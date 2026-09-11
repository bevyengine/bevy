use core::{any::TypeId, hash::Hasher};
use std::hash::DefaultHasher;

use bevy_ecs::{
    label::DynHash,
    schedule::{IntoScheduleConfigs, ScheduleLabel, Schedules},
    system::ScheduleSystem,
    world::World,
};

use crate::{metadata_ptr::MetadataPtr, MainScheduleOrder};

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
    fn get(&self) -> &S {
        match self {
            BeforeOrAfter::Before(s) | BeforeOrAfter::After(s) => s,
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
    // Both the typeid and the dynhash output because otherwise it's difficult to
    label_id: (TypeId, u64),

    other_id: BeforeOrAfter<(TypeId, u64)>,
}

impl StagedScheduleLabel {
    pub(crate) fn new<S: ScheduleLabel + 'static, O: ScheduleLabel + 'static>(
        label: S,
        relative_to: BeforeOrAfter<O>,
    ) -> Option<Self> {
        let label_hash = Self::hash(&label);
        let label_id = (TypeId::of::<S>(), label_hash);
        let other_hash = Self::hash(relative_to.get());
        let other_id = match &relative_to {
            BeforeOrAfter::Before(_) => BeforeOrAfter::Before((TypeId::of::<O>(), other_hash)),
            BeforeOrAfter::After(_) => BeforeOrAfter::After((TypeId::of::<O>(), other_hash)),
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

    fn hasher() -> DefaultHasher {
        DefaultHasher::new()
    }

    fn hash<H>(i: &H) -> u64
    where
        H: DynHash,
    {
        let mut hasher = Self::hasher();
        i.dyn_hash(&mut hasher);
        hasher.finish()
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
