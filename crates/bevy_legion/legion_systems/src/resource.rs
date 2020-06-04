use downcast_rs::{impl_downcast, Downcast};
use fxhash::FxHashMap;
use legion_core::borrow::{AtomicRefCell, Ref, RefMut};
use legion_core::downcast_typename::DowncastTypename;
use legion_core::query::{Read, ReadOnly, Write};
use std::{
    any::{type_name, Any},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
impl DowncastTypename for dyn Resource {
    #[inline(always)]
    fn downcast_typename_mut<T: Any>(&mut self) -> Option<&mut T> {
        if self.is_typename::<T>() {
            // SAFETY: just checked whether we are pointing to the correct type
            unsafe { Some(&mut *(self.as_any_mut() as *mut dyn Any as *mut T)) }
        } else {
            None
        }
    }

    #[inline(always)]
    fn downcast_typename_ref<T: Any>(&self) -> Option<&T> {
        if self.is_typename::<T>() {
            // SAFETY: just checked whether we are pointing to the correct type
            unsafe { Some(&*(self.as_any() as *const dyn Any as *const T)) }
        } else {
            None
        }
    }

    #[inline(always)]
    fn is_typename<T: Any>(&self) -> bool {
        true
        // TODO: it would be nice to add type safety here, but the type names don't match
        // println!("{} {}", type_name_of_val(self), type_name::<T>());
        // type_name_of_val(self) == type_name::<T>()
    }
}
#[cfg(not(feature = "ffi"))]
/// A type ID identifying a component type.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct ResourceTypeId(&'static str);

#[cfg(not(feature = "ffi"))]
impl ResourceTypeId {
    /// Gets the component type ID that represents type `T`.
    pub fn of<T: Resource>() -> Self { Self(type_name::<T>()) }
}

#[cfg(feature = "ffi")]
/// A type ID identifying a component type.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct ResourceTypeId(&'static str, u32);

#[cfg(feature = "ffi")]
impl ResourceTypeId {
    /// Gets the component type ID that represents type `T`.
    pub fn of<T: Resource>() -> Self { Self(type_name::<T>(), 0) }
}

/// Trait which is implemented for tuples of resources and singular resources. This abstracts
/// fetching resources to allow for ergonomic fetching.
///
/// # Example:
/// ```
///
/// struct TypeA(usize);
/// struct TypeB(usize);
///
/// use legion_core::prelude::*;
/// use legion_systems::prelude::*;
/// let mut resources = Resources::default();
/// resources.insert(TypeA(55));
/// resources.insert(TypeB(12));
///
/// {
///     let (a, mut b) = <(Read<TypeA>, Write<TypeB>)>::fetch_mut(&mut resources);
///     assert_ne!(a.0, b.0);
///     b.0 = a.0;
/// }
///
/// {
///     let (a, b) = <(Read<TypeA>, Read<TypeB>)>::fetch(&resources);
///     assert_eq!(a.0, b.0);
/// }
///
/// ```
pub trait ResourceSet: Send + Sync {
    type PreparedResources;

    /// Fetches all defined resources, without checking mutability.
    ///
    /// # Safety
    /// It is up to the end user to validate proper mutability rules across the resources being accessed.
    ///
    unsafe fn fetch_unchecked(resources: &Resources) -> Self::PreparedResources;

    fn fetch_mut(resources: &mut Resources) -> Self::PreparedResources {
        // safe because mutable borrow ensures exclusivity
        unsafe { Self::fetch_unchecked(resources) }
    }

    fn fetch(resources: &Resources) -> Self::PreparedResources
    where
        Self: ReadOnly,
    {
        unsafe { Self::fetch_unchecked(resources) }
    }

    fn read_types() -> Vec<ResourceTypeId>;
    fn write_types() -> Vec<ResourceTypeId>;
}

/// Blanket trait for resource types.
pub trait Resource: 'static + Downcast + Send + Sync {}
impl<T> Resource for T where T: 'static + Send + Sync {}
impl_downcast!(Resource);

/// Wrapper type for safe, lifetime-garunteed immutable access to a resource of type `T'. This
/// is the wrapper type which is provided to the closure in a `System`, meaning it is only scoped
/// to that system execution.
///
/// # Safety
///
/// This type contains an immutable pointer to `T`, and must not outlive its lifetime
pub struct PreparedRead<T: Resource> {
    resource: *const T,
}
impl<T: Resource> PreparedRead<T> {
    pub(crate) unsafe fn new(resource: *const T) -> Self { Self { resource } }
}
impl<T: Resource> Deref for PreparedRead<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { unsafe { &*self.resource } }
}
unsafe impl<T: Resource> Send for PreparedRead<T> {}
unsafe impl<T: Resource> Sync for PreparedRead<T> {}

/// Wrapper type for safe, lifetime-garunteed mutable access to a resource of type `T'. This
/// is the wrapper type which is provided to the closure in a `System`, meaning it is only scoped
/// to that system execution.
///
/// # Safety
///
/// This type contains an mutable pointer to `T`, and must not outlive its lifetime
pub struct PreparedWrite<T: Resource> {
    resource: *mut T,
}
impl<T: Resource> Deref for PreparedWrite<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { unsafe { &*self.resource } }
}

impl<T: Resource> DerefMut for PreparedWrite<T> {
    fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.resource } }
}
impl<T: Resource> PreparedWrite<T> {
    pub(crate) unsafe fn new(resource: *mut T) -> Self { Self { resource } }
}
unsafe impl<T: Resource> Send for PreparedWrite<T> {}
unsafe impl<T: Resource> Sync for PreparedWrite<T> {}

/// Ergonomic wrapper type which contains a `Ref` type.
pub struct Fetch<'a, T: 'a + Resource> {
    inner: Ref<'a, Box<dyn Resource>>,
    _marker: PhantomData<T>,
}
impl<'a, T: Resource> Deref for Fetch<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner.downcast_typename_ref::<T>().unwrap_or_else(|| {
            panic!(
                "Unable to downcast the resource!: {}",
                std::any::type_name::<T>()
            )
        })
    }
}

impl<'a, T: 'a + Resource + std::fmt::Debug> std::fmt::Debug for Fetch<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

/// Ergonomic wrapper type which contains a `RefMut` type.
pub struct FetchMut<'a, T: Resource> {
    inner: RefMut<'a, Box<dyn Resource>>,
    _marker: PhantomData<T>,
}
impl<'a, T: 'a + Resource> Deref for FetchMut<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner.downcast_typename_ref::<T>().unwrap_or_else(|| {
            panic!(
                "Unable to downcast the resource!: {}",
                std::any::type_name::<T>()
            )
        })
    }
}

impl<'a, T: 'a + Resource> DerefMut for FetchMut<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.inner.downcast_typename_mut::<T>().unwrap_or_else(|| {
            panic!(
                "Unable to downcast the resource!: {}",
                std::any::type_name::<T>()
            )
        })
    }
}

impl<'a, T: 'a + Resource + std::fmt::Debug> std::fmt::Debug for FetchMut<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

/// Resources container. This container stores its underlying resources in a `FxHashMap` keyed on
/// `ResourceTypeId`. This means that the ID's used in this storage will not persist between recompiles.
#[derive(Default)]
pub struct Resources {
    storage: FxHashMap<ResourceTypeId, AtomicRefCell<Box<dyn Resource>>>,
}

impl Resources {
    /// Returns `true` if type `T` exists in the store. Otherwise, returns `false`
    pub fn contains<T: Resource>(&self) -> bool {
        self.storage.contains_key(&ResourceTypeId::of::<T>())
    }

    /// Inserts the instance of `T` into the store. If the type already exists, it will be silently
    /// overwritten. If you would like to retain the instance of the resource that already exists,
    /// call `remove` first to retrieve it.
    pub fn insert<T: Resource>(&mut self, value: T) {
        self.storage.insert(
            ResourceTypeId::of::<T>(),
            AtomicRefCell::new(Box::new(value)),
        );
    }

    /// Removes the type `T` from this store if it exists.
    ///
    /// # Returns
    /// If the type `T` was stored, the inner instance of `T is returned. Otherwise, `None`
    pub fn remove<T: Resource>(&mut self) -> Option<T> {
        Some(
            *self
                .storage
                .remove(&ResourceTypeId::of::<T>())?
                .into_inner()
                .downcast::<T>()
                .ok()?,
        )
    }

    /// Retrieve an immutable reference to  `T` from the store if it exists. Otherwise, return `None`
    pub fn get<T: Resource>(&self) -> Option<Fetch<'_, T>> {
        Some(Fetch {
            inner: self.storage.get(&ResourceTypeId::of::<T>())?.get(),
            _marker: Default::default(),
        })
    }

    /// Retrieve a mutable reference to  `T` from the store if it exists. Otherwise, return `None`
    pub fn get_mut<T: Resource>(&self) -> Option<FetchMut<'_, T>> {
        Some(FetchMut {
            inner: self.storage.get(&ResourceTypeId::of::<T>())?.get_mut(),
            _marker: Default::default(),
        })
    }

    /// Attempts to retrieve an immutable reference to `T` from the store. If it does not exist,
    /// the closure `f` is called to construct the object and it is then inserted into the store.
    pub fn get_or_insert_with<T: Resource, F: FnOnce() -> T>(
        &mut self,
        f: F,
    ) -> Option<Fetch<'_, T>> {
        self.get_or_insert((f)())
    }

    /// Attempts to retrieve a mutable reference to `T` from the store. If it does not exist,
    /// the closure `f` is called to construct the object and it is then inserted into the store.
    pub fn get_mut_or_insert_with<T: Resource, F: FnOnce() -> T>(
        &mut self,
        f: F,
    ) -> Option<FetchMut<'_, T>> {
        self.get_mut_or_insert((f)())
    }

    /// Attempts to retrieve an immutable reference to `T` from the store. If it does not exist,
    /// the provided value is inserted and then a reference to it is returned.
    pub fn get_or_insert<T: Resource>(&mut self, value: T) -> Option<Fetch<'_, T>> {
        Some(Fetch {
            inner: self
                .storage
                .entry(ResourceTypeId::of::<T>())
                .or_insert_with(|| AtomicRefCell::new(Box::new(value)))
                .get(),
            _marker: Default::default(),
        })
    }

    /// Attempts to retrieve a mutable reference to `T` from the store. If it does not exist,
    /// the provided value is inserted and then a reference to it is returned.
    pub fn get_mut_or_insert<T: Resource>(&mut self, value: T) -> Option<FetchMut<'_, T>> {
        Some(FetchMut {
            inner: self
                .storage
                .entry(ResourceTypeId::of::<T>())
                .or_insert_with(|| AtomicRefCell::new(Box::new(value)))
                .get_mut(),
            _marker: Default::default(),
        })
    }

    /// Attempts to retrieve an immutable reference to `T` from the store. If it does not exist,
    /// the default constructor for `T` is called.
    ///
    /// `T` must implement `Default` for this method.
    pub fn get_or_default<T: Resource + Default>(&mut self) -> Option<Fetch<'_, T>> {
        Some(Fetch {
            inner: self
                .storage
                .entry(ResourceTypeId::of::<T>())
                .or_insert_with(|| AtomicRefCell::new(Box::new(T::default())))
                .get(),
            _marker: Default::default(),
        })
    }

    /// Attempts to retrieve a mutable reference to `T` from the store. If it does not exist,
    /// the default constructor for `T` is called.
    ///
    /// `T` must implement `Default` for this method.
    pub fn get_mut_or_default<T: Resource + Default>(&mut self) -> Option<FetchMut<'_, T>> {
        Some(FetchMut {
            inner: self
                .storage
                .entry(ResourceTypeId::of::<T>())
                .or_insert_with(|| AtomicRefCell::new(Box::new(T::default())))
                .get_mut(),
            _marker: Default::default(),
        })
    }

    /// Performs merging of two resource storages, which occurs during a world merge.
    /// This merge will retain any already-existant resources in the local world, while moving any
    /// new resources from the source world into this one, consuming the resources.
    pub fn merge(&mut self, mut other: Resources) {
        // Merge resources, retaining our local ones but moving in any non-existant ones
        for resource in other.storage.drain() {
            self.storage.entry(resource.0).or_insert(resource.1);
        }
    }
}

impl ResourceSet for () {
    type PreparedResources = ();

    unsafe fn fetch_unchecked(_: &Resources) {}
    fn read_types() -> Vec<ResourceTypeId> { Vec::new() }
    fn write_types() -> Vec<ResourceTypeId> { Vec::new() }
}

impl<T: Resource> ResourceSet for Read<T> {
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
impl<T: Resource> ResourceSet for Write<T> {
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

macro_rules! impl_resource_tuple {
    ( $( $ty: ident ),* ) => {
        #[allow(unused_parens, non_snake_case)]
        impl<$( $ty: ResourceSet ),*> ResourceSet for ($( $ty, )*)
        {
            type PreparedResources = ($( $ty::PreparedResources, )*);

            unsafe fn fetch_unchecked(resources: &Resources) -> Self::PreparedResources {
                ($( $ty::fetch_unchecked(resources), )*)
            }
            fn read_types() -> Vec<ResourceTypeId> {
                let mut vec = vec![];
                $( vec.extend($ty::read_types()); )*
                vec
            }
            fn write_types() -> Vec<ResourceTypeId> {
                let mut vec = vec![];
                $( vec.extend($ty::write_types()); )*
                vec
            }
        }
    };
}
//($( $ty, )*)

impl_resource_tuple!(A);
impl_resource_tuple!(A, B);
impl_resource_tuple!(A, B, C);
impl_resource_tuple!(A, B, C, D);
impl_resource_tuple!(A, B, C, D, E);
impl_resource_tuple!(A, B, C, D, E, F);
impl_resource_tuple!(A, B, C, D, E, F, G);
impl_resource_tuple!(A, B, C, D, E, F, G, H);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y);
impl_resource_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_read_write_test() {
        let _ = tracing_subscriber::fmt::try_init();

        struct TestOne {
            value: String,
        }

        struct TestTwo {
            value: String,
        }

        let mut resources = Resources::default();
        resources.insert(TestOne {
            value: "poop".to_string(),
        });

        resources.insert(TestTwo {
            value: "balls".to_string(),
        });

        assert_eq!(resources.get::<TestOne>().unwrap().value, "poop");
        assert_eq!(resources.get::<TestTwo>().unwrap().value, "balls");

        // test re-ownership
        let owned = resources.remove::<TestTwo>();
        assert_eq!(owned.unwrap().value, "balls")
    }
}
