use alloc::{boxed::Box, vec::Vec};
use bevy_platform::sync::PoisonError;
use bevy_utils::TypeIdMap;
use core::any::Any;
use core::{any::TypeId, fmt::Debug, ops::Deref};

use crate::component::{enforce_no_required_components_recursion, RequiredComponentsRegistrator};
use crate::{
    component::{
        Component, ComponentDescriptor, ComponentId, Components, RequiredComponents, StorageType,
    },
    query::DebugCheckedUnwrap as _,
    resource::Resource,
};

/// Generates [`ComponentId`]s.
#[derive(Debug, Default)]
pub struct ComponentIds {
    next: bevy_platform::sync::atomic::AtomicUsize,
}

impl ComponentIds {
    /// Peeks the next [`ComponentId`] to be generated without generating it.
    pub fn peek(&self) -> ComponentId {
        ComponentId(
            self.next
                .load(bevy_platform::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Generates and returns the next [`ComponentId`].
    pub fn next(&self) -> ComponentId {
        ComponentId(
            self.next
                .fetch_add(1, bevy_platform::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Peeks the next [`ComponentId`] to be generated without generating it.
    pub fn peek_mut(&mut self) -> ComponentId {
        ComponentId(*self.next.get_mut())
    }

    /// Generates and returns the next [`ComponentId`].
    pub fn next_mut(&mut self) -> ComponentId {
        let id = self.next.get_mut();
        let result = ComponentId(*id);
        *id += 1;
        result
    }

    /// Returns the number of [`ComponentId`]s generated.
    pub fn len(&self) -> usize {
        self.peek().0
    }

    /// Returns true if and only if no ids have been generated.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A [`Components`] wrapper that enables additional features, like registration.
pub struct ComponentsRegistrator<'w> {
    pub(super) components: &'w mut Components,
    pub(super) ids: &'w mut ComponentIds,
    pub(super) recursion_check_stack: Vec<ComponentId>,
}

impl Deref for ComponentsRegistrator<'_> {
    type Target = Components;

    fn deref(&self) -> &Self::Target {
        self.components
    }
}

impl<'w> ComponentsRegistrator<'w> {
    /// Constructs a new [`ComponentsRegistrator`].
    ///
    /// # Safety
    ///
    /// The [`Components`] and [`ComponentIds`] must match.
    /// For example, they must be from the same world.
    pub unsafe fn new(components: &'w mut Components, ids: &'w mut ComponentIds) -> Self {
        Self {
            components,
            ids,
            recursion_check_stack: Vec::new(),
        }
    }

    /// Converts this [`ComponentsRegistrator`] into a [`ComponentsQueuedRegistrator`].
    /// This is intended for use to pass this value to a function that requires [`ComponentsQueuedRegistrator`].
    /// It is generally not a good idea to queue a registration when you can instead register directly on this type.
    pub fn as_queued(&self) -> ComponentsQueuedRegistrator<'_> {
        // SAFETY: ensured by the caller that created self.
        unsafe { ComponentsQueuedRegistrator::new(self.components, self.ids) }
    }

    /// Applies every queued registration.
    /// This ensures that every valid [`ComponentId`] is registered,
    /// enabling retrieving [`ComponentInfo`](super::ComponentInfo), etc.
    pub fn apply_queued_registrations(&mut self) {
        if !self.any_queued_mut() {
            return;
        }

        // Note:
        //
        // This is not just draining the queue. We need to empty the queue without removing the information from `Components`.
        // If we drained directly, we could break invariance.
        //
        // For example, say `ComponentA` and `ComponentB` are queued, and `ComponentA` requires `ComponentB`.
        // If we drain directly, and `ComponentA` was the first to be registered, then, when `ComponentA`
        // registers `ComponentB` in `Component::register_required_components`,
        // `Components` will not know that `ComponentB` was queued
        // (since it will have been drained from the queue.)
        // If that happened, `Components` would assign a new `ComponentId` to `ComponentB`
        // which would be *different* than the id it was assigned in the queue.
        // Then, when the drain iterator gets to `ComponentB`,
        // it would be unsafely registering `ComponentB`, which is already registered.
        //
        // As a result, we need to pop from each queue one by one instead of draining.

        // components
        while let Some(registrator) = {
            let queued = self
                .components
                .queued
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner);
            queued.components.keys().next().copied().map(|type_id| {
                // SAFETY: the id just came from a valid iterator.
                unsafe { queued.components.remove(&type_id).debug_checked_unwrap() }
            })
        } {
            registrator.register(self);
        }

        // resources
        while let Some(registrator) = {
            let queued = self
                .components
                .queued
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner);
            queued.resources.keys().next().copied().map(|type_id| {
                // SAFETY: the id just came from a valid iterator.
                unsafe { queued.resources.remove(&type_id).debug_checked_unwrap() }
            })
        } {
            registrator.register(self);
        }

        // dynamic
        let queued = &mut self
            .components
            .queued
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        if !queued.dynamic_registrations.is_empty() {
            for registrator in core::mem::take(&mut queued.dynamic_registrations) {
                registrator.register(self);
            }
        }
    }

    /// Registers a [`Component`] of type `T` with this instance.
    /// If a component of this type has already been registered, this will return
    /// the ID of the pre-existing component.
    ///
    /// # See also
    ///
    /// * [`Components::component_id()`]
    /// * [`ComponentsRegistrator::register_component_with_descriptor()`]
    #[inline]
    pub fn register_component<T: Component>(&mut self) -> ComponentId {
        self.register_component_checked::<T>()
    }

    /// Same as [`Self::register_component_unchecked`] but keeps a checks for safety.
    #[inline]
    pub(super) fn register_component_checked<T: Component>(&mut self) -> ComponentId {
        let type_id = TypeId::of::<T>();
        if let Some(&id) = self.indices.get(&type_id) {
            enforce_no_required_components_recursion(self, &self.recursion_check_stack, id);
            return id;
        }

        if let Some(registrator) = self
            .components
            .queued
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .components
            .remove(&type_id)
        {
            // If we are trying to register something that has already been queued, we respect the queue.
            // Just like if we are trying to register something that already is, we respect the first registration.
            return registrator.register(self);
        }

        let id = self.ids.next_mut();
        // SAFETY: The component is not currently registered, and the id is fresh.
        unsafe {
            self.register_component_unchecked::<T>(id);
        }
        id
    }

    /// # Safety
    ///
    /// Neither this component, nor its id may be registered or queued. This must be a new registration.
    #[inline]
    unsafe fn register_component_unchecked<T: Component>(&mut self, id: ComponentId) {
        // SAFETY: ensured by caller.
        unsafe {
            self.components
                .register_component_inner(id, ComponentDescriptor::new::<T>());
        }
        let type_id = TypeId::of::<T>();
        let prev = self.components.indices.insert(type_id, id);
        debug_assert!(prev.is_none());

        self.recursion_check_stack.push(id);
        let mut required_components = RequiredComponents::default();
        // SAFETY: `required_components` is empty
        let mut required_components_registrator =
            unsafe { RequiredComponentsRegistrator::new(self, &mut required_components) };
        T::register_required_components(id, &mut required_components_registrator);
        // SAFETY:
        // - `id` was just registered in `self`
        // - RequiredComponentsRegistrator guarantees that only components from `self` are included in `required_components`;
        // - we just initialized the component with id `id` so no component requiring it can exist yet.
        unsafe {
            self.components
                .register_required_by(id, &required_components);
        }
        self.recursion_check_stack.pop();

        // SAFETY: we just inserted it in `register_component_inner`
        let info = unsafe {
            &mut self
                .components
                .components
                .get_mut(id.0)
                .debug_checked_unwrap()
                .as_mut()
                .debug_checked_unwrap()
        };

        info.hooks.update_from_component::<T>();

        info.required_components = required_components;
    }

    /// Registers a component described by `descriptor`.
    ///
    /// # Note
    ///
    /// If this method is called multiple times with identical descriptors, a distinct [`ComponentId`]
    /// will be created for each one.
    ///
    /// # See also
    ///
    /// * [`Components::component_id()`]
    /// * [`ComponentsRegistrator::register_component()`]
    #[inline]
    pub fn register_component_with_descriptor(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        let id = self.ids.next_mut();
        // SAFETY: The id is fresh.
        unsafe {
            self.components.register_component_inner(id, descriptor);
        }
        id
    }

    /// Registers a [`Resource`] of type `T` with this instance.
    /// If a resource of this type has already been registered, this will return
    /// the ID of the pre-existing resource.
    ///
    /// # See also
    ///
    /// * [`Components::resource_id()`]
    /// * [`ComponentsRegistrator::register_resource_with_descriptor()`]
    #[inline]
    pub fn register_resource<T: Resource>(&mut self) -> ComponentId {
        // SAFETY: The [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.register_resource_with(TypeId::of::<T>(), || {
                ComponentDescriptor::new_resource::<T>()
            })
        }
    }

    /// Registers a [non-send resource](crate::system::NonSend) of type `T` with this instance.
    /// If a resource of this type has already been registered, this will return
    /// the ID of the pre-existing resource.
    #[inline]
    pub fn register_non_send<T: Any>(&mut self) -> ComponentId {
        // SAFETY: The [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.register_resource_with(TypeId::of::<T>(), || {
                ComponentDescriptor::new_non_send::<T>(StorageType::default())
            })
        }
    }

    /// Same as [`Components::register_resource_unchecked`] but handles safety.
    ///
    /// # Safety
    ///
    /// The [`ComponentDescriptor`] must match the [`TypeId`].
    #[inline]
    unsafe fn register_resource_with(
        &mut self,
        type_id: TypeId,
        descriptor: impl FnOnce() -> ComponentDescriptor,
    ) -> ComponentId {
        if let Some(id) = self.resource_indices.get(&type_id) {
            return *id;
        }

        if let Some(registrator) = self
            .components
            .queued
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .resources
            .remove(&type_id)
        {
            // If we are trying to register something that has already been queued, we respect the queue.
            // Just like if we are trying to register something that already is, we respect the first registration.
            return registrator.register(self);
        }

        let id = self.ids.next_mut();
        // SAFETY: The resource is not currently registered, the id is fresh, and the [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.components
                .register_resource_unchecked(type_id, id, descriptor());
        }
        id
    }

    /// Registers a [`Resource`] described by `descriptor`.
    ///
    /// # Note
    ///
    /// If this method is called multiple times with identical descriptors, a distinct [`ComponentId`]
    /// will be created for each one.
    ///
    /// # See also
    ///
    /// * [`Components::resource_id()`]
    /// * [`ComponentsRegistrator::register_resource()`]
    #[inline]
    pub fn register_resource_with_descriptor(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        let id = self.ids.next_mut();
        // SAFETY: The id is fresh.
        unsafe {
            self.components.register_component_inner(id, descriptor);
        }
        id
    }

    /// Equivalent of `Components::any_queued_mut`
    pub fn any_queued_mut(&mut self) -> bool {
        self.components.any_queued_mut()
    }

    /// Equivalent of `Components::any_queued_mut`
    pub fn num_queued_mut(&mut self) -> usize {
        self.components.num_queued_mut()
    }
}

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

    /// Queues this function to run as a component registrator if the given
    /// type is not already queued as a component.
    ///
    /// # Safety
    ///
    /// The [`TypeId`] must not already be registered as a component.
    unsafe fn register_arbitrary_component(
        &self,
        type_id: TypeId,
        descriptor: ComponentDescriptor,
        func: impl FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor) + 'static,
    ) -> ComponentId {
        self.components
            .queued
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .components
            .entry(type_id)
            .or_insert_with(|| {
                // SAFETY: The id was just generated.
                unsafe { QueuedRegistration::new(self.ids.next(), descriptor, func) }
            })
            .id
    }

    /// Queues this function to run as a resource registrator if the given
    /// type is not already queued as a resource.
    ///
    /// # Safety
    ///
    /// The [`TypeId`] must not already be registered as a resource.
    unsafe fn register_arbitrary_resource(
        &self,
        type_id: TypeId,
        descriptor: ComponentDescriptor,
        func: impl FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor) + 'static,
    ) -> ComponentId {
        self.components
            .queued
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .resources
            .entry(type_id)
            .or_insert_with(|| {
                // SAFETY: The id was just generated.
                unsafe { QueuedRegistration::new(self.ids.next(), descriptor, func) }
            })
            .id
    }

    /// Queues this function to run as a dynamic registrator.
    fn register_arbitrary_dynamic(
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
            // SAFETY: We just checked that this type was not already registered.
            unsafe {
                self.register_arbitrary_component(
                    TypeId::of::<T>(),
                    ComponentDescriptor::new::<T>(),
                    |registrator, id, _descriptor| {
                        // SAFETY: We just checked that this is not currently registered or queued, and if it was registered since, this would have been dropped from the queue.
                        #[expect(unused_unsafe, reason = "More precise to specify.")]
                        unsafe {
                            registrator.register_component_unchecked::<T>(id);
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
        self.register_arbitrary_dynamic(descriptor, |registrator, id, descriptor| {
            // SAFETY: Id uniqueness handled by caller.
            unsafe {
                registrator
                    .components
                    .register_component_inner(id, descriptor);
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
            // SAFETY: We just checked that this type was not already registered.
            unsafe {
                self.register_arbitrary_resource(
                    type_id,
                    ComponentDescriptor::new_resource::<T>(),
                    move |registrator, id, descriptor| {
                        // SAFETY: We just checked that this is not currently registered or queued, and if it was registered since, this would have been dropped from the queue.
                        // SAFETY: Id uniqueness handled by caller, and the type_id matches descriptor.
                        #[expect(unused_unsafe, reason = "More precise to specify.")]
                        unsafe {
                            registrator
                                .components
                                .register_resource_unchecked(type_id, id, descriptor);
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
            // SAFETY: We just checked that this type was not already registered.
            unsafe {
                self.register_arbitrary_resource(
                    type_id,
                    ComponentDescriptor::new_non_send::<T>(StorageType::default()),
                    move |registrator, id, descriptor| {
                        // SAFETY: We just checked that this is not currently registered or queued, and if it was registered since, this would have been dropped from the queue.
                        // SAFETY: Id uniqueness handled by caller, and the type_id matches descriptor.
                        #[expect(unused_unsafe, reason = "More precise to specify.")]
                        unsafe {
                            registrator
                                .components
                                .register_resource_unchecked(type_id, id, descriptor);
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
        self.register_arbitrary_dynamic(descriptor, |registrator, id, descriptor| {
            // SAFETY: Id uniqueness handled by caller.
            unsafe {
                registrator
                    .components
                    .register_component_inner(id, descriptor);
            }
        })
    }
}
