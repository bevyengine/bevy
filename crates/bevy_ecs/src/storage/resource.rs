use crate::archetype::ArchetypeComponentId;
use crate::change_detection::{MutUntyped, TicksMut};
use crate::component::{ComponentId, ComponentTicks, Components, Tick, TickCells};
use crate::query::DebugCheckedUnwrap;
use crate::storage::{blob_vec::BlobBox, SparseArray};
use bevy_ptr::{OwningPtr, Ptr, UnsafeCellDeref};
use std::cell::UnsafeCell;
use std::{mem::ManuallyDrop, thread::ThreadId};

/// The type-erased backing storage and metadata for a single resource within a [`World`].
///
/// If `SEND` is false, values of this type will panic if dropped from a different thread.
///
/// [`World`]: crate::world::World
pub struct ResourceData<const SEND: bool> {
    data: ManuallyDrop<BlobBox>,
    added_tick: UnsafeCell<Tick>,
    changed_tick: UnsafeCell<Tick>,
    type_name: String,
    id: ArchetypeComponentId,
    origin_thread_id: Option<ThreadId>,
}

impl<const SEND: bool> Drop for ResourceData<SEND> {
    fn drop(&mut self) {
        if self.is_present() {
            // If this thread is already panicking, panicking again will cause
            // the entire process to abort. In this case we choose to avoid
            // dropping or checking this altogether and just leak the column.
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
            ManuallyDrop::drop(&mut self.data);
        }
    }
}

impl<const SEND: bool> ResourceData<SEND> {
    #[inline]
    fn validate_access(&self) {
        if SEND {
            return;
        }
        if self.origin_thread_id != Some(std::thread::current().id()) {
            // Panic in tests, as testing for aborting is nearly impossible
            panic!(
                "Attempted to access or drop non-send resource {} from thread {:?} on a thread {:?}. This is not allowed. Aborting.",
                self.type_name,
                self.origin_thread_id,
                std::thread::current().id()
            );
        }
    }

    /// Returns true if the resource is populated.
    #[inline]
    pub fn is_present(&self) -> bool {
        self.data.is_present()
    }

    /// Gets the [`ArchetypeComponentId`] for the resource.
    #[inline]
    pub fn id(&self) -> ArchetypeComponentId {
        self.id
    }

    /// Gets a read-only pointer to the underlying resource, if available.
    ///
    /// # Panics
    /// If `SEND` is false, this will panic if a value is present and is not accessed from the
    /// original thread it was inserted from.
    #[inline]
    pub fn get_data(&self) -> Option<Ptr<'_>> {
        self.data.get_ptr().map(|res| {
            self.validate_access();
            res
        })
    }

    /// Gets a read-only reference to the change ticks of the underlying resource, if available.
    #[inline]
    pub fn get_ticks(&self) -> Option<ComponentTicks> {
        // SAFETY: If the data is present, the ticks have been written to with valid values
        self.is_present().then(|| unsafe {
            ComponentTicks {
                added: self.added_tick.read(),
                changed: self.changed_tick.read(),
            }
        })
    }

    /// # Panics
    /// If `SEND` is false, this will panic if a value is present and is not accessed from the
    /// original thread it was inserted in.
    #[inline]
    pub(crate) fn get_with_ticks(&self) -> Option<(Ptr<'_>, TickCells<'_>)> {
        self.data.get_ptr().map(|res| {
            self.validate_access();
            (
                res,
                TickCells {
                    added: &self.added_tick,
                    changed: &self.changed_tick,
                },
            )
        })
    }

    pub(crate) fn get_mut(
        &mut self,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Option<MutUntyped<'_>> {
        let (ptr, ticks) = self.get_with_ticks()?;
        Some(MutUntyped {
            // SAFETY: We have exclusive access to the underlying storage.
            value: unsafe { ptr.assert_unique() },
            // SAFETY: We have exclusive access to the underlying storage.
            ticks: unsafe { TicksMut::from_tick_cells(ticks, last_change_tick, change_tick) },
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
    pub(crate) unsafe fn insert(&mut self, value: OwningPtr<'_>, change_tick: u32) {
        if self.is_present() {
            self.validate_access();
            self.data.replace(value);
        } else {
            if !SEND {
                self.origin_thread_id = Some(std::thread::current().id());
            }
            self.data.initialize(value);
        }
        let tick = Tick::new(change_tick);
        *self.added_tick.get_mut() = tick;
        *self.changed_tick.get_mut() = tick;
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
    ) {
        if self.is_present() {
            self.validate_access();
            self.data.replace(value);
        } else {
            if !SEND {
                self.origin_thread_id = Some(std::thread::current().id());
            }
            self.data.initialize(value);
        }
        *self.added_tick.get_mut() = change_ticks.added;
        *self.changed_tick.get_mut() = change_ticks.changed;
    }

    /// Removes a value from the resource, if present.
    ///
    /// # Panics
    /// If `SEND` is false, this will panic if a value is present and is not removed from the
    /// original thread it was inserted from.
    #[inline]
    #[must_use = "The returned pointer to the removed component should be used or dropped"]
    pub(crate) fn remove(&mut self) -> Option<(OwningPtr<'_>, ComponentTicks)> {
        let res = if SEND {
            self.data.remove_and_forget()
        } else {
            self.is_present()
                .then(|| self.validate_access())
                .and_then(|_| self.data.remove_and_forget())
        };
        res.map(|ptr| {
            (
                ptr,
                ComponentTicks {
                    added: *self.added_tick.get_mut(),
                    changed: *self.changed_tick.get_mut(),
                },
            )
        })
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
            self.data.clear();
        }
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        self.added_tick.get_mut().check_tick(change_tick);
        self.changed_tick.get_mut().check_tick(change_tick);
    }
}

/// The backing store for all [`Resource`]s stored in the [`World`].
///
/// [`Resource`]: crate::system::Resource
/// [`World`]: crate::world::World
#[derive(Default)]
pub struct Resources<const SEND: bool> {
    component_ids: Vec<ComponentId>,
    resources: SparseArray<ComponentId, ResourceData<SEND>>,
}

impl<const SEND: bool> Resources<SEND> {
    /// The total number of resources stored in the [`World`]
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn len(&self) -> usize {
        self.component_ids.len()
    }

    /// Iterate over all resources that have been initialized, i.e. given a [`ComponentId`]
    pub fn iter(&self) -> impl Iterator<Item = (ComponentId, &ResourceData<SEND>)> {
        self.component_ids.iter().copied().map(|component_id| {
            // SAFETY: If a component ID is in component_ids, it's populated in the sparse array.
            (component_id, unsafe {
                self.resources.get(component_id).debug_checked_unwrap()
            })
        })
    }

    /// Returns true if there are no resources stored in the [`World`],
    /// false otherwise.
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.component_ids.is_empty()
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

    /// Fetches or initializes a new resource and returns back it's underlying column.
    ///
    /// # Panics
    /// Will panic if `component_id` is not valid for the provided `components`
    /// If `SEND` is false, this will panic if `component_id`'s `ComponentInfo` is not registered as being `Send` + `Sync`.
    pub(crate) fn initialize_with(
        &mut self,
        component_id: ComponentId,
        components: &Components,
        f: impl FnOnce() -> ArchetypeComponentId,
    ) -> &mut ResourceData<SEND> {
        self.resources.get_or_insert_with(component_id, || {
            let component_info = components.get_info(component_id).unwrap();
            self.component_ids.push(component_id);
            if SEND {
                assert!(component_info.is_send_and_sync());
            }
            // SAFETY: component_info.drop() is valid for the types that will be inserted.
            let data = unsafe { BlobBox::new(component_info.layout(), component_info.drop()) };
            ResourceData {
                data: ManuallyDrop::new(data),
                added_tick: UnsafeCell::new(Tick::new(0)),
                changed_tick: UnsafeCell::new(Tick::new(0)),
                type_name: String::from(component_info.name()),
                id: f(),
                origin_thread_id: None,
            }
        })
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        for component_id in &self.component_ids {
            // SAFETY: If a component ID is in component_ids, it's populated in the sparse array.
            unsafe {
                self.resources
                    .get_mut(*component_id)
                    .debug_checked_unwrap()
                    .check_change_ticks(change_tick);
            }
        }
    }
}
