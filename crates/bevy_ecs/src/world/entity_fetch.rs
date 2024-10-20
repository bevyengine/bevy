use core::mem::MaybeUninit;

use crate::{
    entity::{Entity, EntityHash, EntityHashMap, EntityHashSet},
    world::{
        error::EntityFetchError, unsafe_world_cell::UnsafeWorldCell, EntityMut, EntityRef,
        EntityWorldMut,
    },
};

/// Types that can be used to fetch [`Entity`] references from a [`World`].
///
/// Provided implementations are:
/// - [`Entity`]: Fetch a single entity.
/// - `[Entity; N]`/`&[Entity; N]`: Fetch multiple entities, receiving a
///   same-sized array of references.
/// - `&[Entity]`: Fetch multiple entities, receiving a vector of references.
/// - [`&EntityHashSet`](EntityHashSet): Fetch multiple entities, receiving a
///   hash map of [`Entity`] IDs to references.
///
/// # Performance
///
/// - The slice and array implementations perform an aliased mutabiltiy check
///   in [`WorldEntityFetch::fetch_mut`] that is `O(N^2)`.
/// - The [`EntityHashSet`] implementation performs no such check as the type
///   itself guarantees no duplicates.
/// - The single [`Entity`] implementation performs no such check as only one
///   reference is returned.
///
/// # Safety
///
/// Implementor must ensure that:
/// - No aliased mutability is caused by the returned references.
/// - [`WorldEntityFetch::fetch_ref`] returns only read-only references.
/// - [`WorldEntityFetch::fetch_deferred_mut`] returns only non-structurally-mutable references.
///
/// [`World`]: crate::world::World
pub unsafe trait WorldEntityFetch {
    /// The read-only reference type returned by [`WorldEntityFetch::fetch_ref`].
    type Ref<'w>;

    /// The mutable reference type returned by [`WorldEntityFetch::fetch_mut`].
    type Mut<'w>;

    /// The mutable reference type returned by [`WorldEntityFetch::fetch_deferred_mut`],
    /// but without structural mutability.
    type DeferredMut<'w>;

    /// Returns read-only reference(s) to the entities with the given
    /// [`Entity`] IDs, as determined by `self`.
    ///
    /// # Safety
    ///
    /// It is the caller's responsibility to ensure that:
    /// - The given [`UnsafeWorldCell`] has read-only access to the fetched entities.
    /// - No other mutable references to the fetched entities exist at the same time.
    ///
    /// # Errors
    ///
    /// - Returns [`Entity`] if the entity does not exist.
    unsafe fn fetch_ref(self, cell: UnsafeWorldCell<'_>) -> Result<Self::Ref<'_>, Entity>;

    /// Returns mutable reference(s) to the entities with the given [`Entity`]
    /// IDs, as determined by `self`.
    ///
    /// # Safety
    ///
    /// It is the caller's responsibility to ensure that:
    /// - The given [`UnsafeWorldCell`] has mutable access to the fetched entities.
    /// - No other references to the fetched entities exist at the same time.
    ///
    /// # Errors
    ///
    /// - Returns [`EntityFetchError::NoSuchEntity`] if the entity does not exist.
    /// - Returns [`EntityFetchError::AliasedMutability`] if the entity was
    ///   requested mutably more than once.
    unsafe fn fetch_mut(self, cell: UnsafeWorldCell<'_>)
        -> Result<Self::Mut<'_>, EntityFetchError>;

    /// Returns mutable reference(s) to the entities with the given [`Entity`]
    /// IDs, as determined by `self`, but without structural mutability.
    ///
    /// No structural mutability means components cannot be removed from the
    /// entity, new components cannot be added to the entity, and the entity
    /// cannot be despawned.
    ///
    /// # Safety
    ///
    /// It is the caller's responsibility to ensure that:
    /// - The given [`UnsafeWorldCell`] has mutable access to the fetched entities.
    /// - No other references to the fetched entities exist at the same time.
    ///
    /// # Errors
    ///
    /// - Returns [`EntityFetchError::NoSuchEntity`] if the entity does not exist.
    /// - Returns [`EntityFetchError::AliasedMutability`] if the entity was
    ///   requested mutably more than once.
    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityFetchError>;
}

// SAFETY:
// - No aliased mutability is caused because a single reference is returned.
// - No mutable references are returned by `fetch_ref`.
// - No structurally-mutable references are returned by `fetch_deferred_mut`.
unsafe impl WorldEntityFetch for Entity {
    type Ref<'w> = EntityRef<'w>;
    type Mut<'w> = EntityWorldMut<'w>;
    type DeferredMut<'w> = EntityMut<'w>;

    unsafe fn fetch_ref(self, cell: UnsafeWorldCell<'_>) -> Result<Self::Ref<'_>, Entity> {
        let ecell = cell.get_entity(self).ok_or(self)?;
        // SAFETY: caller ensures that the world cell has read-only access to the entity.
        Ok(unsafe { EntityRef::new(ecell) })
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityFetchError> {
        let location = cell
            .entities()
            .get(self)
            .ok_or(EntityFetchError::NoSuchEntity(self))?;
        // SAFETY: caller ensures that the world cell has mutable access to the entity.
        let world = unsafe { cell.world_mut() };
        // SAFETY: location was fetched from the same world's `Entities`.
        Ok(unsafe { EntityWorldMut::new(world, self, location) })
    }

    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityFetchError> {
        let ecell = cell
            .get_entity(self)
            .ok_or(EntityFetchError::NoSuchEntity(self))?;
        // SAFETY: caller ensures that the world cell has mutable access to the entity.
        Ok(unsafe { EntityMut::new(ecell) })
    }
}

// SAFETY:
// - No aliased mutability is caused because the array is checked for duplicates.
// - No mutable references are returned by `fetch_ref`.
// - No structurally-mutable references are returned by `fetch_deferred_mut`.
unsafe impl<const N: usize> WorldEntityFetch for [Entity; N] {
    type Ref<'w> = [EntityRef<'w>; N];
    type Mut<'w> = [EntityMut<'w>; N];
    type DeferredMut<'w> = [EntityMut<'w>; N];

    unsafe fn fetch_ref(self, cell: UnsafeWorldCell<'_>) -> Result<Self::Ref<'_>, Entity> {
        <&Self>::fetch_ref(&self, cell)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityFetchError> {
        <&Self>::fetch_mut(&self, cell)
    }

    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityFetchError> {
        <&Self>::fetch_deferred_mut(&self, cell)
    }
}

// SAFETY:
// - No aliased mutability is caused because the array is checked for duplicates.
// - No mutable references are returned by `fetch_ref`.
// - No structurally-mutable references are returned by `fetch_deferred_mut`.
unsafe impl<const N: usize> WorldEntityFetch for &'_ [Entity; N] {
    type Ref<'w> = [EntityRef<'w>; N];
    type Mut<'w> = [EntityMut<'w>; N];
    type DeferredMut<'w> = [EntityMut<'w>; N];

    unsafe fn fetch_ref(self, cell: UnsafeWorldCell<'_>) -> Result<Self::Ref<'_>, Entity> {
        let mut refs = [MaybeUninit::uninit(); N];
        for (r, &id) in core::iter::zip(&mut refs, self) {
            let ecell = cell.get_entity(id).ok_or(id)?;
            // SAFETY: caller ensures that the world cell has read-only access to the entity.
            *r = MaybeUninit::new(unsafe { EntityRef::new(ecell) });
        }

        // SAFETY: Each item was initialized in the loop above.
        let refs = refs.map(|r| unsafe { MaybeUninit::assume_init(r) });

        Ok(refs)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityFetchError> {
        // Check for duplicate entities.
        for i in 0..self.len() {
            for j in 0..i {
                if self[i] == self[j] {
                    return Err(EntityFetchError::AliasedMutability(self[i]));
                }
            }
        }

        let mut refs = [const { MaybeUninit::uninit() }; N];
        for (r, &id) in core::iter::zip(&mut refs, self) {
            let ecell = cell
                .get_entity(id)
                .ok_or(EntityFetchError::NoSuchEntity(id))?;
            // SAFETY: caller ensures that the world cell has mutable access to the entity.
            *r = MaybeUninit::new(unsafe { EntityMut::new(ecell) });
        }

        // SAFETY: Each item was initialized in the loop above.
        let refs = refs.map(|r| unsafe { MaybeUninit::assume_init(r) });

        Ok(refs)
    }

    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityFetchError> {
        // SAFETY: caller ensures that the world cell has mutable access to the entity,
        // and `fetch_mut` does not return structurally-mutable references.
        unsafe { self.fetch_mut(cell) }
    }
}

// SAFETY:
// - No aliased mutability is caused because the slice is checked for duplicates.
// - No mutable references are returned by `fetch_ref`.
// - No structurally-mutable references are returned by `fetch_deferred_mut`.
unsafe impl WorldEntityFetch for &'_ [Entity] {
    type Ref<'w> = Vec<EntityRef<'w>>;
    type Mut<'w> = Vec<EntityMut<'w>>;
    type DeferredMut<'w> = Vec<EntityMut<'w>>;

    unsafe fn fetch_ref(self, cell: UnsafeWorldCell<'_>) -> Result<Self::Ref<'_>, Entity> {
        let mut refs = Vec::with_capacity(self.len());
        for &id in self {
            let ecell = cell.get_entity(id).ok_or(id)?;
            // SAFETY: caller ensures that the world cell has read-only access to the entity.
            refs.push(unsafe { EntityRef::new(ecell) });
        }

        Ok(refs)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityFetchError> {
        // Check for duplicate entities.
        for i in 0..self.len() {
            for j in 0..i {
                if self[i] == self[j] {
                    return Err(EntityFetchError::AliasedMutability(self[i]));
                }
            }
        }

        let mut refs = Vec::with_capacity(self.len());
        for &id in self {
            let ecell = cell
                .get_entity(id)
                .ok_or(EntityFetchError::NoSuchEntity(id))?;
            // SAFETY: caller ensures that the world cell has mutable access to the entity.
            refs.push(unsafe { EntityMut::new(ecell) });
        }

        Ok(refs)
    }

    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityFetchError> {
        // SAFETY: caller ensures that the world cell has mutable access to the entity,
        // and `fetch_mut` does not return structurally-mutable references.
        unsafe { self.fetch_mut(cell) }
    }
}

// SAFETY:
// - No aliased mutability is caused because `EntityHashSet` guarantees no duplicates.
// - No mutable references are returned by `fetch_ref`.
// - No structurally-mutable references are returned by `fetch_deferred_mut`.
unsafe impl WorldEntityFetch for &'_ EntityHashSet {
    type Ref<'w> = EntityHashMap<EntityRef<'w>>;
    type Mut<'w> = EntityHashMap<EntityMut<'w>>;
    type DeferredMut<'w> = EntityHashMap<EntityMut<'w>>;

    unsafe fn fetch_ref(self, cell: UnsafeWorldCell<'_>) -> Result<Self::Ref<'_>, Entity> {
        let mut refs = EntityHashMap::with_capacity_and_hasher(self.len(), EntityHash);
        for &id in self {
            let ecell = cell.get_entity(id).ok_or(id)?;
            // SAFETY: caller ensures that the world cell has read-only access to the entity.
            refs.insert(id, unsafe { EntityRef::new(ecell) });
        }
        Ok(refs)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityFetchError> {
        let mut refs = EntityHashMap::with_capacity_and_hasher(self.len(), EntityHash);
        for &id in self {
            let ecell = cell
                .get_entity(id)
                .ok_or(EntityFetchError::NoSuchEntity(id))?;
            // SAFETY: caller ensures that the world cell has mutable access to the entity.
            refs.insert(id, unsafe { EntityMut::new(ecell) });
        }
        Ok(refs)
    }

    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityFetchError> {
        // SAFETY: caller ensures that the world cell has mutable access to the entity,
        // and `fetch_mut` does not return structurally-mutable references.
        unsafe { self.fetch_mut(cell) }
    }
}
