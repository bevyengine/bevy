use super::{FetchResource, ResourceQuery};
use crate::system::SystemId;
use bevy_hecs::{Archetype, Entity, Ref, RefMut, TypeInfo, TypeState};
use bevy_utils::HashMap;
use core::any::TypeId;
use std::ptr::NonNull;

/// A Resource type
pub trait Resource: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Resource for T {}

#[derive(Debug)]
pub(crate) struct ResourceData {
    archetype: Archetype,
    default_index: Option<usize>,
    system_id_to_archetype_index: HashMap<usize, usize>,
}

#[derive(Debug)]
pub enum ResourceIndex {
    Global,
    System(SystemId),
}

/// A collection of resource instances identified by their type.
#[derive(Debug, Default)]
pub struct Resources {
    pub(crate) resource_data: HashMap<TypeId, ResourceData>,
}

impl Resources {
    pub fn insert<T: Resource>(&mut self, resource: T) {
        self.insert_resource(resource, ResourceIndex::Global);
    }

    pub fn contains<T: Resource>(&self) -> bool {
        self.get_resource::<T>(ResourceIndex::Global).is_some()
    }

    pub fn get<T: Resource>(&self) -> Option<Ref<'_, T>> {
        self.get_resource(ResourceIndex::Global)
    }

    pub fn get_mut<T: Resource>(&self) -> Option<RefMut<'_, T>> {
        self.get_resource_mut(ResourceIndex::Global)
    }

    /// Returns a clone of the underlying resource, this is helpful when borrowing something
    /// cloneable (like a task pool) without taking a borrow on the resource map
    pub fn get_cloned<T: Resource + Clone>(&self) -> Option<T> {
        self.get::<T>().map(|r| (*r).clone())
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn get_local<'a, T: Resource>(&'a self, id: SystemId) -> Option<Ref<'a, T>> {
        self.get_resource(ResourceIndex::System(id))
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn get_local_mut<'a, T: Resource>(&'a self, id: SystemId) -> Option<RefMut<'a, T>> {
        self.get_resource_mut(ResourceIndex::System(id))
    }

    pub fn insert_local<T: Resource>(&mut self, id: SystemId, resource: T) {
        self.insert_resource(resource, ResourceIndex::System(id))
    }

    fn insert_resource<T: Resource>(&mut self, mut resource: T, resource_index: ResourceIndex) {
        let type_id = TypeId::of::<T>();
        let data = self.resource_data.entry(type_id).or_insert_with(|| {
            let mut types = Vec::new();
            types.push(TypeInfo::of::<T>());
            ResourceData {
                archetype: Archetype::new(types),
                default_index: None,
                system_id_to_archetype_index: HashMap::default(),
            }
        });

        let archetype = &mut data.archetype;
        let mut added = false;
        let index = match resource_index {
            ResourceIndex::Global => *data.default_index.get_or_insert_with(|| {
                added = true;
                archetype.len()
            }),
            ResourceIndex::System(id) => *data
                .system_id_to_archetype_index
                .entry(id.0)
                .or_insert_with(|| {
                    added = true;
                    archetype.len()
                }),
        };

        use std::cmp::Ordering;
        match index.cmp(&archetype.len()) {
            Ordering::Equal => {
                unsafe { archetype.allocate(Entity::new(index as u32)) };
            }
            Ordering::Greater => panic!("attempted to access index beyond 'current_capacity + 1'"),
            Ordering::Less => (),
        }

        unsafe {
            let resource_ptr = (&mut resource as *mut T).cast::<u8>();
            archetype.put_dynamic(
                resource_ptr,
                type_id,
                core::mem::size_of::<T>(),
                index,
                added,
            );
            std::mem::forget(resource);
        }
    }

    fn get_resource<T: Resource>(&self, resource_index: ResourceIndex) -> Option<Ref<'_, T>> {
        self.resource_data
            .get(&TypeId::of::<T>())
            .and_then(|data| unsafe {
                let index = match resource_index {
                    ResourceIndex::Global => data.default_index?,
                    ResourceIndex::System(id) => *data.system_id_to_archetype_index.get(&id.0)?,
                };
                Ref::new(&data.archetype, index).ok()
            })
    }

    fn get_resource_mut<T: Resource>(
        &self,
        resource_index: ResourceIndex,
    ) -> Option<RefMut<'_, T>> {
        self.resource_data
            .get(&TypeId::of::<T>())
            .and_then(|data| unsafe {
                let index = match resource_index {
                    ResourceIndex::Global => data.default_index?,
                    ResourceIndex::System(id) => *data.system_id_to_archetype_index.get(&id.0)?,
                };
                RefMut::new(&data.archetype, index).ok()
            })
    }

    pub fn query<Q: ResourceQuery>(&self) -> Option<<Q::Fetch as FetchResource>::Item> {
        unsafe {
            if Q::Fetch::is_some(&self, None) {
                Some(Q::Fetch::get(&self, None))
            } else {
                None
            }
        }
    }

    pub fn query_system<Q: ResourceQuery>(
        &self,
        id: SystemId,
    ) -> Option<<Q::Fetch as FetchResource>::Item> {
        unsafe {
            if Q::Fetch::is_some(&self, Some(id)) {
                Some(Q::Fetch::get(&self, Some(id)))
            } else {
                None
            }
        }
    }

    #[inline]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_unsafe_ref<T: Resource>(&self, resource_index: ResourceIndex) -> NonNull<T> {
        self.get_resource_data_index::<T>(resource_index)
            .and_then(|(data, index)| {
                Some(NonNull::new_unchecked(
                    data.archetype.get::<T>()?.as_ptr().add(index),
                ))
            })
            .unwrap_or_else(|| panic!("Resource does not exist {}", std::any::type_name::<T>()))
    }

    #[inline]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_unsafe_ref_with_type_state<T: Resource>(
        &self,
        resource_index: ResourceIndex,
    ) -> (NonNull<T>, &TypeState) {
        self.get_resource_data_index::<T>(resource_index)
            .and_then(|(data, index)| {
                data.archetype
                    .get_with_type_state::<T>()
                    .map(|(resource, type_state)| {
                        (
                            NonNull::new_unchecked(resource.as_ptr().add(index)),
                            type_state,
                        )
                    })
            })
            .unwrap_or_else(|| panic!("Resource does not exist {}", std::any::type_name::<T>()))
    }

    #[inline]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_unsafe_added_and_mutated<T: Resource>(
        &self,
        resource_index: ResourceIndex,
    ) -> (NonNull<bool>, NonNull<bool>) {
        self.get_resource_data_index::<T>(resource_index)
            .and_then(|(data, index)| {
                let type_state = data.archetype.get_type_state(TypeId::of::<T>())?;
                Some((
                    NonNull::new_unchecked(type_state.added().as_ptr().add(index)),
                    NonNull::new_unchecked(type_state.mutated().as_ptr().add(index)),
                ))
            })
            .unwrap_or_else(|| panic!("Resource does not exist {}", std::any::type_name::<T>()))
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

    pub fn borrow<T: Resource>(&self) {
        if let Some(data) = self.resource_data.get(&TypeId::of::<T>()) {
            data.archetype.borrow::<T>();
        }
    }

    pub fn release<T: Resource>(&self) {
        if let Some(data) = self.resource_data.get(&TypeId::of::<T>()) {
            data.archetype.release::<T>();
        }
    }

    pub fn borrow_mut<T: Resource>(&self) {
        if let Some(data) = self.resource_data.get(&TypeId::of::<T>()) {
            data.archetype.borrow_mut::<T>();
        }
    }

    pub fn release_mut<T: Resource>(&self) {
        if let Some(data) = self.resource_data.get(&TypeId::of::<T>()) {
            data.archetype.release_mut::<T>();
        }
    }

    /// Clears each resource's tracker state.
    /// For example, each resource's component "mutated" state will be reset to `false`.
    pub fn clear_trackers(&mut self) {
        for (_, resource_data) in self.resource_data.iter_mut() {
            resource_data.archetype.clear_trackers();
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
    #[should_panic(expected = "i32 already borrowed")]
    fn resource_double_mut_panic() {
        let mut resources = Resources::default();
        resources.insert(123);
        let _x = resources.get_mut::<i32>();
        let _y = resources.get_mut::<i32>();
    }
}
