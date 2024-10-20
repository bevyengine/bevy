#[cfg(feature = "reflect")]
use bevy_ecs::reflect::{
    ReflectComponent, ReflectFromWorld, ReflectMapEntities, ReflectVisitEntities,
    ReflectVisitEntitiesMut,
};
use bevy_ecs::{
    component::Component,
    entity::{Entity, VisitEntitiesMut},
};
use core::{fmt::Debug, marker::PhantomData, ops::Deref};
use smallvec::{smallvec, SmallVec};

use crate::relationship::Relationship;

/// Represents one half of a many-to-many relationship between an [`Entity`] and some number of other [entities](Entity).
///
/// The type of relationship is denoted by the parameters `FK` and `PK`, shorthand
/// for Primary Key and Foreign Key.
/// An undirected relationship would have equal `FK` and `PK` types.
/// Whereas, an directed relationship would have differing parameters.
#[derive(Component)]
#[component(
    on_insert = <Self as Relationship>::associate,
    on_replace = <Self as Relationship>::disassociate,
)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(
    feature = "reflect",
    reflect(
        Component,
        MapEntities,
        VisitEntities,
        VisitEntitiesMut,
        PartialEq,
        Debug,
        FromWorld
    )
)]
pub struct ManyToMany<FK, PK = FK> {
    // [Entity; 7] chosen to keep entities at 64 bytes on 64 bit platforms.
    entities: SmallVec<[Entity; 7]>,
    #[cfg_attr(feature = "reflect", reflect(ignore))]
    _phantom: PhantomData<fn(&FK, &PK)>,
}

impl<FK: 'static, PK: 'static> Relationship for ManyToMany<FK, PK> {
    type Other = ManyToMany<PK, FK>;

    fn has(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }

    fn new(entity: Entity) -> Self {
        Self {
            entities: smallvec![entity],
            _phantom: PhantomData,
        }
    }

    fn with(mut self, entity: Entity) -> Self {
        if !self.has(entity) {
            self.entities.push(entity);
        }

        self
    }

    fn without(mut self, entity: Entity) -> Option<Self> {
        self.entities.retain(|&mut id| id != entity);

        (!self.entities.is_empty()).then_some(self)
    }

    fn iter(&self) -> impl ExactSizeIterator<Item = Entity> {
        self.entities.iter().copied()
    }
}

impl<FK, PK> PartialEq for ManyToMany<FK, PK> {
    fn eq(&self, other: &Self) -> bool {
        self.entities == other.entities
    }
}

impl<FK, PK> Eq for ManyToMany<FK, PK> {}

impl<FK, PK> Debug for ManyToMany<FK, PK> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Has Many {:?} ({}) With Many ({})",
            self.entities,
            core::any::type_name::<FK>(),
            core::any::type_name::<PK>()
        )
    }
}

impl<FK, PK> VisitEntitiesMut for ManyToMany<FK, PK> {
    fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, mut f: F) {
        for entity in &mut self.entities {
            f(entity);
        }
    }
}

impl<FK, PK> Deref for ManyToMany<FK, PK> {
    type Target = [Entity];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.entities
    }
}

impl<'a, FK, PK> IntoIterator for &'a ManyToMany<FK, PK> {
    type Item = <Self::IntoIter as Iterator>::Item;

    type IntoIter = core::slice::Iter<'a, Entity>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.entities.iter()
    }
}

impl<FK, PK> FromIterator<Entity> for ManyToMany<FK, PK> {
    fn from_iter<T: IntoIterator<Item = Entity>>(iter: T) -> Self {
        Self::from_smallvec(iter.into_iter().collect())
    }
}

impl<FK, PK> Default for ManyToMany<FK, PK> {
    fn default() -> Self {
        Self::new()
    }
}

impl<FK, PK> ManyToMany<FK, PK> {
    /// Gets the other [`Entity`] as a slice of length 1.
    #[inline(always)]
    pub fn as_slice(&self) -> &[Entity] {
        &self.entities
    }

    /// Create a new relationship.
    #[inline(always)]
    #[must_use]
    pub fn new() -> Self {
        Self::from_smallvec(SmallVec::new())
    }

    #[inline(always)]
    #[must_use]
    fn from_smallvec(entities: SmallVec<[Entity; 7]>) -> Self {
        Self {
            entities,
            _phantom: PhantomData,
        }
    }

    /// Ensures the provided [`Entity`] is present in this relationship.
    #[inline(always)]
    #[must_use]
    pub fn with(mut self, other: Entity) -> Self {
        if !self.entities.contains(&other) {
            self.entities.push(other);
        }
        self
    }

    /// Ensures the provided [`Entity`] is _not_ present in this relationship.
    #[inline(always)]
    #[must_use]
    pub fn without(mut self, other: Entity) -> Self {
        self.entities.retain(|&mut e| e != other);
        self
    }

    /// Swaps the entity at `a_index` with the entity at `b_index`.
    ///
    /// # Panics
    ///
    /// Will panic if either index is out-of-bounds.
    #[inline]
    pub fn swap(&mut self, a_index: usize, b_index: usize) {
        self.entities.swap(a_index, b_index);
    }

    /// Sorts entities [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided comparator function.
    ///
    /// For the underlying implementation, see [`slice::sort_by`].
    ///
    /// For the unstable version, see [`sort_unstable_by`](ManyToMany::sort_unstable_by).
    ///
    /// See also [`sort_by_key`](ManyToMany::sort_by_key), [`sort_by_cached_key`](ManyToMany::sort_by_cached_key).
    #[inline]
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Entity, &Entity) -> core::cmp::Ordering,
    {
        self.entities.sort_by(compare);
    }

    /// Sorts entities [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function.
    ///
    /// For the underlying implementation, see [`slice::sort_by_key`].
    ///
    /// For the unstable version, see [`sort_unstable_by_key`](ManyToMany::sort_unstable_by_key).
    ///
    /// See also [`sort_by`](ManyToMany::sort_by), [`sort_by_cached_key`](ManyToMany::sort_by_cached_key).
    #[inline]
    pub fn sort_by_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.entities.sort_by_key(compare);
    }

    /// Sorts entities [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function. Only evaluates each key at most
    /// once per sort, caching the intermediate results in memory.
    ///
    /// For the underlying implementation, see [`slice::sort_by_cached_key`].
    ///
    /// See also [`sort_by`](ManyToMany::sort_by), [`sort_by_key`](ManyToMany::sort_by_key).
    #[inline]
    pub fn sort_by_cached_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.entities.sort_by_cached_key(compare);
    }

    /// Sorts entities [unstably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided comparator function.
    ///
    /// For the underlying implementation, see [`slice::sort_unstable_by`].
    ///
    /// For the stable version, see [`sort_by`](ManyToMany::sort_by).
    ///
    /// See also [`sort_unstable_by_key`](ManyToMany::sort_unstable_by_key).
    #[inline]
    pub fn sort_unstable_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Entity, &Entity) -> core::cmp::Ordering,
    {
        self.entities.sort_unstable_by(compare);
    }

    /// Sorts entities [unstably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function.
    ///
    /// For the underlying implementation, see [`slice::sort_unstable_by_key`].
    ///
    /// For the stable version, see [`sort_by_key`](ManyToMany::sort_by_key).
    ///
    /// See also [`sort_unstable_by`](ManyToMany::sort_unstable_by).
    #[inline]
    pub fn sort_unstable_by_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.entities.sort_unstable_by_key(compare);
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::world::World;

    use super::ManyToMany;

    /// A familial relationship
    struct Friendship;

    /// Shorthand for a group of friends
    type Friends = ManyToMany<Friendship>;

    #[test]
    fn simple_add_then_remove() {
        let mut world = World::new();

        world.register_component::<Friends>();

        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn(Friends::new().with(a).with(b)).id();

        world.flush();

        assert_eq!(
            world
                .get::<Friends>(a)
                .map(|c| c.iter().copied().collect::<Vec<_>>()),
            Some(vec![c])
        );
        assert_eq!(
            world
                .get::<Friends>(b)
                .map(|c| c.iter().copied().collect::<Vec<_>>()),
            Some(vec![c])
        );
        assert_eq!(
            world
                .get::<Friends>(c)
                .map(|c| c.iter().copied().collect::<Vec<_>>()),
            Some(vec![a, b])
        );

        world.entity_mut(a).remove::<Friends>();

        world.flush();

        assert_eq!(world.get::<Friends>(a), None);
        assert_eq!(
            world
                .get::<Friends>(b)
                .map(|c| c.iter().copied().collect::<Vec<_>>()),
            Some(vec![c])
        );
        assert_eq!(
            world
                .get::<Friends>(c)
                .map(|c| c.iter().copied().collect::<Vec<_>>()),
            Some(vec![b])
        );
    }
}
