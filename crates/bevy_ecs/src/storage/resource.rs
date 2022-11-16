use crate::archetype::ArchetypeComponentId;
use crate::component::{ComponentId, ComponentTicks, Components};
use crate::storage::{Column, SparseSet};
use bevy_ptr::{OwningPtr, Ptr, UnsafeCellDeref};
use std::cell::UnsafeCell;
use std::thread::ThreadId;
use std::mem::ManuallyDrop;

/// The type-erased backing storage and metadata for a single resource within a [`World`].
///
/// [`World`]: crate::world::World
pub struct ResourceData {
    column: ManuallyDrop<Column>,
    type_name: String,
    id: ArchetypeComponentId,
    origin_thread_id: Option<ThreadId>,
}

impl Drop for ResourceData {
    fn drop(&mut self) {
        if self.is_present() {
            self.validate_access();
        }
        // SAFETY: Drop is only called once upon dropping the ResourceData
        // and is inaccessible after this as the parent ResourceData has 
        // been dropped.
        unsafe { ManuallyDrop::drop(&mut self.column); }
    }
}

impl ResourceData {
    #[inline]
    fn validate_access(&self) {
        // Avoid aborting due to double panicking on the same thread.
        if std::thread::panicking() {
            return;
        }
        if let Some(origin_thread_id) = self.origin_thread_id {
            if origin_thread_id != std::thread::current().id() {
                // Panic in tests, as testing for aborting is nearly impossible
                panic!(
                    "Attempted to access or drop non-send resource {} from thread {:?} on a thread {:?}. This is not allowed. Aborting.",
                    self.type_name,
                    origin_thread_id,
                    std::thread::current().id()
                );
            }
        }
    }

    /// Returns true if the resource is populated.
    #[inline]
    pub fn is_present(&self) -> bool {
        !self.column.is_empty()
    }

    /// Gets the [`ArchetypeComponentId`] for the resource.
    #[inline]
    pub fn id(&self) -> ArchetypeComponentId {
        self.id
    }

    /// Gets a read-only pointer to the underlying resource, if available.
    ///
    /// # Panics
    /// This will panic if a value is present, the underlying type is
    /// `!Send`, and is not accessed from the original thread it was inserted in.
    #[inline]
    pub fn get_data(&self) -> Option<Ptr<'_>> {
        self.column.get_data(0).map(|res| {
            self.validate_access();
            res
        })
    }

    /// Gets a read-only reference to the change ticks of the underlying resource, if available.
    #[inline]
    pub fn get_ticks(&self) -> Option<&ComponentTicks> {
        self.column
            .get_ticks(0)
            // SAFETY:
            //  - This borrow's lifetime is bounded by the lifetime on self.
            //  - A read-only borrow on self can only exist while a mutable borrow doesn't
            //    exist.
            .map(|ticks| unsafe { ticks.deref() })
    }

    /// # Panics
    /// This will panic if a value is present, the underlying type is
    /// `!Send`, and is not accessed from the original thread it was inserted in.
    #[inline]
    pub(crate) fn get_with_ticks(&self) -> Option<(Ptr<'_>, &UnsafeCell<ComponentTicks>)> {
        self.column.get(0).map(|res| {
            self.validate_access();
            res
        })
    }

    /// Inserts a value into the resource. If a value is already present
    /// it will be replaced.
    /// 
    /// # Panics
    /// This will panic if a value is present, the underlying type is
    /// `!Send`, and is not accessed from the original thread it was inserted in.
    ///
    /// # Safety
    /// `value` must be valid for the underlying type for the resource.
    #[inline]
    pub(crate) unsafe fn insert(&mut self, value: OwningPtr<'_>, change_tick: u32) {
        if self.is_present() {
            self.validate_access();
            self.column.replace(0, value, change_tick);
        } else {
            self.origin_thread_id = self.origin_thread_id.map(|_| std::thread::current().id());
            self.column.push(value, ComponentTicks::new(change_tick));
        }
    }

    /// Inserts a value into the resource with a pre-existing change tick. If a
    /// value is already present it will be replaced.
    /// 
    /// # Panics
    /// This will panic if a value is present, the underlying type is
    /// `!Send`, and is not accessed from the original thread it was inserted in.
    ///
    /// # Safety
    /// `value` must be valid for the underlying type for the resource.
    #[inline]
    pub(crate) unsafe fn insert_with_ticks(
        &mut self,
        value: OwningPtr<'_>,
        change_ticks: ComponentTicks,
    ) {
        if self.is_present() {
            self.validate_access();
            self.column.replace_untracked(0, value);
            *self.column.get_ticks_unchecked(0).deref_mut() = change_ticks;
        } else {
            self.origin_thread_id = self.origin_thread_id.map(|_| std::thread::current().id());
            self.column.push(value, change_ticks);
        }
    }

    /// Removes a value from the resource, if present.
    /// 
    /// # Panics
    /// This will panic if a value is present, the underlying type is
    /// `!Send`, and is not accessed from the original thread it was inserted in.
    ///
    /// # Safety
    /// The underlying type must be [`Send`] or be removed from the thread it was
    /// inserted from.
    ///
    /// The removed value must be used or dropped.
    #[inline]
    #[must_use = "The returned pointer to the removed component should be used or dropped"]
    pub(crate) unsafe fn remove(&mut self) -> Option<(OwningPtr<'_>, ComponentTicks)> {
        self.is_present()
            .then(|| self.validate_access())
            .and_then(|_| self.column.swap_remove_and_forget(0))
    }

    /// Removes a value from the resource, if present, and drops it.
    /// 
    /// # Panics
    /// This will panic if a value is present, the underlying type is
    /// `!Send`, and is not accessed from the original thread it was inserted in.
    ///
    /// # Safety
    /// The underlying type must be [`Send`] or be removed from the thread it was
    /// inserted from.
    #[inline]
    pub(crate) unsafe fn remove_and_drop(&mut self) {
        if self.is_present() {
            self.validate_access();
            self.column.clear();
        }
    }
}

/// The backing store for all [`Resource`]s stored in the [`World`].
///
/// [`Resource`]: crate::system::Resource
/// [`World`]: crate::world::World
#[derive(Default)]
pub struct Resources {
    resources: SparseSet<ComponentId, ResourceData>,
}

impl Resources {
    /// The total number of resources stored in the [`World`]
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn len(&self) -> usize {
        self.resources.len()
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
    pub fn get(&self, component_id: ComponentId) -> Option<&ResourceData> {
        self.resources.get(component_id)
    }

    /// Gets mutable access to a resource, if it exists.
    #[inline]
    pub(crate) fn get_mut(&mut self, component_id: ComponentId) -> Option<&mut ResourceData> {
        self.resources.get_mut(component_id)
    }

    /// Fetches or initializes a new resource and returns back it's underlying column.
    ///
    /// # Panics
    /// Will panic if `component_id` is not valid for the provided `components`
    ///
    /// # Safety
    /// `is_send` must be accurate for the Resource that is being initialized.
    pub(crate) unsafe fn initialize_with(
        &mut self,
        component_id: ComponentId,
        components: &Components,
        is_send: bool,
        f: impl FnOnce() -> ArchetypeComponentId,
    ) -> &mut ResourceData {
        self.resources.get_or_insert_with(component_id, || {
            let component_info = components.get_info(component_id).unwrap();
            ResourceData {
                column: ManuallyDrop::new(Column::with_capacity(component_info, 1)),
                type_name: String::from(component_info.name()),
                id: f(),
                origin_thread_id: (!is_send).then(|| std::thread::current().id()),
            }
        })
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        for info in self.resources.values_mut() {
            info.column.check_change_ticks(change_tick);
        }
    }
}
