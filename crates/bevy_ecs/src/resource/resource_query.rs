use super::FromResources;
use crate::{Resource, ResourceIndex, Resources, SystemId};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

// TODO: align TypeAccess api with Query::Fetch

/// A shared borrow of a Resource
/// that will only return in a query if the Resource has been changed
#[derive(Debug)]
pub struct ChangedRes<'a, T: Resource> {
    value: &'a T,
}

impl<'a, T: Resource> ChangedRes<'a, T> {
    /// Creates a reference cell to a Resource from a pointer
    ///
    /// # Safety
    /// The pointer must have correct lifetime / storage
    pub unsafe fn new(value: NonNull<T>) -> Self {
        Self {
            value: &*value.as_ptr(),
        }
    }
}

impl<'a, T: Resource> Deref for ChangedRes<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

/// Shared borrow of a Resource
#[derive(Debug)]
pub struct Res<'a, T: Resource> {
    value: &'a T,
}

impl<'a, T: Resource> Res<'a, T> {
    /// Creates a reference cell to a Resource from a pointer
    ///
    /// # Safety
    /// The pointer must have correct lifetime / storage
    pub unsafe fn new(value: NonNull<T>) -> Self {
        Self {
            value: &*value.as_ptr(),
        }
    }
}

impl<'a, T: Resource> Deref for Res<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

/// Unique borrow of a Resource
#[derive(Debug)]
pub struct ResMut<'a, T: Resource> {
    _marker: PhantomData<&'a T>,
    value: *mut T,
    mutated: *mut bool,
}

impl<'a, T: Resource> ResMut<'a, T> {
    /// Creates a mutable reference cell to a Resource from a pointer
    ///
    /// # Safety
    /// The pointer must have correct lifetime / storage / ownership
    pub unsafe fn new(value: NonNull<T>, mutated: NonNull<bool>) -> Self {
        Self {
            value: value.as_ptr(),
            mutated: mutated.as_ptr(),
            _marker: Default::default(),
        }
    }
}

impl<'a, T: Resource> Deref for ResMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.value }
    }
}

impl<'a, T: Resource> DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            *self.mutated = true;
            &mut *self.value
        }
    }
}

/// Local<T> resources are unique per-system. Two instances of the same system will each have their own resource.
/// Local resources are automatically initialized using the FromResources trait.
#[derive(Debug)]
pub struct Local<'a, T: Resource + FromResources> {
    value: *mut T,
    _marker: PhantomData<&'a T>,
}

impl<'a, T: Resource + FromResources> Local<'a, T> {
    pub(crate) unsafe fn new(resources: &Resources, id: SystemId) -> Self {
        Local {
            value: resources
                .get_unsafe_ref::<T>(ResourceIndex::System(id))
                .as_ptr(),
            _marker: Default::default(),
        }
    }
}

impl<'a, T: Resource + FromResources> Deref for Local<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.value }
    }
}

impl<'a, T: Resource + FromResources> DerefMut for Local<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn changed_resource() {
//         let mut resources = Resources::default();
//         resources.insert(123);
//         assert_eq!(
//             resources.query::<ChangedRes<i32>>().as_deref(),
//             Some(&(123 as i32))
//         );
//         resources.clear_trackers();
//         assert_eq!(resources.query::<ChangedRes<i32>>().as_deref(), None);
//         *resources.query::<ResMut<i32>>().unwrap() += 1;
//         assert_eq!(
//             resources.query::<ChangedRes<i32>>().as_deref(),
//             Some(&(124 as i32))
//         );
//     }

//     #[test]
//     fn or_changed_resource() {
//         let mut resources = Resources::default();
//         resources.insert(123);
//         resources.insert(0.2);
//         assert!(resources
//             .query::<OrRes<(ChangedRes<i32>, ChangedRes<f64>)>>()
//             .is_some(),);
//         resources.clear_trackers();
//         assert!(resources
//             .query::<OrRes<(ChangedRes<i32>, ChangedRes<f64>)>>()
//             .is_none(),);
//         *resources.query::<ResMut<i32>>().unwrap() += 1;
//         assert!(resources
//             .query::<OrRes<(ChangedRes<i32>, ChangedRes<f64>)>>()
//             .is_some(),);
//         assert!(resources
//             .query::<(ChangedRes<i32>, ChangedRes<f64>)>()
//             .is_none(),);
//     }
// }
