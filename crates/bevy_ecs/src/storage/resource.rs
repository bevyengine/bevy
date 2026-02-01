use crate::{
    change_detection::{
        CheckChangeTicks, ComponentTickCells, ComponentTicks, ComponentTicksMut, MaybeLocation,
        MutUntyped, Tick,
    },
    component::{ComponentId, Components},
    storage::{blob_array::BlobArray, SparseSet},
};
use bevy_ptr::{OwningPtr, Ptr, UnsafeCellDeref};
use bevy_utils::prelude::DebugName;
use core::{cell::UnsafeCell, panic::Location};

#[cfg(feature = "std")]
use std::thread::ThreadId;

/// The type-erased backing storage and metadata for a single resource within a [`World`].
///
/// If `SEND` is false, values of this type will panic if dropped from a different thread.
///
/// [`World`]: crate::world::World
pub struct ResourceData<const SEND: bool> {
    /// Capacity is 1, length is 1 if `is_present` and 0 otherwise.
    data: BlobArray,
    is_present: bool,
    added_ticks: UnsafeCell<Tick>,
    changed_ticks: UnsafeCell<Tick>,
    #[cfg_attr(
        not(feature = "std"),
        expect(dead_code, reason = "currently only used with the std feature")
    )]
    type_name: DebugName,
    #[cfg(feature = "std")]
    origin_thread_id: Option<ThreadId>,
    changed_by: MaybeLocation<UnsafeCell<&'static Location<'static>>>,
}

impl<const SEND: bool> Drop for ResourceData<SEND> {
    fn drop(&mut self) {
        // For Non Send resources we need to validate that correct thread
        // is dropping the resource. This validation is not needed in case
        // of SEND resources. Or if there is no data.
        if !SEND && self.is_present() {
            // If this thread is already panicking, panicking again will cause
            // the entire process to abort. In this case we choose to avoid
            // dropping or checking this altogether and just leak the column.
            #[cfg(feature = "std")]
            if std::thread::panicking() {
                return;
            }
            self.validate_access();
        }
        // SAFETY: Drop is only called once upon dropping the ResourceData
        // and is inaccessible after this as the parent ResourceData has
        // been dropped. The validate_access call above will check that the
        // data is dropped on the thread it was inserted from.
        unsafe {
            self.data.drop(1, self.is_present().into());
        }
    }
}

impl<const SEND: bool> ResourceData<SEND> {
    /// The only row in the underlying `BlobArray`.
    const ROW: usize = 0;

    /// Validates the access to `!Send` resources is only done on the thread they were created from.
    ///
    /// # Panics
    /// If `SEND` is false, this will panic if called from a different thread than the one it was inserted from.
    #[inline]
    fn validate_access(&self) {
        if !SEND {
            #[cfg(feature = "std")]
            if self.origin_thread_id != Some(std::thread::current().id()) {
                // Panic in tests, as testing for aborting is nearly impossible
                panic!(
                    "Attempted to access or drop non-send resource {} from thread {:?} on a thread {:?}. This is not allowed. Aborting.",
                    self.type_name,
                    self.origin_thread_id,
                    std::thread::current().id()
                );
            }

            // TODO: Handle no_std non-send.
            // Currently, no_std is single-threaded only, so this is safe to ignore.
            // To support no_std multithreading, an alternative will be required.
            // Remove the #[expect] attribute above when this is addressed.
        }
    }

    /// Returns true if the resource is populated.
    #[inline]
    pub fn is_present(&self) -> bool {
        self.is_present
    }

    /// Returns a reference to the resource, if it exists.
    ///
    /// # Panics
    /// If `SEND` is false, this will panic if a value is present and is not accessed from the
    /// original thread it was inserted from.
    #[inline]
    pub fn get_data(&self) -> Option<Ptr<'_>> {
        self.is_present().then(|| {
            self.validate_access();
            // SAFETY: We've already checked if a value is present, and there should only be one.
            unsafe { self.data.get_unchecked(Self::ROW) }
        })
    }

    /// Returns a reference to the resource's change ticks, if it exists.
    #[inline]
    pub fn get_ticks(&self) -> Option<ComponentTicks> {
        // SAFETY: This is being fetched through a read-only reference to Self, so no other mutable references
        // to the ticks can exist.
        unsafe {
            self.is_present().then(|| ComponentTicks {
                added: self.added_ticks.read(),
                changed: self.changed_ticks.read(),
            })
        }
    }

    /// Returns references to the resource and its change ticks, if it exists.
    ///
    /// # Panics
    /// If `SEND` is false, this will panic if a value is present and is not accessed from the
    /// original thread it was inserted in.
    #[inline]
    pub(crate) fn get_with_ticks(&self) -> Option<(Ptr<'_>, ComponentTickCells<'_>)> {
        self.is_present().then(|| {
            self.validate_access();
            (
                // SAFETY: We've already checked if a value is present, and there should only be one.
                unsafe { self.data.get_unchecked(Self::ROW) },
                ComponentTickCells {
                    added: &self.added_ticks,
                    changed: &self.changed_ticks,
                    changed_by: self.changed_by.as_ref(),
                },
            )
        })
    }

    /// Returns a mutable reference to the resource, if it exists.
    ///
    /// # Panics
    /// If `SEND` is false, this will panic if a value is present and is not accessed from the
    /// original thread it was inserted in.
    pub(crate) fn get_mut(&mut self, last_run: Tick, this_run: Tick) -> Option<MutUntyped<'_>> {
        let (ptr, ticks) = self.get_with_ticks()?;
        Some(MutUntyped {
            // SAFETY: We have exclusive access to the underlying storage.
            value: unsafe { ptr.assert_unique() },
            // SAFETY: We have exclusive access to the underlying storage.
            ticks: unsafe { ComponentTicksMut::from_tick_cells(ticks, last_run, this_run) },
        })
    }

    /// Inserts a value into the resource. If a value is already present
    /// it will be replaced.
    ///
    /// # Panics
    /// If `SEND` is false, this will panic if a value is present and is not replaced from
    /// the original thread it was inserted in.
    ///
    /// # Safety
    /// - `value` must be valid for the underlying type for the resource.
    #[inline]
    pub(crate) unsafe fn insert(
        &mut self,
        value: OwningPtr<'_>,
        change_tick: Tick,
        caller: MaybeLocation,
    ) {
        if self.is_present() {
            self.validate_access();
            // SAFETY: The caller ensures that the provided value is valid for the underlying type and
            // is properly initialized. We've ensured that a value is already present and previously
            // initialized.
            unsafe { self.data.replace_unchecked(Self::ROW, value) };
        } else {
            #[cfg(feature = "std")]
            if !SEND {
                self.origin_thread_id = Some(std::thread::current().id());
            }
            // SAFETY:
            // - There is only one element, and it's always allocated.
            // - The caller guarantees must be valid for the underlying type and thus its
            //   layout must be identical.
            // - The value was previously not present and thus must not have been initialized.
            unsafe { self.data.initialize_unchecked(Self::ROW, value) };
            *self.added_ticks.deref_mut() = change_tick;
            self.is_present = true;
        }
        *self.changed_ticks.deref_mut() = change_tick;

        self.changed_by
            .as_ref()
            .map(|changed_by| changed_by.deref_mut())
            .assign(caller);
    }

    /// Inserts a value into the resource with a pre-existing change tick. If a
    /// value is already present it will be replaced.
    ///
    /// # Panics
    /// If `SEND` is false, this will panic if a value is present and is not replaced from
    /// the original thread it was inserted in.
    ///
    /// # Safety
    /// - `value` must be valid for the underlying type for the resource.
    #[inline]
    pub(crate) unsafe fn insert_with_ticks(
        &mut self,
        value: OwningPtr<'_>,
        change_ticks: ComponentTicks,
        caller: MaybeLocation,
    ) {
        if self.is_present() {
            self.validate_access();
            // SAFETY: The caller ensures that the provided value is valid for the underlying type and
            // is properly initialized. We've ensured that a value is already present and previously
            // initialized.
            unsafe { self.data.replace_unchecked(Self::ROW, value) };
        } else {
            #[cfg(feature = "std")]
            if !SEND {
                self.origin_thread_id = Some(std::thread::current().id());
            }
            // SAFETY:
            // - There is only one element, and it's always allocated.
            // - The caller guarantees must be valid for the underlying type and thus its
            //   layout must be identical.
            // - The value was previously not present and thus must not have been initialized.
            unsafe { self.data.initialize_unchecked(Self::ROW, value) };
            self.is_present = true;
        }
        *self.added_ticks.deref_mut() = change_ticks.added;
        *self.changed_ticks.deref_mut() = change_ticks.changed;
        self.changed_by
            .as_ref()
            .map(|changed_by| changed_by.deref_mut())
            .assign(caller);
    }

    /// Removes a value from the resource, if present.
    ///
    /// # Panics
    /// If `SEND` is false, this will panic if a value is present and is not removed from the
    /// original thread it was inserted from.
    #[inline]
    #[must_use = "The returned pointer to the removed component should be used or dropped"]
    pub(crate) fn remove(&mut self) -> Option<(OwningPtr<'_>, ComponentTicks, MaybeLocation)> {
        if !self.is_present() {
            return None;
        }
        if !SEND {
            self.validate_access();
        }

        self.is_present = false;

        // SAFETY:
        // - There is always only one row in the `BlobArray` created during initialization.
        // - This function has validated that the row is present with the check of `self.is_present`.
        // - The caller is to take ownership of the value, returned as a `OwningPtr`.
        let res = unsafe { self.data.get_unchecked_mut(Self::ROW).promote() };

        let caller = self
            .changed_by
            .as_ref()
            // SAFETY: This function is being called through an exclusive mutable reference to Self
            .map(|changed_by| unsafe { *changed_by.deref_mut() });

        // SAFETY: This function is being called through an exclusive mutable reference to Self, which
        // makes it sound to read these ticks.
        unsafe {
            Some((
                res,
                ComponentTicks {
                    added: self.added_ticks.read(),
                    changed: self.changed_ticks.read(),
                },
                caller,
            ))
        }
    }

    /// Removes a value from the resource, if present, and drops it.
    ///
    /// # Panics
    /// If `SEND` is false, this will panic if a value is present and is not
    /// accessed from the original thread it was inserted in.
    #[inline]
    pub(crate) fn remove_and_drop(&mut self) {
        if self.is_present() {
            self.validate_access();
            // SAFETY: There is only one element, and it's always allocated.
            unsafe { self.data.drop_last_element(Self::ROW) };
            self.is_present = false;
        }
    }

    pub(crate) fn check_change_ticks(&mut self, check: CheckChangeTicks) {
        self.added_ticks.get_mut().check_tick(check);
        self.changed_ticks.get_mut().check_tick(check);
    }
}

/// The backing store for all [`Resource`]s stored in the [`World`].
///
/// [`Resource`]: crate::resource::Resource
/// [`World`]: crate::world::World
#[derive(Default)]
pub struct Resources<const SEND: bool> {
    resources: SparseSet<ComponentId, ResourceData<SEND>>,
}

impl<const SEND: bool> Resources<SEND> {
    /// The total number of resources stored in the [`World`]
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Iterate over all resources that have been initialized, i.e. given a [`ComponentId`]
    pub fn iter(&self) -> impl Iterator<Item = (ComponentId, &ResourceData<SEND>)> {
        self.resources.iter().map(|(id, data)| (*id, data))
    }

    /// Returns true if there are no resources stored in the [`World`],
    /// false otherwise.
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// Gets read-only access to a resource, if it exists.
    #[inline]
    pub fn get(&self, component_id: ComponentId) -> Option<&ResourceData<SEND>> {
        self.resources.get(component_id)
    }

    /// Clears all resources.
    #[inline]
    pub fn clear(&mut self) {
        self.resources.clear();
    }

    /// Gets mutable access to a resource, if it exists.
    #[inline]
    pub(crate) fn get_mut(&mut self, component_id: ComponentId) -> Option<&mut ResourceData<SEND>> {
        self.resources.get_mut(component_id)
    }

    /// Fetches or initializes a new resource and returns back its underlying column.
    ///
    /// # Panics
    /// Will panic if `component_id` is not valid for the provided `components`
    /// If `SEND` is true, this will panic if `component_id`'s `ComponentInfo` is not registered as being `Send` + `Sync`.
    pub(crate) fn initialize_with(
        &mut self,
        component_id: ComponentId,
        components: &Components,
    ) -> &mut ResourceData<SEND> {
        self.resources.get_or_insert_with(component_id, || {
            let component_info = components.get_info(component_id).unwrap();
            if SEND {
                assert!(
                    component_info.is_send_and_sync(),
                    "Send + Sync resource {} initialized as non_send. It may have been inserted via World::insert_non_send_resource by accident. Try using World::insert_resource instead.",
                    component_info.name(),
                );
            }
            // SAFETY: component_info.drop() is valid for the types that will be inserted.
            let data = unsafe {
                BlobArray::with_capacity(
                    component_info.layout(),
                    component_info.drop(),
                    1
                )
            };
            ResourceData {
                data,
                is_present: false,
                added_ticks: UnsafeCell::new(Tick::new(0)),
                changed_ticks: UnsafeCell::new(Tick::new(0)),
                type_name: component_info.name(),
                #[cfg(feature = "std")]
                origin_thread_id: None,
                changed_by: MaybeLocation::caller().map(UnsafeCell::new),
            }
        })
    }

    pub(crate) fn check_change_ticks(&mut self, check: CheckChangeTicks) {
        for info in self.resources.values_mut() {
            info.check_change_ticks(check);
        }
    }
}
