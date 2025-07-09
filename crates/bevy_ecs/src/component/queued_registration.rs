use alloc::{boxed::Box, vec::Vec};
use bevy_platform::sync::PoisonError;
use bevy_utils::TypeIdMap;
use core::any::Any;
use core::{any::TypeId, fmt::Debug, ops::Deref};

use crate::component::{
    Component, ComponentDescriptor, ComponentId, ComponentIds, Components, ComponentsRegistrator,
    StorageType,
};
use crate::resource::Resource;

/// A queued component registration.
pub(super) struct QueuedRegistration {
    pub(super) registrator:
        Box<dyn FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor)>,
    pub(super) id: ComponentId,
    pub(super) descriptor: ComponentDescriptor,
}

impl QueuedRegistration {
    /// Creates the [`QueuedRegistration`].
    ///
    /// # Safety
    ///
    /// [`ComponentId`] must be unique.
    unsafe fn new(
        id: ComponentId,
        descriptor: ComponentDescriptor,
        func: impl FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor) + 'static,
    ) -> Self {
        Self {
            registrator: Box::new(func),
            id,
            descriptor,
        }
    }

    /// Performs the registration, returning the now valid [`ComponentId`].
    pub(super) fn register(self, registrator: &mut ComponentsRegistrator) -> ComponentId {
        (self.registrator)(registrator, self.id, self.descriptor);
        self.id
    }
}

/// Allows queuing components to be registered.
#[derive(Default)]
pub struct QueuedComponents {
    pub(super) components: TypeIdMap<QueuedRegistration>,
    pub(super) resources: TypeIdMap<QueuedRegistration>,
    pub(super) dynamic_registrations: Vec<QueuedRegistration>,
}

impl Debug for QueuedComponents {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let components = self
            .components
            .iter()
            .map(|(type_id, queued)| (type_id, queued.id))
            .collect::<Vec<_>>();
        let resources = self
            .resources
            .iter()
            .map(|(type_id, queued)| (type_id, queued.id))
            .collect::<Vec<_>>();
        let dynamic_registrations = self
            .dynamic_registrations
            .iter()
            .map(|queued| queued.id)
            .collect::<Vec<_>>();
        write!(f, "components: {components:?}, resources: {resources:?}, dynamic_registrations: {dynamic_registrations:?}")
    }
}

/// A type that enables queuing registration in [`Components`].
///
/// # Note
///
/// These queued registrations return [`ComponentId`]s.
/// These ids are not yet valid, but they will become valid
/// when either [`ComponentsRegistrator::apply_queued_registrations`] is called or the same registration is made directly.
/// In either case, the returned [`ComponentId`]s will be correct, but they are not correct yet.
///
/// Generally, that means these [`ComponentId`]s can be safely used for read-only purposes.
/// Modifying the contents of the world through these [`ComponentId`]s directly without waiting for them to be fully registered
/// and without then confirming that they have been fully registered is not supported.
/// Hence, extra care is needed with these [`ComponentId`]s to ensure all safety rules are followed.
///
/// As a rule of thumb, if you have mutable access to [`ComponentsRegistrator`], prefer to use that instead.
/// Use this only if you need to know the id of a component but do not need to modify the contents of the world based on that id.
#[derive(Clone, Copy)]
pub struct ComponentsQueuedRegistrator<'w> {
    components: &'w Components,
    ids: &'w ComponentIds,
}

impl Deref for ComponentsQueuedRegistrator<'_> {
    type Target = Components;

    fn deref(&self) -> &Self::Target {
        self.components
    }
}

impl<'w> ComponentsQueuedRegistrator<'w> {
    /// Constructs a new [`ComponentsQueuedRegistrator`].
    ///
    /// # Safety
    ///
    /// The [`Components`] and [`ComponentIds`] must match.
    /// For example, they must be from the same world.
    pub unsafe fn new(components: &'w Components, ids: &'w ComponentIds) -> Self {
        Self { components, ids }
    }

    /// Queues this function to run as a component registrator.
    ///
    /// # Safety
    ///
    /// The [`TypeId`] must not already be registered or queued as a component.
    unsafe fn force_register_arbitrary_component(
        &self,
        type_id: TypeId,
        descriptor: ComponentDescriptor,
        func: impl FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor) + 'static,
    ) -> ComponentId {
        let id = self.ids.next();
        self.components
            .queued
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .components
            .insert(
                type_id,
                // SAFETY: The id was just generated.
                unsafe { QueuedRegistration::new(id, descriptor, func) },
            );
        id
    }

    /// Queues this function to run as a resource registrator.
    ///
    /// # Safety
    ///
    /// The [`TypeId`] must not already be registered or queued as a resource.
    unsafe fn force_register_arbitrary_resource(
        &self,
        type_id: TypeId,
        descriptor: ComponentDescriptor,
        func: impl FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor) + 'static,
    ) -> ComponentId {
        let id = self.ids.next();
        self.components
            .queued
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .resources
            .insert(
                type_id,
                // SAFETY: The id was just generated.
                unsafe { QueuedRegistration::new(id, descriptor, func) },
            );
        id
    }

    /// Queues this function to run as a dynamic registrator.
    fn force_register_arbitrary_dynamic(
        &self,
        descriptor: ComponentDescriptor,
        func: impl FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor) + 'static,
    ) -> ComponentId {
        let id = self.ids.next();
        self.components
            .queued
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .dynamic_registrations
            .push(
                // SAFETY: The id was just generated.
                unsafe { QueuedRegistration::new(id, descriptor, func) },
            );
        id
    }

    /// This is a queued version of [`ComponentsRegistrator::register_component`].
    /// This will reserve an id and queue the registration.
    /// These registrations will be carried out at the next opportunity.
    ///
    /// If this has already been registered or queued, this returns the previous [`ComponentId`].
    ///
    /// # Note
    ///
    /// Technically speaking, the returned [`ComponentId`] is not valid, but it will become valid later.
    /// See type level docs for details.
    #[inline]
    pub fn queue_register_component<T: Component>(&self) -> ComponentId {
        self.component_id::<T>().unwrap_or_else(|| {
            // SAFETY: We just checked that this type was not in the queue.
            unsafe {
                self.force_register_arbitrary_component(
                    TypeId::of::<T>(),
                    ComponentDescriptor::new::<T>(),
                    |registrator, id, _descriptor| {
                        // SAFETY: We just checked that this is not currently registered or queued, and if it was registered since, this would have been dropped from the queue.
                        #[expect(unused_unsafe, reason = "More precise to specify.")]
                        unsafe {
                            registrator.register_component_unchecked::<T>(&mut Vec::new(), id);
                        }
                    },
                )
            }
        })
    }

    /// This is a queued version of [`ComponentsRegistrator::register_component_with_descriptor`].
    /// This will reserve an id and queue the registration.
    /// These registrations will be carried out at the next opportunity.
    ///
    /// # Note
    ///
    /// Technically speaking, the returned [`ComponentId`] is not valid, but it will become valid later.
    /// See type level docs for details.
    #[inline]
    pub fn queue_register_component_with_descriptor(
        &self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        self.force_register_arbitrary_dynamic(descriptor, |registrator, id, descriptor| {
            // SAFETY: Id uniqueness handled by caller.
            unsafe {
                registrator.register_component_inner(id, descriptor);
            }
        })
    }

    /// This is a queued version of [`ComponentsRegistrator::register_resource`].
    /// This will reserve an id and queue the registration.
    /// These registrations will be carried out at the next opportunity.
    ///
    /// If this has already been registered or queued, this returns the previous [`ComponentId`].
    ///
    /// # Note
    ///
    /// Technically speaking, the returned [`ComponentId`] is not valid, but it will become valid later.
    /// See type level docs for details.
    #[inline]
    pub fn queue_register_resource<T: Resource>(&self) -> ComponentId {
        let type_id = TypeId::of::<T>();
        self.get_resource_id(type_id).unwrap_or_else(|| {
            // SAFETY: We just checked that this type was not in the queue.
            unsafe {
                self.force_register_arbitrary_resource(
                    type_id,
                    ComponentDescriptor::new_resource::<T>(),
                    move |registrator, id, descriptor| {
                        // SAFETY: We just checked that this is not currently registered or queued, and if it was registered since, this would have been dropped from the queue.
                        // SAFETY: Id uniqueness handled by caller, and the type_id matches descriptor.
                        #[expect(unused_unsafe, reason = "More precise to specify.")]
                        unsafe {
                            registrator.register_resource_unchecked(type_id, id, descriptor);
                        }
                    },
                )
            }
        })
    }

    /// This is a queued version of [`ComponentsRegistrator::register_non_send`].
    /// This will reserve an id and queue the registration.
    /// These registrations will be carried out at the next opportunity.
    ///
    /// If this has already been registered or queued, this returns the previous [`ComponentId`].
    ///
    /// # Note
    ///
    /// Technically speaking, the returned [`ComponentId`] is not valid, but it will become valid later.
    /// See type level docs for details.
    #[inline]
    pub fn queue_register_non_send<T: Any>(&self) -> ComponentId {
        let type_id = TypeId::of::<T>();
        self.get_resource_id(type_id).unwrap_or_else(|| {
            // SAFETY: We just checked that this type was not in the queue.
            unsafe {
                self.force_register_arbitrary_resource(
                    type_id,
                    ComponentDescriptor::new_non_send::<T>(StorageType::default()),
                    move |registrator, id, descriptor| {
                        // SAFETY: We just checked that this is not currently registered or queued, and if it was registered since, this would have been dropped from the queue.
                        // SAFETY: Id uniqueness handled by caller, and the type_id matches descriptor.
                        #[expect(unused_unsafe, reason = "More precise to specify.")]
                        unsafe {
                            registrator.register_resource_unchecked(type_id, id, descriptor);
                        }
                    },
                )
            }
        })
    }

    /// This is a queued version of [`ComponentsRegistrator::register_resource_with_descriptor`].
    /// This will reserve an id and queue the registration.
    /// These registrations will be carried out at the next opportunity.
    ///
    /// # Note
    ///
    /// Technically speaking, the returned [`ComponentId`] is not valid, but it will become valid later.
    /// See type level docs for details.
    #[inline]
    pub fn queue_register_resource_with_descriptor(
        &self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        self.force_register_arbitrary_dynamic(descriptor, |registrator, id, descriptor| {
            // SAFETY: Id uniqueness handled by caller.
            unsafe {
                registrator.register_component_inner(id, descriptor);
            }
        })
    }
}
