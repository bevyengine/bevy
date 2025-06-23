use alloc::vec::Vec;
use core::mem::MaybeUninit;

use crate::{
    entity::{Entity, EntityDoesNotExistError, EntityHashMap, EntityHashSet},
    error::Result,
    world::{
        error::EntityMutableFetchError, unsafe_world_cell::UnsafeWorldCell, EntityMut, EntityRef,
        EntityWorldMut,
    },
};

/// Provides a safe interface for non-structural access to the entities in a [`World`].
///
/// This cannot add or remove components, or spawn or despawn entities,
/// making it relatively safe to access in concert with other ECS data.
/// This type can be constructed via [`World::entities_and_commands`],
/// or [`DeferredWorld::entities_and_commands`].
///
/// [`World`]: crate::world::World
/// [`World::entities_and_commands`]: crate::world::World::entities_and_commands
/// [`DeferredWorld::entities_and_commands`]: crate::world::DeferredWorld::entities_and_commands
pub struct EntityFetcher<'w> {
    cell: UnsafeWorldCell<'w>,
}

impl<'w> EntityFetcher<'w> {
    // SAFETY:
    // - The given `cell` has mutable access to all entities.
    // - No other references to entities exist at the same time.
    pub(crate) unsafe fn new(cell: UnsafeWorldCell<'w>) -> Self {
        Self { cell }
    }

    /// Returns [`EntityRef`]s that expose read-only operations for the given
    /// `entities`, returning [`Err`] if any of the given entities do not exist.
    ///
    /// This function supports fetching a single entity or multiple entities:
    /// - Pass an [`Entity`] to receive a single [`EntityRef`].
    /// - Pass a slice of [`Entity`]s to receive a [`Vec<EntityRef>`].
    /// - Pass an array of [`Entity`]s to receive an equally-sized array of [`EntityRef`]s.
    /// - Pass a reference to a [`EntityHashSet`](crate::entity::EntityHashMap) to receive an
    ///   [`EntityHashMap<EntityRef>`](crate::entity::EntityHashMap).
    ///
    /// # Errors
    ///
    /// If any of the given `entities` do not exist in the world, the first
    /// [`Entity`] found to be missing will return an [`EntityDoesNotExistError`].
    ///
    /// # Examples
    ///
    /// For examples, see [`World::entity`].
    ///
    /// [`World::entity`]: crate::world::World::entity
    #[inline]
    pub fn get<F: WorldEntityFetch>(
        &self,
        entities: F,
    ) -> Result<F::Ref<'_>, EntityDoesNotExistError> {
        // SAFETY: `&self` gives read access to all entities, and prevents mutable access.
        unsafe { entities.fetch_ref(self.cell) }
    }

    /// Returns [`EntityMut`]s that expose read and write operations for the
    /// given `entities`, returning [`Err`] if any of the given entities do not
    /// exist.
    ///
    /// This function supports fetching a single entity or multiple entities:
    /// - Pass an [`Entity`] to receive a single [`EntityMut`].
    ///    - This reference type allows for structural changes to the entity,
    ///      such as adding or removing components, or despawning the entity.
    /// - Pass a slice of [`Entity`]s to receive a [`Vec<EntityMut>`].
    /// - Pass an array of [`Entity`]s to receive an equally-sized array of [`EntityMut`]s.
    /// - Pass a reference to a [`EntityHashSet`](crate::entity::EntityHashMap) to receive an
    ///   [`EntityHashMap<EntityMut>`](crate::entity::EntityHashMap).
    /// # Errors
    ///
    /// - Returns [`EntityMutableFetchError::EntityDoesNotExist`] if any of the given `entities` do not exist in the world.
    ///     - Only the first entity found to be missing will be returned.
    /// - Returns [`EntityMutableFetchError::AliasedMutability`] if the same entity is requested multiple times.
    ///
    /// # Examples
    ///
    /// For examples, see [`DeferredWorld::entity_mut`].
    ///
    /// [`DeferredWorld::entity_mut`]: crate::world::DeferredWorld::entity_mut
    #[inline]
    pub fn get_mut<F: WorldEntityFetch>(
        &mut self,
        entities: F,
    ) -> Result<F::DeferredMut<'_>, EntityMutableFetchError> {
        // SAFETY: `&mut self` gives mutable access to all entities,
        // and prevents any other access to entities.
        unsafe { entities.fetch_deferred_mut(self.cell) }
    }
}

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
}

// SAFETY:
// - No aliased mutability is caused because a single reference is returned.
// - No mutable references are returned by `fetch_ref`.
// - No structurally-mutable references are returned by `fetch_deferred_mut`.
unsafe impl WorldEntityFetch for Entity {
    type Ref<'w> = EntityRef<'w>;
    type Mut<'w> = EntityWorldMut<'w>;
    type DeferredMut<'w> = EntityMut<'w>;

    #[inline]
    unsafe fn fetch_ref(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityDoesNotExistError> {
        let ecell = cell.get_entity(self)?;
        // SAFETY: caller ensures that the world cell has read-only access to the entity.
        Ok(unsafe { EntityRef::new(ecell) })
    }

    #[inline]
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
        Ok(unsafe { EntityWorldMut::new(world, self, Some(location)) })
    }

    #[inline]
    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityMutableFetchError> {
        let ecell = cell.get_entity(self)?;
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

    #[inline]
    unsafe fn fetch_ref(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityDoesNotExistError> {
        <&Self>::fetch_ref(&self, cell)
    }

    #[inline]
    unsafe fn fetch_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityMutableFetchError> {
        <&Self>::fetch_mut(&self, cell)
    }

    #[inline]
    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityMutableFetchError> {
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

    #[inline]
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

    #[inline]
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

    #[inline]
    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityMutableFetchError> {
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

    #[inline]
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

    #[inline]
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

    #[inline]
    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityMutableFetchError> {
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

    #[inline]
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

    #[inline]
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

    #[inline]
    unsafe fn fetch_deferred_mut(
        self,
        cell: UnsafeWorldCell<'_>,
    ) -> Result<Self::DeferredMut<'_>, EntityMutableFetchError> {
        // SAFETY: caller ensures that the world cell has mutable access to the entity,
        // and `fetch_mut` does not return structurally-mutable references.
        unsafe { self.fetch_mut(cell) }
    }
}
