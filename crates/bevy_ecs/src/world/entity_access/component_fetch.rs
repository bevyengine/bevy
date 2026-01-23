use crate::{
    change_detection::MutUntyped,
    component::ComponentId,
    world::{error::EntityComponentError, unsafe_world_cell::UnsafeEntityCell, AsAccess},
};

use alloc::vec::Vec;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_ptr::Ptr;
use core::mem::MaybeUninit;

/// Types that can be used to fetch components from an entity dynamically by
/// [`ComponentId`]s.
///
/// Provided implementations are:
/// - [`ComponentId`]: Returns a single untyped reference.
/// - `[ComponentId; N]` and `&[ComponentId; N]`: Returns a same-sized array of untyped references.
/// - `&[ComponentId]`: Returns a [`Vec`] of untyped references.
/// - [`&HashSet<ComponentId>`](HashSet): Returns a [`HashMap`] of IDs to untyped references.
///
/// # Performance
///
/// - The slice and array implementations perform an aliased mutability check in
///   [`DynamicComponentFetch::fetch_mut`] that is `O(N^2)`.
/// - The [`HashSet`] implementation performs no such check as the type itself
///   guarantees unique IDs.
/// - The single [`ComponentId`] implementation performs no such check as only
///   one reference is returned.
///
/// # Safety
///
/// Implementor must ensure that:
/// - No aliased mutability is caused by the returned references.
/// - [`DynamicComponentFetch::fetch_ref`] returns only read-only references.
pub unsafe trait DynamicComponentFetch {
    /// The read-only reference type returned by [`DynamicComponentFetch::fetch_ref`].
    type Ref<'w>;

    /// The mutable reference type returned by [`DynamicComponentFetch::fetch_mut`].
    type Mut<'w>;

    /// Returns untyped read-only reference(s) to the component(s) with the
    /// given [`ComponentId`]s, as determined by `self`.
    ///
    /// # Safety
    ///
    /// Caller must ensure the provided [`AsAccess`] does not exceed the read
    /// permissions of `cell` in a way that would violate Rust's aliasing rules,
    /// including via copies of `cell` or other indirect means.
    ///
    /// # Errors
    ///
    /// - Returns [`EntityComponentError::MissingComponent`] if a component is missing from the entity.
    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Ref<'_>, EntityComponentError>;

    /// Returns untyped mutable reference(s) to the component(s) with the
    /// given [`ComponentId`]s, as determined by `self`.
    ///
    /// # Safety
    ///
    /// Caller must ensure the provided [`AsAccess`] does not exceed the write
    /// permissions of `cell` in a way that would violate Rust's aliasing rules,
    /// including via copies of `cell` or other indirect means.
    ///
    /// # Errors
    ///
    /// - Returns [`EntityComponentError::MissingComponent`] if a component is missing from the entity.
    /// - Returns [`EntityComponentError::AliasedMutability`] if a component is requested multiple times.
    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError>;

    /// Returns untyped mutable reference(s) to the component(s) with the
    /// given [`ComponentId`]s, as determined by `self`.
    /// Assumes all [`ComponentId`]s refer to mutable components.
    ///
    /// # Safety
    ///
    /// Caller must ensure that:
    /// - The provided [`AsAccess`] does not exceed the write permissions of
    ///   `cell` in a way that would violate Rust's aliasing rules, including
    ///   via copies of `cell` or other indirect means.
    /// - The requested components are all mutable.
    ///
    /// # Errors
    ///
    /// - Returns [`EntityComponentError::MissingComponent`] if a component is missing from the entity.
    /// - Returns [`EntityComponentError::AliasedMutability`] if a component is requested multiple times.
    unsafe fn fetch_mut_assume_mutable(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError>;
}

// SAFETY:
// - No aliased mutability is caused because a single reference is returned.
// - No mutable references are returned by `fetch_ref`.
unsafe impl DynamicComponentFetch for ComponentId {
    type Ref<'w> = Ptr<'w>;
    type Mut<'w> = MutUntyped<'w>;

    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Ref<'_>, EntityComponentError> {
        // SAFETY: caller ensures that the cell has read access to the component.
        unsafe { cell.get_by_id(access, self) }.ok_or(EntityComponentError::MissingComponent(self))
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        // SAFETY: caller ensures that the cell has mutable access to the component.
        unsafe { cell.get_mut_by_id(access, self) }
            .map_err(|_| EntityComponentError::MissingComponent(self))
    }

    unsafe fn fetch_mut_assume_mutable(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        // SAFETY: caller ensures that the cell has mutable access to the component.
        unsafe { cell.get_mut_assume_mutable_by_id(access, self) }
            .map_err(|_| EntityComponentError::MissingComponent(self))
    }
}

// SAFETY:
// - No aliased mutability is caused because the array is checked for duplicates.
// - No mutable references are returned by `fetch_ref`.
unsafe impl<const N: usize> DynamicComponentFetch for [ComponentId; N] {
    type Ref<'w> = [Ptr<'w>; N];
    type Mut<'w> = [MutUntyped<'w>; N];

    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Ref<'_>, EntityComponentError> {
        <&Self>::fetch_ref(&self, cell, access)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        <&Self>::fetch_mut(&self, cell, access)
    }

    unsafe fn fetch_mut_assume_mutable(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        <&Self>::fetch_mut_assume_mutable(&self, cell, access)
    }
}

// SAFETY:
// - No aliased mutability is caused because the array is checked for duplicates.
// - No mutable references are returned by `fetch_ref`.
unsafe impl<const N: usize> DynamicComponentFetch for &'_ [ComponentId; N] {
    type Ref<'w> = [Ptr<'w>; N];
    type Mut<'w> = [MutUntyped<'w>; N];

    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Ref<'_>, EntityComponentError> {
        let mut ptrs = [const { MaybeUninit::uninit() }; N];
        for (ptr, &id) in core::iter::zip(&mut ptrs, self) {
            *ptr = MaybeUninit::new(
                // SAFETY: caller ensures that the cell has read access to the component.
                unsafe { cell.get_by_id(access, id) }
                    .ok_or(EntityComponentError::MissingComponent(id))?,
            );
        }

        // SAFETY: Each ptr was initialized in the loop above.
        let ptrs = ptrs.map(|ptr| unsafe { MaybeUninit::assume_init(ptr) });

        Ok(ptrs)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        // Check for duplicate component IDs.
        for i in 0..self.len() {
            for j in 0..i {
                if self[i] == self[j] {
                    return Err(EntityComponentError::AliasedMutability(self[i]));
                }
            }
        }

        let mut ptrs = [const { MaybeUninit::uninit() }; N];
        for (ptr, &id) in core::iter::zip(&mut ptrs, self) {
            *ptr = MaybeUninit::new(
                // SAFETY: caller ensures that the cell has mutable access to the component.
                unsafe { cell.get_mut_by_id(access, id) }
                    .map_err(|_| EntityComponentError::MissingComponent(id))?,
            );
        }

        // SAFETY: Each ptr was initialized in the loop above.
        let ptrs = ptrs.map(|ptr| unsafe { MaybeUninit::assume_init(ptr) });

        Ok(ptrs)
    }

    unsafe fn fetch_mut_assume_mutable(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        // Check for duplicate component IDs.
        for i in 0..self.len() {
            for j in 0..i {
                if self[i] == self[j] {
                    return Err(EntityComponentError::AliasedMutability(self[i]));
                }
            }
        }

        let mut ptrs = [const { MaybeUninit::uninit() }; N];
        for (ptr, &id) in core::iter::zip(&mut ptrs, self) {
            *ptr = MaybeUninit::new(
                // SAFETY: caller ensures that the cell has mutable access to the component.
                unsafe { cell.get_mut_assume_mutable_by_id(access, id) }
                    .map_err(|_| EntityComponentError::MissingComponent(id))?,
            );
        }

        // SAFETY: Each ptr was initialized in the loop above.
        let ptrs = ptrs.map(|ptr| unsafe { MaybeUninit::assume_init(ptr) });

        Ok(ptrs)
    }
}

// SAFETY:
// - No aliased mutability is caused because the slice is checked for duplicates.
// - No mutable references are returned by `fetch_ref`.
unsafe impl DynamicComponentFetch for &'_ [ComponentId] {
    type Ref<'w> = Vec<Ptr<'w>>;
    type Mut<'w> = Vec<MutUntyped<'w>>;

    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Ref<'_>, EntityComponentError> {
        let mut ptrs = Vec::with_capacity(self.len());
        for &id in self {
            ptrs.push(
                // SAFETY: caller ensures that the cell has read access to the component.
                unsafe { cell.get_by_id(access, id) }
                    .ok_or(EntityComponentError::MissingComponent(id))?,
            );
        }
        Ok(ptrs)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        // Check for duplicate component IDs.
        for i in 0..self.len() {
            for j in 0..i {
                if self[i] == self[j] {
                    return Err(EntityComponentError::AliasedMutability(self[i]));
                }
            }
        }

        let mut ptrs = Vec::with_capacity(self.len());
        for &id in self {
            ptrs.push(
                // SAFETY: caller ensures that the cell has mutable access to the component.
                unsafe { cell.get_mut_by_id(access, id) }
                    .map_err(|_| EntityComponentError::MissingComponent(id))?,
            );
        }
        Ok(ptrs)
    }

    unsafe fn fetch_mut_assume_mutable(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        // Check for duplicate component IDs.
        for i in 0..self.len() {
            for j in 0..i {
                if self[i] == self[j] {
                    return Err(EntityComponentError::AliasedMutability(self[i]));
                }
            }
        }

        let mut ptrs = Vec::with_capacity(self.len());
        for &id in self {
            ptrs.push(
                // SAFETY: caller ensures that the cell has mutable access to the component.
                unsafe { cell.get_mut_assume_mutable_by_id(access, id) }
                    .map_err(|_| EntityComponentError::MissingComponent(id))?,
            );
        }
        Ok(ptrs)
    }
}

// SAFETY:
// - No aliased mutability is caused because `HashSet` guarantees unique elements.
// - No mutable references are returned by `fetch_ref`.
unsafe impl DynamicComponentFetch for &'_ HashSet<ComponentId> {
    type Ref<'w> = HashMap<ComponentId, Ptr<'w>>;
    type Mut<'w> = HashMap<ComponentId, MutUntyped<'w>>;

    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Ref<'_>, EntityComponentError> {
        let mut ptrs = HashMap::with_capacity_and_hasher(self.len(), Default::default());
        for &id in self {
            ptrs.insert(
                id,
                // SAFETY: caller ensures that the cell has read access to the component.
                unsafe { cell.get_by_id(access, id) }
                    .ok_or(EntityComponentError::MissingComponent(id))?,
            );
        }
        Ok(ptrs)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        let mut ptrs = HashMap::with_capacity_and_hasher(self.len(), Default::default());
        for &id in self {
            ptrs.insert(
                id,
                // SAFETY: caller ensures that the cell has mutable access to the component.
                unsafe { cell.get_mut_by_id(access, id) }
                    .map_err(|_| EntityComponentError::MissingComponent(id))?,
            );
        }
        Ok(ptrs)
    }

    unsafe fn fetch_mut_assume_mutable(
        self,
        cell: UnsafeEntityCell<'_>,
        access: impl AsAccess,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        let mut ptrs = HashMap::with_capacity_and_hasher(self.len(), Default::default());
        for &id in self {
            ptrs.insert(
                id,
                // SAFETY: caller ensures that the cell has mutable access to the component.
                unsafe { cell.get_mut_assume_mutable_by_id(access, id) }
                    .map_err(|_| EntityComponentError::MissingComponent(id))?,
            );
        }
        Ok(ptrs)
    }
}
