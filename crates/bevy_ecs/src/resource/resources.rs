use crate::{system::SystemId, AtomicBorrow, TypeInfo};
use bevy_utils::HashMap;
use core::any::TypeId;
use downcast_rs::{impl_downcast, Downcast};
use std::{
    cell::UnsafeCell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    thread::ThreadId,
};

/// A Resource type
pub trait Resource: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Resource for T {}

pub(crate) struct ResourceData {
    storage: Box<dyn ResourceStorage>,
    default_index: Option<usize>,
    system_id_to_archetype_index: HashMap<usize, usize>,
}

#[derive(Debug)]
pub enum ResourceIndex {
    Global,
    System(SystemId),
}

// TODO: consider using this for normal resources (would require change tracking)
trait ResourceStorage: Downcast {
    fn clear_trackers(&mut self);
}
impl_downcast!(ResourceStorage);

struct StoredResource<T: 'static> {
    value: UnsafeCell<T>,
    added: UnsafeCell<bool>,
    mutated: UnsafeCell<bool>,
    atomic_borrow: AtomicBorrow,
}

pub struct VecResourceStorage<T: 'static> {
    stored: Vec<StoredResource<T>>,
}

impl<T: 'static> VecResourceStorage<T> {
    fn get(&self, index: usize) -> Option<ResourceRef<'_, T>> {
        self.stored
            .get(index)
            .map(|stored| ResourceRef::new(stored))
    }

    fn get_mut(&self, index: usize) -> Option<ResourceRefMut<'_, T>> {
        self.stored
            .get(index)
            .map(|stored| ResourceRefMut::new(stored))
    }

    unsafe fn get_unsafe_ref(&self, index: usize) -> NonNull<T> {
        NonNull::new_unchecked(self.stored.get_unchecked(index).value.get())
    }

    fn push(&mut self, resource: T) {
        self.stored.push(StoredResource {
            atomic_borrow: AtomicBorrow::new(),
            value: UnsafeCell::new(resource),
            added: UnsafeCell::new(true),
            mutated: UnsafeCell::new(true),
        });
    }

    fn set(&mut self, index: usize, resource: T) {
        self.stored[index].value = UnsafeCell::new(resource);
        self.stored[index].mutated = UnsafeCell::new(true);
    }

    fn is_empty(&self) -> bool {
        self.stored.is_empty()
    }
}

impl<T: 'static> Default for VecResourceStorage<T> {
    fn default() -> Self {
        Self {
            stored: Default::default(),
        }
    }
}

impl<T: 'static> ResourceStorage for VecResourceStorage<T> {
    fn clear_trackers(&mut self) {
        for stored in &mut self.stored {
            stored.added = UnsafeCell::new(false);
            stored.mutated = UnsafeCell::new(false);
        }
    }
}

/// A collection of resource instances identified by their type.
pub struct Resources {
    pub(crate) resource_data: HashMap<TypeId, ResourceData>,
    thread_local_data: HashMap<TypeId, Box<dyn ResourceStorage>>,
    main_thread_id: ThreadId,
}

impl Default for Resources {
    fn default() -> Self {
        Resources {
            resource_data: Default::default(),
            thread_local_data: Default::default(),
            main_thread_id: std::thread::current().id(),
        }
    }
}

impl Resources {
    pub fn insert<T: Resource>(&mut self, resource: T) {
        self.insert_resource(resource, ResourceIndex::Global);
    }

    pub fn insert_thread_local<T: 'static>(&mut self, resource: T) {
        self.check_thread_local();
        let entry = self
            .thread_local_data
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(VecResourceStorage::<T>::default()));
        let resources = entry.downcast_mut::<VecResourceStorage<T>>().unwrap();
        if resources.is_empty() {
            resources.push(resource);
        } else {
            resources.set(0, resource);
        }
    }

    fn check_thread_local(&self) {
        if std::thread::current().id() != self.main_thread_id {
            panic!("Attempted to access a thread local resource off of the main thread.")
        }
    }

    pub fn contains<T: Resource>(&self) -> bool {
        self.get_resource::<T>(ResourceIndex::Global).is_some()
    }

    pub fn get<T: Resource>(&self) -> Option<ResourceRef<'_, T>> {
        self.get_resource(ResourceIndex::Global)
    }

    pub fn get_mut<T: Resource>(&self) -> Option<ResourceRefMut<'_, T>> {
        self.get_resource_mut(ResourceIndex::Global)
    }

    pub fn get_thread_local<T: 'static>(&self) -> Option<ResourceRef<'_, T>> {
        self.check_thread_local();
        self.thread_local_data
            .get(&TypeId::of::<T>())
            .and_then(|storage| {
                let resources = storage.downcast_ref::<VecResourceStorage<T>>().unwrap();
                resources.get(0)
            })
    }

    pub fn get_thread_local_mut<T: 'static>(&self) -> Option<ResourceRefMut<'_, T>> {
        self.check_thread_local();
        self.thread_local_data
            .get(&TypeId::of::<T>())
            .and_then(|storage| {
                let resources = storage.downcast_ref::<VecResourceStorage<T>>().unwrap();
                resources.get_mut(0)
            })
    }

    pub fn get_or_insert_with<T: Resource>(
        &mut self,
        get_resource: impl FnOnce() -> T,
    ) -> ResourceRefMut<'_, T> {
        // NOTE: this double-get is really weird. why cant we use an if-let here?
        if self.get::<T>().is_some() {
            return self.get_mut::<T>().unwrap();
        }
        self.insert(get_resource());
        self.get_mut().unwrap()
    }

    /// Returns a clone of the underlying resource, this is helpful when borrowing something
    /// cloneable (like a task pool) without taking a borrow on the resource map
    pub fn get_cloned<T: Resource + Clone>(&self) -> Option<T> {
        self.get::<T>().map(|r| (*r).clone())
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn get_local<'a, T: Resource>(&'a self, id: SystemId) -> Option<ResourceRef<'a, T>> {
        self.get_resource(ResourceIndex::System(id))
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn get_local_mut<'a, T: Resource>(&'a self, id: SystemId) -> Option<ResourceRefMut<'a, T>> {
        self.get_resource_mut(ResourceIndex::System(id))
    }

    pub fn insert_local<T: Resource>(&mut self, id: SystemId, resource: T) {
        self.insert_resource(resource, ResourceIndex::System(id))
    }

    fn insert_resource<T: Resource>(&mut self, resource: T, resource_index: ResourceIndex) {
        let type_id = TypeId::of::<T>();
        let data = self.resource_data.entry(type_id).or_insert_with(|| {
            let mut types = Vec::new();
            types.push(TypeInfo::of::<T>());
            ResourceData {
                storage: Box::new(VecResourceStorage::<T>::default()),
                default_index: None,
                system_id_to_archetype_index: HashMap::default(),
            }
        });

        let storage = data
            .storage
            .downcast_mut::<VecResourceStorage<T>>()
            .unwrap();
        let index = match resource_index {
            ResourceIndex::Global => *data
                .default_index
                .get_or_insert_with(|| storage.stored.len()),
            ResourceIndex::System(id) => *data
                .system_id_to_archetype_index
                .entry(id.0)
                .or_insert_with(|| storage.stored.len()),
        };

        use std::cmp::Ordering;
        match index.cmp(&storage.stored.len()) {
            Ordering::Equal => {
                storage.push(resource);
            }
            Ordering::Greater => panic!("Attempted to access index beyond 'current_capacity + 1'."),
            Ordering::Less => {
                *storage.get_mut(index).unwrap() = resource;
            }
        }
    }

    fn get_resource<T: Resource>(
        &self,
        resource_index: ResourceIndex,
    ) -> Option<ResourceRef<'_, T>> {
        self.get_resource_data_index::<T>(resource_index)
            .and_then(|(data, index)| {
                let resources = data
                    .storage
                    .downcast_ref::<VecResourceStorage<T>>()
                    .unwrap();
                resources.get(index)
            })
    }

    fn get_resource_mut<T: Resource>(
        &self,
        resource_index: ResourceIndex,
    ) -> Option<ResourceRefMut<'_, T>> {
        self.get_resource_data_index::<T>(resource_index)
            .and_then(|(data, index)| {
                let resources = data
                    .storage
                    .downcast_ref::<VecResourceStorage<T>>()
                    .unwrap();
                resources.get_mut(index)
            })
    }

    #[inline]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_unsafe_ref<T: Resource>(&self, resource_index: ResourceIndex) -> NonNull<T> {
        self.get_resource_data_index::<T>(resource_index)
            .map(|(data, index)| {
                let resources = data
                    .storage
                    .downcast_ref::<VecResourceStorage<T>>()
                    .unwrap();
                resources.get_unsafe_ref(index)
            })
            .unwrap_or_else(|| panic!("Resource does not exist {}.", std::any::type_name::<T>()))
    }

    #[inline]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_unsafe_ref_with_added_and_mutated<T: Resource>(
        &self,
        resource_index: ResourceIndex,
    ) -> (NonNull<T>, NonNull<bool>, NonNull<bool>) {
        self.get_resource_data_index::<T>(resource_index)
            .map(|(data, index)| {
                let resources = data
                    .storage
                    .downcast_ref::<VecResourceStorage<T>>()
                    .unwrap();

                (
                    resources.get_unsafe_ref(index),
                    NonNull::new_unchecked(resources.stored[index].added.get()),
                    NonNull::new_unchecked(resources.stored[index].mutated.get()),
                )
            })
            .unwrap_or_else(|| panic!("Resource does not exist {}.", std::any::type_name::<T>()))
    }

    #[inline]
    fn get_resource_data_index<T: Resource>(
        &self,
        resource_index: ResourceIndex,
    ) -> Option<(&ResourceData, usize)> {
        self.resource_data.get(&TypeId::of::<T>()).and_then(|data| {
            let index = match resource_index {
                ResourceIndex::Global => data.default_index?,
                ResourceIndex::System(id) => {
                    data.system_id_to_archetype_index.get(&id.0).cloned()?
                }
            };
            Some((data, index as usize))
        })
    }

    /// Clears each resource's tracker state.
    /// For example, each resource's component "mutated" state will be reset to `false`.
    pub fn clear_trackers(&mut self) {
        for (_, resource_data) in self.resource_data.iter_mut() {
            resource_data.storage.clear_trackers();
        }
    }
}

unsafe impl Send for Resources {}
unsafe impl Sync for Resources {}

/// Creates `Self` using data from the `Resources` collection
pub trait FromResources {
    /// Creates `Self` using data from the `Resources` collection
    fn from_resources(resources: &Resources) -> Self;
}

impl<T> FromResources for T
where
    T: Default,
{
    fn from_resources(_resources: &Resources) -> Self {
        Self::default()
    }
}

/// Shared borrow of an entity's component
#[derive(Clone)]
pub struct ResourceRef<'a, T: 'static> {
    borrow: &'a AtomicBorrow,
    resource: &'a T,
}

impl<'a, T: 'static> ResourceRef<'a, T> {
    /// Creates a new resource borrow
    fn new(
        StoredResource {
            value,
            added: _,
            mutated: _,
            atomic_borrow,
        }: &'a StoredResource<T>,
    ) -> Self {
        if atomic_borrow.borrow() {
            Self {
                // Safe because we acquired the lock
                resource: unsafe { &*value.get() },
                borrow: atomic_borrow,
            }
        } else {
            panic!(
                "Failed to acquire shared lock on resource: {}.",
                std::any::type_name::<T>()
            );
        }
    }
}

unsafe impl<T: 'static> Send for ResourceRef<'_, T> {}
unsafe impl<T: 'static> Sync for ResourceRef<'_, T> {}

impl<'a, T: 'static> Drop for ResourceRef<'a, T> {
    fn drop(&mut self) {
        self.borrow.release()
    }
}

impl<'a, T: 'static> Deref for ResourceRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.resource
    }
}

impl<'a, T: 'static> Debug for ResourceRef<'a, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.deref().fmt(f)
    }
}

/// Unique borrow of a resource
pub struct ResourceRefMut<'a, T: 'static> {
    borrow: &'a AtomicBorrow,
    resource: &'a mut T,
    mutated: &'a mut bool,
}

impl<'a, T: 'static> ResourceRefMut<'a, T> {
    /// Creates a new entity component mutable borrow
    fn new(
        StoredResource {
            value,
            added: _,
            mutated,
            atomic_borrow,
        }: &'a StoredResource<T>,
    ) -> Self {
        if atomic_borrow.borrow_mut() {
            Self {
                // Safe because we acquired the lock
                resource: unsafe { &mut *value.get() },
                // same
                mutated: unsafe { &mut *mutated.get() },
                borrow: atomic_borrow,
            }
        } else {
            panic!(
                "Failed to acquire exclusive lock on resource: {}.",
                std::any::type_name::<T>()
            );
        }
    }
}

unsafe impl<T: 'static> Send for ResourceRefMut<'_, T> {}
unsafe impl<T: 'static> Sync for ResourceRefMut<'_, T> {}

impl<'a, T: 'static> Drop for ResourceRefMut<'a, T> {
    fn drop(&mut self) {
        self.borrow.release_mut();
    }
}

impl<'a, T: 'static> Deref for ResourceRefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.resource
    }
}

impl<'a, T: 'static> DerefMut for ResourceRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        *self.mutated = true;
        self.resource
    }
}

impl<'a, T: 'static> Debug for ResourceRefMut<'a, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.deref().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::Resources;
    use crate::system::SystemId;

    #[test]
    fn resource() {
        let mut resources = Resources::default();
        assert!(resources.get::<i32>().is_none());

        resources.insert(123);
        assert_eq!(*resources.get::<i32>().expect("resource exists"), 123);

        resources.insert(456.0);
        assert_eq!(*resources.get::<f64>().expect("resource exists"), 456.0);

        resources.insert(789.0);
        assert_eq!(*resources.get::<f64>().expect("resource exists"), 789.0);

        {
            let mut value = resources.get_mut::<f64>().expect("resource exists");
            assert_eq!(*value, 789.0);
            *value = -1.0;
        }

        assert_eq!(*resources.get::<f64>().expect("resource exists"), -1.0);

        assert!(resources.get_local::<i32>(SystemId(0)).is_none());
        resources.insert_local(SystemId(0), 111);
        assert_eq!(
            *resources
                .get_local::<i32>(SystemId(0))
                .expect("resource exists"),
            111
        );
        assert_eq!(*resources.get::<i32>().expect("resource exists"), 123);
        resources.insert_local(SystemId(0), 222);
        assert_eq!(
            *resources
                .get_local::<i32>(SystemId(0))
                .expect("resource exists"),
            222
        );
        assert_eq!(*resources.get::<i32>().expect("resource exists"), 123);
    }

    #[test]
    #[should_panic(expected = "Failed to acquire exclusive lock on resource: i32")]
    fn resource_double_mut_panic() {
        let mut resources = Resources::default();
        resources.insert(123);
        let _x = resources.get_mut::<i32>();
        let _y = resources.get_mut::<i32>();
    }

    #[test]
    fn thread_local_resource() {
        let mut resources = Resources::default();
        resources.insert_thread_local(123i32);
        resources.insert_thread_local(456i64);
        assert_eq!(*resources.get_thread_local::<i32>().unwrap(), 123);
        assert_eq!(*resources.get_thread_local_mut::<i64>().unwrap(), 456);
    }

    #[test]
    fn thread_local_resource_ref_aliasing() {
        let mut resources = Resources::default();
        resources.insert_thread_local(123i32);
        let a = resources.get_thread_local::<i32>().unwrap();
        let b = resources.get_thread_local::<i32>().unwrap();
        assert_eq!(*a, 123);
        assert_eq!(*b, 123);
    }

    #[test]
    #[should_panic]
    fn thread_local_resource_mut_ref_aliasing() {
        let mut resources = Resources::default();
        resources.insert_thread_local(123i32);
        let _a = resources.get_thread_local::<i32>().unwrap();
        let _b = resources.get_thread_local_mut::<i32>().unwrap();
    }

    #[test]
    #[should_panic]
    fn thread_local_resource_panic() {
        let mut resources = Resources::default();
        resources.insert_thread_local(0i32);
        std::thread::spawn(move || {
            let _ = resources.get_thread_local_mut::<i32>();
        })
        .join()
        .unwrap();
    }
}
