use crate::resource::{
    PreparedRead, PreparedWrite, Resource, ResourceSet, ResourceTypeId, Resources,
};
use std::ops::{Deref, DerefMut};

impl<T: Resource> ResourceSet for PreparedRead<T> {
    type PreparedResources = PreparedRead<T>;

    unsafe fn fetch_unchecked(resources: &Resources) -> Self::PreparedResources {
        let resource = resources
            .get::<T>()
            .unwrap_or_else(|| panic!("Failed to fetch resource!: {}", std::any::type_name::<T>()));
        PreparedRead::new(resource.deref() as *const T)
    }
    fn read_types() -> Vec<ResourceTypeId> { vec![ResourceTypeId::of::<T>()] }
    fn write_types() -> Vec<ResourceTypeId> { Vec::new() }
}

impl<T: Resource> ResourceSet for PreparedWrite<T> {
    type PreparedResources = PreparedWrite<T>;

    unsafe fn fetch_unchecked(resources: &Resources) -> Self::PreparedResources {
        let mut resource = resources
            .get_mut::<T>()
            .unwrap_or_else(|| panic!("Failed to fetch resource!: {}", std::any::type_name::<T>()));
        PreparedWrite::new(resource.deref_mut() as *mut T)
    }
    fn read_types() -> Vec<ResourceTypeId> { Vec::new() }
    fn write_types() -> Vec<ResourceTypeId> { vec![ResourceTypeId::of::<T>()] }
}
