use bevy_ecs::{
    message::Message,
    observer::{IntoObserver, Observer},
    resource::Resource,
    schedule::{IntoScheduleConfigs, ScheduleLabel, Schedules},
    system::ScheduleSystem,
    world::World,
};
use bevy_platform::collections::{HashMap, HashSet};
use core::{
    alloc::Layout,
    any::{Any, TypeId},
    hash::Hash,
    ptr::NonNull,
};
use std::{
    alloc::{alloc, dealloc},
    boxed::Box,
    collections::VecDeque,
    vec::Vec,
};

use crate::{approval::Approval, metadata_ptr::MetadataPtr};

/// A type erased [`Resource`], implemented using a [`MetadataPtr`]. This is
/// necessary due to [`Resource`] not being dyn compatible.
pub(crate) struct ErasedResource(pub(crate) MetadataPtr);

/// Data structure to get around the fact that Resource is not dyn compatible.
pub(crate) struct StagedResource {
    /// The type-erased resource.
    pub(crate) erased_resource: ErasedResource,
    /// Function that un-erases the resource and adds it to a given world.
    pub(crate) unerase_and_insert: Box<dyn Fn(&mut World, ErasedResource)>,
    /// Function that un-erases the resource and check if it meets the plugin's requirements.
    pub(crate) approval_from_plugin: Approval<ErasedResource>,
    /// If the staged resource is a default, we care about it less than any given
    pub(crate) is_default: bool,
}

impl StagedResource {
    pub(crate) fn new<R: Resource>(
        resource: R,
        is_default: bool,
        check_ok: impl Fn(&R) -> bool + 'static,
    ) -> Option<Self> {
        Some(StagedResource {
            erased_resource: ErasedResource(MetadataPtr::new(resource)?),
            unerase_and_insert: Box::new(|world, erased| match erased.0.try_reverse_erase::<R>() {
                Ok(resource) => world.insert_resource(resource),
                Err(_) => {}
            }),
            approval_from_plugin: Approval::new(move |erased: &ErasedResource| {
                erased
                    .0
                    .visit::<R, _>(|data| check_ok(data))
                    // If the types didn't match, we should never veto (it's not this type's problem).
                    .unwrap_or(true)
            }),
            is_default,
        })
    }
}
