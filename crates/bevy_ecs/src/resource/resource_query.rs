use crate::{
    system::{SystemId, TypeAccess},
};
use core::{
    any::TypeId,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};
use hecs::{smaller_tuples_too, MissingComponent, Component, Archetype};
use std::collections::HashMap;
use super::{Resources, FromResources};
/// Shared borrow of an entity's component
pub struct Res<'a, T: Component> {
    archetype: &'a Archetype,
    target: NonNull<T>,
}

impl<'a, T: Component> Res<'a, T> {
    #[allow(missing_docs)]
    pub unsafe fn new(archetype: &'a Archetype, index: u32) -> Result<Self, MissingComponent> {
        let target = NonNull::new_unchecked(
            archetype
                .get::<T>()
                .ok_or_else(MissingComponent::new::<T>)?
                .as_ptr()
                .add(index as usize),
        );
        Ok(Self { archetype, target })
    }
}

pub trait UnsafeClone {
    unsafe fn unsafe_clone(&self) -> Self;
}

// TODO: this is unsafe. lets think of a better solution that allows us to clone internally
impl<'a, T: Component> UnsafeClone for ResMut<'a, T> {
    unsafe fn unsafe_clone(&self) -> Self {
        Self {
            archetype: self.archetype,
            target: self.target,
        }
    }
}

impl<'a, T: Component> UnsafeClone for Res<'a, T> {
    unsafe fn unsafe_clone(&self) -> Self {
        Self {
            archetype: self.archetype,
            target: self.target,
        }
    }
}

unsafe impl<T: Component> Send for Res<'_, T> {}
unsafe impl<T: Component> Sync for Res<'_, T> {}

impl<'a, T: Component> Deref for Res<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.target.as_ref() }
    }
}

/// Unique borrow of a resource
pub struct ResMut<'a, T: Component> {
    archetype: &'a Archetype,
    target: NonNull<T>,
}

impl<'a, T: Component> ResMut<'a, T> {
    #[allow(missing_docs)]
    pub unsafe fn new(archetype: &'a Archetype, index: u32) -> Result<Self, MissingComponent> {
        let target = NonNull::new_unchecked(
            archetype
                .get::<T>()
                .ok_or_else(MissingComponent::new::<T>)?
                .as_ptr()
                .add(index as usize),
        );
        Ok(Self { archetype, target })
    }
}

unsafe impl<T: Component> Send for ResMut<'_, T> {}
unsafe impl<T: Component> Sync for ResMut<'_, T> {}

impl<'a, T: Component> Deref for ResMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.target.as_ref() }
    }
}

impl<'a, T: Component> DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.target.as_mut() }
    }
}

pub struct Local<'a, T: Component + FromResources> {
    archetype: &'a Archetype,
    target: NonNull<T>,
}

impl<'a, T: Component + FromResources> Local<'a, T> {
    #[allow(missing_docs)]
    pub unsafe fn new(archetype: &'a Archetype, index: u32) -> Result<Self, MissingComponent> {
        let target = NonNull::new_unchecked(
            archetype
                .get::<T>()
                .ok_or_else(MissingComponent::new::<T>)?
                .as_ptr()
                .add(index as usize),
        );
        Ok(Self { archetype, target })
    }
}

impl<'a, T: Component + FromResources> UnsafeClone for Local<'a, T> {
    unsafe fn unsafe_clone(&self) -> Self {
        Self {
            archetype: self.archetype,
            target: self.target,
        }
    }
}

impl<'a, T: Component + FromResources> Deref for Local<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.target.as_ref() }
    }
}

impl<'a, T: Component + FromResources> DerefMut for Local<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.target.as_mut() }
    }
}

/// A collection of component types to fetch from a `World`
pub trait ResourceQuery {
    type Fetch: for<'a> FetchResource<'a>;

    fn initialize(_resources: &mut Resources, _system_id: Option<SystemId>) {}
}

/// Streaming iterators over contiguous homogeneous ranges of components
pub trait FetchResource<'a>: Sized {
    /// Type of value to be fetched
    type Item: UnsafeClone;

    fn access() -> TypeAccess;
    fn borrow(resource_archetypes: &HashMap<TypeId, Archetype>);
    fn release(resource_archetypes: &HashMap<TypeId, Archetype>);

    /// Construct a `Fetch` for `archetype` if it should be traversed
    ///
    /// # Safety
    /// `offset` must be in bounds of `archetype`
    unsafe fn get(resources: &'a Resources, system_id: Option<SystemId>) -> Self::Item;
}

impl<'a, T: Component> ResourceQuery for Res<'a, T> {
    type Fetch = FetchResourceRead<T>;
}

pub struct FetchResourceRead<T>(NonNull<T>);

impl<'a, T: Component> FetchResource<'a> for FetchResourceRead<T> {
    type Item = Res<'a, T>;
    unsafe fn get(resources: &'a Resources, _system_id: Option<SystemId>) -> Self::Item {
        resources.get_res::<T>()
    }

    fn borrow(resource_archetypes: &HashMap<TypeId, Archetype>) {
        if let Some(archetype) = resource_archetypes.get(&TypeId::of::<T>()) {
            archetype.borrow::<T>();
        }
    }
    fn release(resource_archetypes: &HashMap<TypeId, Archetype>) {
        if let Some(archetype) = resource_archetypes.get(&TypeId::of::<T>()) {
            archetype.release::<T>();
        }
    }
    fn access() -> TypeAccess {
        let mut access = TypeAccess::default();
        access.immutable.insert(TypeId::of::<T>());
        access
    }
}

impl<'a, T: Component> ResourceQuery for ResMut<'a, T> {
    type Fetch = FetchResourceWrite<T>;
}

pub struct FetchResourceWrite<T>(NonNull<T>);

impl<'a, T: Component> FetchResource<'a> for FetchResourceWrite<T> {
    type Item = ResMut<'a, T>;
    unsafe fn get(resources: &'a Resources, _system_id: Option<SystemId>) -> Self::Item {
        resources.get_res_mut::<T>()
    }

    fn borrow(resource_archetypes: &HashMap<TypeId, Archetype>) {
        if let Some(archetype) = resource_archetypes.get(&TypeId::of::<T>()) {
            archetype.borrow_mut::<T>();
        }
    }
    fn release(resource_archetypes: &HashMap<TypeId, Archetype>) {
        if let Some(archetype) = resource_archetypes.get(&TypeId::of::<T>()) {
            archetype.release_mut::<T>();
        }
    }
    fn access() -> TypeAccess {
        let mut access = TypeAccess::default();
        access.mutable.insert(TypeId::of::<T>());
        access
    }
}

impl<'a, T: Component + FromResources> ResourceQuery for Local<'a, T> {
    type Fetch = FetchResourceLocalMut<T>;

    fn initialize(resources: &mut Resources, id: Option<SystemId>) {
        let value = T::from_resources(resources);
        let id = id.expect("Local<T> resources can only be used by systems");
        resources.insert_local(id, value);
    }
}

pub struct FetchResourceLocalMut<T>(NonNull<T>);

impl<'a, T: Component + FromResources> FetchResource<'a> for FetchResourceLocalMut<T> {
    type Item = Local<'a, T>;
    unsafe fn get(resources: &'a Resources, system_id: Option<SystemId>) -> Self::Item {
        if let Some(system_id) = system_id {
            let archetype = resources
                .resource_archetypes
                .get(&TypeId::of::<T>())
                .unwrap_or_else(|| {
                    panic!("Resource does not exist {}", std::any::type_name::<T>())
                });
            let index = resources
                .system_id_to_archetype_index
                .get(&system_id.0)
                .expect("System does not have this resource");
            Local::new(archetype, *index).expect("Resource does not exist")
        } else {
            panic!("Only Systems can use Local<T> resources");
        }
    }

    fn borrow(resource_archetypes: &HashMap<TypeId, Archetype>) {
        if let Some(archetype) = resource_archetypes.get(&TypeId::of::<T>()) {
            archetype.borrow_mut::<T>();
        }
    }
    fn release(resource_archetypes: &HashMap<TypeId, Archetype>) {
        if let Some(archetype) = resource_archetypes.get(&TypeId::of::<T>()) {
            archetype.release_mut::<T>();
        }
    }
    fn access() -> TypeAccess {
        let mut access = TypeAccess::default();
        access.mutable.insert(TypeId::of::<T>());
        access
    }
}

macro_rules! tuple_impl {
    ($($name: ident),*) => {
        impl<'a, $($name: FetchResource<'a>),*> FetchResource<'a> for ($($name,)*) {
            type Item = ($($name::Item,)*);

            #[allow(unused_variables)]
            fn borrow(resource_archetypes: &HashMap<TypeId, Archetype>) {
                $($name::borrow(resource_archetypes);)*
            }

            #[allow(unused_variables)]
            fn release(resource_archetypes: &HashMap<TypeId, Archetype>) {
                $($name::release(resource_archetypes);)*
            }

            #[allow(unused_variables)]
            unsafe fn get(resources: &'a Resources, system_id: Option<SystemId>) -> Self::Item {
                ($($name::get(resources, system_id),)*)
            }

            #[allow(unused_mut)]
            fn access() -> TypeAccess {
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
