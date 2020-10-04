use super::{FromResources, Resources};
use crate::{system::SystemId, Resource, ResourceIndex};
use bevy_hecs::{smaller_tuples_too, TypeAccess};
use core::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};
use std::{any::TypeId, marker::PhantomData};

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

impl<'a, T: Resource> UnsafeClone for ChangedRes<'a, T> {
    unsafe fn unsafe_clone(&self) -> Self {
        Self { value: self.value }
    }
}

unsafe impl<T: Resource> Send for ChangedRes<'_, T> {}
unsafe impl<T: Resource> Sync for ChangedRes<'_, T> {}

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

/// A clone that is unsafe to perform. You probably shouldn't use this.
pub trait UnsafeClone {
    #[allow(clippy::missing_safety_doc)]
    unsafe fn unsafe_clone(&self) -> Self;
}

impl<'a, T: Resource> UnsafeClone for Res<'a, T> {
    unsafe fn unsafe_clone(&self) -> Self {
        Self { value: self.value }
    }
}

unsafe impl<T: Resource> Send for Res<'_, T> {}
unsafe impl<T: Resource> Sync for Res<'_, T> {}

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

unsafe impl<T: Resource> Send for ResMut<'_, T> {}
unsafe impl<T: Resource> Sync for ResMut<'_, T> {}

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

impl<'a, T: Resource> UnsafeClone for ResMut<'a, T> {
    unsafe fn unsafe_clone(&self) -> Self {
        Self {
            value: self.value,
            mutated: self.mutated,
            _marker: Default::default(),
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

impl<'a, T: Resource + FromResources> UnsafeClone for Local<'a, T> {
    unsafe fn unsafe_clone(&self) -> Self {
        Self {
            value: self.value,
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

/// A collection of resource types fetch from a `Resources` collection
pub trait ResourceQuery {
    type Fetch: for<'a> FetchResource<'a>;

    fn initialize(_resources: &mut Resources, _system_id: Option<SystemId>) {}
}

/// Streaming iterators over contiguous homogeneous ranges of resources
pub trait FetchResource<'a>: Sized {
    /// Type of value to be fetched
    type Item: UnsafeClone;

    fn access() -> TypeAccess<TypeId>;
    fn borrow(resources: &Resources);
    fn release(resources: &Resources);

    #[allow(clippy::missing_safety_doc)]
    unsafe fn get(resources: &'a Resources, system_id: Option<SystemId>) -> Self::Item;

    #[allow(clippy::missing_safety_doc)]
    unsafe fn is_some(_resources: &'a Resources, _system_id: Option<SystemId>) -> bool {
        true
    }
}

impl<'a, T: Resource> ResourceQuery for Res<'a, T> {
    type Fetch = FetchResourceRead<T>;
}

/// Fetches a shared resource reference
#[derive(Debug)]
pub struct FetchResourceRead<T>(NonNull<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchResourceRead<T> {
    type Item = Res<'a, T>;

    unsafe fn get(resources: &'a Resources, _system_id: Option<SystemId>) -> Self::Item {
        Res::new(resources.get_unsafe_ref::<T>(ResourceIndex::Global))
    }

    fn borrow(resources: &Resources) {
        resources.borrow::<T>();
    }

    fn release(resources: &Resources) {
        resources.release::<T>();
    }

    fn access() -> TypeAccess<TypeId> {
        let mut access = TypeAccess::default();
        access.add_read(TypeId::of::<T>().into());
        access
    }
}

impl<'a, T: Resource> ResourceQuery for ChangedRes<'a, T> {
    type Fetch = FetchResourceChanged<T>;
}

/// Fetches a shared resource reference
#[derive(Debug)]
pub struct FetchResourceChanged<T>(NonNull<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchResourceChanged<T> {
    type Item = ChangedRes<'a, T>;

    unsafe fn get(resources: &'a Resources, _system_id: Option<SystemId>) -> Self::Item {
        ChangedRes::new(resources.get_unsafe_ref::<T>(ResourceIndex::Global))
    }

    unsafe fn is_some(resources: &'a Resources, _system_id: Option<SystemId>) -> bool {
        let (added, mutated) = resources.get_unsafe_added_and_mutated::<T>(ResourceIndex::Global);
        *added.as_ptr() || *mutated.as_ptr()
    }

    fn borrow(resources: &Resources) {
        resources.borrow::<T>();
    }

    fn release(resources: &Resources) {
        resources.release::<T>();
    }

    fn access() -> TypeAccess<TypeId> {
        let mut access = TypeAccess::default();
        access.add_read(TypeId::of::<T>().into());
        access
    }
}

impl<'a, T: Resource> ResourceQuery for ResMut<'a, T> {
    type Fetch = FetchResourceWrite<T>;
}

/// Fetches a unique resource reference
#[derive(Debug)]
pub struct FetchResourceWrite<T>(NonNull<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchResourceWrite<T> {
    type Item = ResMut<'a, T>;

    unsafe fn get(resources: &'a Resources, _system_id: Option<SystemId>) -> Self::Item {
        let (value, type_state) =
            resources.get_unsafe_ref_with_type_state::<T>(ResourceIndex::Global);
        ResMut::new(value, type_state.mutated())
    }

    fn borrow(resources: &Resources) {
        resources.borrow_mut::<T>();
    }

    fn release(resources: &Resources) {
        resources.release_mut::<T>();
    }

    fn access() -> TypeAccess<TypeId> {
        let mut access = TypeAccess::default();
        access.add_write(TypeId::of::<T>().into());
        access
    }
}

impl<'a, T: Resource + FromResources> ResourceQuery for Local<'a, T> {
    type Fetch = FetchResourceLocalMut<T>;

    fn initialize(resources: &mut Resources, id: Option<SystemId>) {
        let id = id.expect("Local<T> resources can only be used by systems");

        // Only add the local resource if it doesn't already exist for this system
        if resources.get_local::<T>(id).is_none() {
            let value = T::from_resources(resources);
            resources.insert_local(id, value);
        }
    }
}

/// Fetches a `Local<T>` resource reference
#[derive(Debug)]
pub struct FetchResourceLocalMut<T>(NonNull<T>);

impl<'a, T: Resource + FromResources> FetchResource<'a> for FetchResourceLocalMut<T> {
    type Item = Local<'a, T>;

    unsafe fn get(resources: &'a Resources, system_id: Option<SystemId>) -> Self::Item {
        let id = system_id.expect("Local<T> resources can only be used by systems");
        Local {
            value: resources
                .get_unsafe_ref::<T>(ResourceIndex::System(id))
                .as_ptr(),
            _marker: Default::default(),
        }
    }

    fn borrow(resources: &Resources) {
        resources.borrow_mut::<T>();
    }

    fn release(resources: &Resources) {
        resources.release_mut::<T>();
    }

    fn access() -> TypeAccess<TypeId> {
        let mut access = TypeAccess::default();
        access.add_write(TypeId::of::<T>().into());
        access
    }
}

macro_rules! tuple_impl {
    ($($name: ident),*) => {
        impl<'a, $($name: FetchResource<'a>),*> FetchResource<'a> for ($($name,)*) {
            type Item = ($($name::Item,)*);

            #[allow(unused_variables)]
            fn borrow(resources: &Resources) {
                $($name::borrow(resources);)*
            }

            #[allow(unused_variables)]
            fn release(resources: &Resources) {
                $($name::release(resources);)*
            }

            #[allow(unused_variables)]
            unsafe fn get(resources: &'a Resources, system_id: Option<SystemId>) -> Self::Item {
                ($($name::get(resources, system_id),)*)
            }

            #[allow(unused_variables)]
            unsafe fn is_some(resources: &'a Resources, system_id: Option<SystemId>) -> bool {
                true $(&& $name::is_some(resources, system_id))*
            }

            #[allow(unused_mut)]
            fn access() -> TypeAccess<TypeId> {
                let mut access = TypeAccess::default();
                $(access.union(&$name::access());)*
                access
            }
        }

        impl<$($name: ResourceQuery),*> ResourceQuery for ($($name,)*) {
            type Fetch = ($($name::Fetch,)*);

            #[allow(unused_variables)]
            fn initialize(resources: &mut Resources, system_id: Option<SystemId>) {
                $($name::initialize(resources, system_id);)*
            }
        }

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($name: UnsafeClone),*> UnsafeClone for ($($name,)*) {
            unsafe fn unsafe_clone(&self) -> Self {
                let ($($name,)*) = self;
                ($($name.unsafe_clone(),)*)
            }
        }
    };
}

smaller_tuples_too!(tuple_impl, O, N, M, L, K, J, I, H, G, F, E, D, C, B, A);

#[derive(Debug)]
pub struct OrRes<T>(T);

#[derive(Debug)]
pub struct FetchResourceOr<T>(NonNull<T>);

macro_rules! tuple_impl_or {
    ($($name: ident),*) => {
        impl<'a, $($name: FetchResource<'a>),*> FetchResource<'a> for FetchResourceOr<($($name,)*)> {
            type Item = OrRes<($($name::Item,)*)>;

            #[allow(unused_variables)]
            fn borrow(resources: &Resources) {
                $($name::borrow(resources);)*
            }

            #[allow(unused_variables)]
            fn release(resources: &Resources) {
                $($name::release(resources);)*
            }

            #[allow(unused_variables)]
            unsafe fn get(resources: &'a Resources, system_id: Option<SystemId>) -> Self::Item {
                OrRes(($($name::get(resources, system_id),)*))
            }

            #[allow(unused_variables)]
            unsafe fn is_some(resources: &'a Resources, system_id: Option<SystemId>) -> bool {
                false $(|| $name::is_some(resources, system_id))*
            }

            #[allow(unused_mut)]
            fn access() -> TypeAccess<TypeId> {
                let mut access = TypeAccess::default();
                $(access.union(&$name::access());)*
                access
            }
        }

        impl<$($name: ResourceQuery),*> ResourceQuery for OrRes<($($name,)*)> {
            type Fetch = FetchResourceOr<($($name::Fetch,)*)>;

            #[allow(unused_variables)]
            fn initialize(resources: &mut Resources, system_id: Option<SystemId>) {
                $($name::initialize(resources, system_id);)*
            }
        }

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($name: UnsafeClone),*> UnsafeClone for OrRes<($($name,)*)> {
            unsafe fn unsafe_clone(&self) -> Self {
                let OrRes(($($name,)*)) = self;
                OrRes(($($name.unsafe_clone(),)*))
            }
        }

        impl<$($name,)*> Deref for OrRes<($($name,)*)> {
            type Target = ($($name,)*);

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

smaller_tuples_too!(tuple_impl_or, O, N, M, L, K, J, I, H, G, F, E, D, C, B, A);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_resource() {
        let mut resources = Resources::default();
        resources.insert(123);
        assert_eq!(
            resources.query::<ChangedRes<i32>>().as_deref(),
            Some(&(123 as i32))
        );
        resources.clear_trackers();
        assert_eq!(resources.query::<ChangedRes<i32>>().as_deref(), None);
        *resources.query::<ResMut<i32>>().unwrap() += 1;
        assert_eq!(
            resources.query::<ChangedRes<i32>>().as_deref(),
            Some(&(124 as i32))
        );
    }

    #[test]
    fn or_changed_resource() {
        let mut resources = Resources::default();
        resources.insert(123);
        resources.insert(0.2);
        assert!(resources
            .query::<OrRes<(ChangedRes<i32>, ChangedRes<f64>)>>()
            .is_some(),);
        resources.clear_trackers();
        assert!(resources
            .query::<OrRes<(ChangedRes<i32>, ChangedRes<f64>)>>()
            .is_none(),);
        *resources.query::<ResMut<i32>>().unwrap() += 1;
        assert!(resources
            .query::<OrRes<(ChangedRes<i32>, ChangedRes<f64>)>>()
            .is_some(),);
        assert!(resources
            .query::<(ChangedRes<i32>, ChangedRes<f64>)>()
            .is_none(),);
    }
}
