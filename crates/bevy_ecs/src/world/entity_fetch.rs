use alloc::vec::Vec;
use core::mem::MaybeUninit;

use crate::{
    entity::{
        hash_map::EntityHashMap, hash_set::EntityHashSet, Entity, EntityDoesNotExistError,
        UniqueEntitySlice,
    },
    query::{QueryData, QueryEntityError, QueryFilter, QueryItem},
    system::Query,
    world::{
        error::EntityMutableFetchError, unsafe_world_cell::UnsafeWorldCell, EntityMut, EntityRef,
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
/// - The slice and array implementations perform an aliased mutability check
///   in [`WorldEntityFetch::fetch_mut`] that is `O(N^2)`.
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

    /// The query data returned by [`Query::get_inner`].
    type Data<'w, D: QueryData>;

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
    /// - Returns [`EntityDoesNotExistError`] if the entity does not exist.
    unsafe fn fetch_ref(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityDoesNotExistError>;

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
    /// - Returns [`EntityMutableFetchError::EntityDoesNotExist`] if the entity does not exist.
    /// - Returns [`EntityMutableFetchError::AliasedMutability`] if the entity was
    ///   requested mutably more than once.
    unsafe fn fetch_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityMutableFetchError>;

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
    /// - Returns [`EntityMutableFetchError::EntityDoesNotExist`] if the entity does not exist.
    /// - Returns [`EntityMutableFetchError::AliasedMutability`] if the entity was
    ///   requested mutably more than once.
    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityMutableFetchError>;

    /// Returns query data for the entities with the given [`Entity`] IDs, as determined by `self`.
    ///
    /// # Errors
    ///
    /// - Returns [`EntityMutableFetchError::EntityDoesNotExist`] if the entity does not exist.
    /// - Returns [`EntityMutableFetchError::AliasedMutability`] if the entity was
    ///   requested mutably more than once and the query performs mutable access.
    fn fetch_query_data<'w, 's, D: QueryData, F: QueryFilter>(
        self,
        query: Query<'w, 's, D, F>,
    ) -> Result<Self::Data<'w, D>, QueryEntityError<'w>>;
}

// SAFETY:
// - No aliased mutability is caused because a single reference is returned.
// - No mutable references are returned by `fetch_ref`.
// - No structurally-mutable references are returned by `fetch_deferred_mut`.
unsafe impl WorldEntityFetch for Entity {
    type Ref<'w> = EntityRef<'w>;
    type Mut<'w> = EntityWorldMut<'w>;
    type DeferredMut<'w> = EntityMut<'w>;
    type Data<'w, D: QueryData> = QueryItem<'w, D>;

    unsafe fn fetch_ref(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityDoesNotExistError> {
        let ecell = cell.get_entity(self)?;
        // SAFETY: caller ensures that the world cell has read-only access to the entity.
        Ok(unsafe { EntityRef::new(ecell) })
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityMutableFetchError> {
        let location = cell
            .entities()
            .get(self)
            .ok_or(EntityDoesNotExistError::new(self, cell.entities()))?;
        // SAFETY: caller ensures that the world cell has mutable access to the entity.
        let world = unsafe { cell.world_mut() };
        // SAFETY: location was fetched from the same world's `Entities`.
        Ok(unsafe { EntityWorldMut::new(world, self, location) })
    }

    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityMutableFetchError> {
        let ecell = cell.get_entity(self)?;
        // SAFETY: caller ensures that the world cell has mutable access to the entity.
        Ok(unsafe { EntityMut::new(ecell) })
    }

    fn fetch_query_data<'w, 's, D: QueryData, F: QueryFilter>(
        self,
        query: Query<'w, 's, D, F>,
    ) -> Result<Self::Data<'w, D>, QueryEntityError<'w>> {
        // SAFETY: This is the only call made from this query
        unsafe { query.get_inner_unsafe(self) }
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
    type Data<'w, D: QueryData> = [QueryItem<'w, D>; N];

    unsafe fn fetch_ref(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityDoesNotExistError> {
        <&Self>::fetch_ref(&self, cell)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityMutableFetchError> {
        <&Self>::fetch_mut(&self, cell)
    }

    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityMutableFetchError> {
        <&Self>::fetch_deferred_mut(&self, cell)
    }

    fn fetch_query_data<'w, 's, D: QueryData, F: QueryFilter>(
        self,
        query: Query<'w, 's, D, F>,
    ) -> Result<Self::Data<'w, D>, QueryEntityError<'w>> {
        <&Self>::fetch_query_data(&self, query)
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
    type Data<'w, D: QueryData> = [QueryItem<'w, D>; N];

    unsafe fn fetch_ref(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityDoesNotExistError> {
        let mut refs = [MaybeUninit::uninit(); N];
        for (r, &id) in core::iter::zip(&mut refs, self) {
            let ecell = cell.get_entity(id)?;
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
    ) -> Result<Self::Mut<'_>, EntityMutableFetchError> {
        // Check for duplicate entities.
        for i in 0..self.len() {
            for j in 0..i {
                if self[i] == self[j] {
                    return Err(EntityMutableFetchError::AliasedMutability(self[i]));
                }
            }
        }

        let mut refs = [const { MaybeUninit::uninit() }; N];
        for (r, &id) in core::iter::zip(&mut refs, self) {
            let ecell = cell.get_entity(id)?;
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
    ) -> Result<Self::DeferredMut<'_>, EntityMutableFetchError> {
        // SAFETY: caller ensures that the world cell has mutable access to the entity,
        // and `fetch_mut` does not return structurally-mutable references.
        unsafe { self.fetch_mut(cell) }
    }

    fn fetch_query_data<'w, 's, D: QueryData, F: QueryFilter>(
        self,
        query: Query<'w, 's, D, F>,
    ) -> Result<Self::Data<'w, D>, QueryEntityError<'w>> {
        if !D::IS_READ_ONLY {
            // Check for duplicate entities.
            for i in 0..self.len() {
                for j in 0..i {
                    if self[i] == self[j] {
                        return Err(QueryEntityError::AliasedMutability(self[i]));
                    }
                }
            }
        }

        let mut values = [(); N].map(|_| MaybeUninit::uninit());

        for (value, &entity) in core::iter::zip(&mut values, self) {
            // SAFETY: We ensured that every entity is distinct
            let item = unsafe { query.get_inner_unsafe(entity) }?;
            *value = MaybeUninit::new(item);
        }

        // SAFETY: Each value has been fully initialized.
        Ok(values.map(|x| unsafe { x.assume_init() }))
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
    type Data<'w, D: QueryData> = Vec<QueryItem<'w, D>>;

    unsafe fn fetch_ref(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityDoesNotExistError> {
        let mut refs = Vec::with_capacity(self.len());
        for &id in self {
            let ecell = cell.get_entity(id)?;
            // SAFETY: caller ensures that the world cell has read-only access to the entity.
            refs.push(unsafe { EntityRef::new(ecell) });
        }

        Ok(refs)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityMutableFetchError> {
        // Check for duplicate entities.
        for i in 0..self.len() {
            for j in 0..i {
                if self[i] == self[j] {
                    return Err(EntityMutableFetchError::AliasedMutability(self[i]));
                }
            }
        }

        let mut refs = Vec::with_capacity(self.len());
        for &id in self {
            let ecell = cell.get_entity(id)?;
            // SAFETY: caller ensures that the world cell has mutable access to the entity.
            refs.push(unsafe { EntityMut::new(ecell) });
        }

        Ok(refs)
    }

    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityMutableFetchError> {
        // SAFETY: caller ensures that the world cell has mutable access to the entity,
        // and `fetch_mut` does not return structurally-mutable references.
        unsafe { self.fetch_mut(cell) }
    }

    fn fetch_query_data<'w, 's, D: QueryData, F: QueryFilter>(
        self,
        query: Query<'w, 's, D, F>,
    ) -> Result<Self::Data<'w, D>, QueryEntityError<'w>> {
        if !D::IS_READ_ONLY {
            // Check for duplicate entities.
            for i in 0..self.len() {
                for j in 0..i {
                    if self[i] == self[j] {
                        return Err(QueryEntityError::AliasedMutability(self[i]));
                    }
                }
            }
        }

        // SAFETY: We checked for duplicates above
        let entities = unsafe { UniqueEntitySlice::from_slice_unchecked(self) };
        Ok(query.iter_many_unique_inner(entities).collect())
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
    type Data<'w, D: QueryData> = EntityHashMap<QueryItem<'w, D>>;

    unsafe fn fetch_ref(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityDoesNotExistError> {
        let mut refs = EntityHashMap::with_capacity(self.len());
        for &id in self {
            let ecell = cell.get_entity(id)?;
            // SAFETY: caller ensures that the world cell has read-only access to the entity.
            refs.insert(id, unsafe { EntityRef::new(ecell) });
        }
        Ok(refs)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityMutableFetchError> {
        let mut refs = EntityHashMap::with_capacity(self.len());
        for &id in self {
            let ecell = cell.get_entity(id)?;
            // SAFETY: caller ensures that the world cell has mutable access to the entity.
            refs.insert(id, unsafe { EntityMut::new(ecell) });
        }
        Ok(refs)
    }

    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityMutableFetchError> {
        // SAFETY: caller ensures that the world cell has mutable access to the entity,
        // and `fetch_mut` does not return structurally-mutable references.
        unsafe { self.fetch_mut(cell) }
    }

    fn fetch_query_data<'w, 's, D: QueryData, F: QueryFilter>(
        self,
        query: Query<'w, 's, D, F>,
    ) -> Result<Self::Data<'w, D>, QueryEntityError<'w>> {
        let mut refs = EntityHashMap::with_capacity(self.len());
        for &id in self {
            // SAFETY: EntityHashset ensures that every entity is distinct
            refs.insert(id, unsafe { query.get_inner_unsafe(id) }?);
        }
        Ok(refs)
    }
}
