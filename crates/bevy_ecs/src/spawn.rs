//! Entity spawning abstractions, largely focused on spawning related hierarchies of entities. See [`related`](crate::related) and [`SpawnRelated`]
//! for the best entry points into these APIs and examples of how to use them.

use crate::{
    bundle::{Bundle, BundleEffect, DynamicBundle, NoBundleEffect},
    entity::Entity,
    relationship::{RelatedSpawner, Relationship, RelationshipTarget},
    world::{EntityWorldMut, World},
};
use alloc::vec::Vec;
use core::marker::PhantomData;
use variadics_please::all_tuples;

/// A wrapper over a [`Bundle`] indicating that an entity should be spawned with that [`Bundle`].
/// This is intended to be used for hierarchical spawning via traits like [`SpawnableList`] and [`SpawnRelated`].
///
/// Also see the [`children`](crate::children) and [`related`](crate::related) macros that abstract over the [`Spawn`] API.
///
/// ```
/// # use bevy_ecs::hierarchy::Children;
/// # use bevy_ecs::spawn::{Spawn, SpawnRelated};
/// # use bevy_ecs::name::Name;
/// # use bevy_ecs::world::World;
/// let mut world = World::new();
/// world.spawn((
///     Name::new("Root"),
///     Children::spawn((
///         Spawn(Name::new("Child1")),
///         Spawn((
///             Name::new("Child2"),
///             Children::spawn(Spawn(Name::new("Grandchild"))),
///         ))
///     )),
/// ));
/// ```
pub struct Spawn<B: Bundle>(pub B);

/// A spawn-able list of changes to a given [`World`] and relative to a given [`Entity`]. This is generally used
/// for spawning "related" entities, such as children.
pub trait SpawnableList<R> {
    /// Spawn this list of changes in a given [`World`] and relative to a given [`Entity`]. This is generally used
    /// for spawning "related" entities, such as children.
    fn spawn(self, world: &mut World, entity: Entity);
    /// Returns a size hint, which is used to reserve space for this list in a [`RelationshipTarget`]. This should be
    /// less than or equal to the actual size of the list. When in doubt, just use 0.
    fn size_hint(&self) -> usize;
}

impl<R: Relationship, B: Bundle<Effect: NoBundleEffect>> SpawnableList<R> for Vec<B> {
    fn spawn(self, world: &mut World, entity: Entity) {
        let mapped_bundles = self.into_iter().map(|b| (R::from(entity), b));
        world.spawn_batch(mapped_bundles);
    }

    fn size_hint(&self) -> usize {
        self.len()
    }
}

impl<R: Relationship, B: Bundle> SpawnableList<R> for Spawn<B> {
    fn spawn(self, world: &mut World, entity: Entity) {
        world.spawn((R::from(entity), self.0));
    }

    fn size_hint(&self) -> usize {
        1
    }
}

/// A [`SpawnableList`] that spawns entities using an iterator of a given [`Bundle`]:
///
/// ```
/// # use bevy_ecs::hierarchy::Children;
/// # use bevy_ecs::spawn::{Spawn, SpawnIter, SpawnRelated};
/// # use bevy_ecs::name::Name;
/// # use bevy_ecs::world::World;
/// let mut world = World::new();
/// world.spawn((
///     Name::new("Root"),
///     Children::spawn((
///         Spawn(Name::new("Child1")),
///         SpawnIter(["Child2", "Child3"].into_iter().map(Name::new)),
///     )),
/// ));
/// ```
pub struct SpawnIter<I>(pub I);

impl<R: Relationship, I: Iterator<Item = B> + Send + Sync + 'static, B: Bundle> SpawnableList<R>
    for SpawnIter<I>
{
    fn spawn(self, world: &mut World, entity: Entity) {
        for bundle in self.0 {
            world.spawn((R::from(entity), bundle));
        }
    }

    fn size_hint(&self) -> usize {
        self.0.size_hint().0
    }
}

/// A [`SpawnableList`] that spawns entities using a [`FnOnce`] with a [`RelatedSpawner`] as an argument:
///
/// ```
/// # use bevy_ecs::hierarchy::{Children, ChildOf};
/// # use bevy_ecs::spawn::{Spawn, SpawnWith, SpawnRelated};
/// # use bevy_ecs::name::Name;
/// # use bevy_ecs::relationship::RelatedSpawner;
/// # use bevy_ecs::world::World;
/// let mut world = World::new();
/// world.spawn((
///     Name::new("Root"),
///     Children::spawn((
///         Spawn(Name::new("Child1")),
///         SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
///             parent.spawn(Name::new("Child2"));
///             parent.spawn(Name::new("Child3"));
///         }),
///     )),
/// ));
/// ```
pub struct SpawnWith<F>(pub F);

impl<R: Relationship, F: FnOnce(&mut RelatedSpawner<R>) + Send + Sync + 'static> SpawnableList<R>
    for SpawnWith<F>
{
    fn spawn(self, world: &mut World, entity: Entity) {
        world.entity_mut(entity).with_related_entities(self.0);
    }

    fn size_hint(&self) -> usize {
        1
    }
}

macro_rules! spawnable_list_impl {
    ($($list: ident),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such, the lints below may not always apply."
        )]
        impl<R: Relationship, $($list: SpawnableList<R>),*> SpawnableList<R> for ($($list,)*) {
            fn spawn(self, _world: &mut World, _entity: Entity) {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($list,)*) = self;
                $($list.spawn(_world, _entity);)*
            }

            fn size_hint(&self) -> usize {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($list,)*) = self;
                0 $(+ $list.size_hint())*
            }
       }
    }
}

all_tuples!(spawnable_list_impl, 0, 12, P);

/// A [`Bundle`] that:
/// 1. Contains a [`RelationshipTarget`] component (associated with the given [`Relationship`]). This reserves space for the [`SpawnableList`].
/// 2. Spawns a [`SpawnableList`] of related entities with a given [`Relationship`].
///
/// This is intended to be created using [`SpawnRelated`].
pub struct SpawnRelatedBundle<R: Relationship, L: SpawnableList<R>> {
    list: L,
    marker: PhantomData<R>,
}

impl<R: Relationship, L: SpawnableList<R>> BundleEffect for SpawnRelatedBundle<R, L> {
    fn apply(self, entity: &mut EntityWorldMut) {
        let id = entity.id();
        entity.world_scope(|world: &mut World| {
            self.list.spawn(world, id);
        });
    }
}

// SAFETY: This internally relies on the RelationshipTarget's Bundle implementation, which is sound.
unsafe impl<R: Relationship, L: SpawnableList<R> + Send + Sync + 'static> Bundle
    for SpawnRelatedBundle<R, L>
{
    fn component_ids(
        components: &mut crate::component::ComponentsRegistrator,
        ids: &mut impl FnMut(crate::component::ComponentId),
    ) {
        <R::RelationshipTarget as Bundle>::component_ids(components, ids);
    }

    fn get_component_ids(
        components: &crate::component::Components,
        ids: &mut impl FnMut(Option<crate::component::ComponentId>),
    ) {
        <R::RelationshipTarget as Bundle>::get_component_ids(components, ids);
    }

    fn register_required_components(
        components: &mut crate::component::ComponentsRegistrator,
        required_components: &mut crate::component::RequiredComponents,
    ) {
        <R::RelationshipTarget as Bundle>::register_required_components(
            components,
            required_components,
        );
    }
}

impl<R: Relationship, L: SpawnableList<R>> DynamicBundle for SpawnRelatedBundle<R, L> {
    type Effect = Self;

    fn get_components(
        self,
        func: &mut impl FnMut(crate::component::StorageType, bevy_ptr::OwningPtr<'_>),
    ) -> Self::Effect {
        <R::RelationshipTarget as RelationshipTarget>::with_capacity(self.list.size_hint())
            .get_components(func);
        self
    }
}

/// A [`Bundle`] that:
/// 1. Contains a [`RelationshipTarget`] component (associated with the given [`Relationship`]). This reserves space for a single entity.
/// 2. Spawns a single related entity containing the given `B` [`Bundle`] and the given [`Relationship`].
///
/// This is intended to be created using [`SpawnRelated`].
pub struct SpawnOneRelated<R: Relationship, B: Bundle> {
    bundle: B,
    marker: PhantomData<R>,
}

impl<R: Relationship, B: Bundle> BundleEffect for SpawnOneRelated<R, B> {
    fn apply(self, entity: &mut EntityWorldMut) {
        entity.with_related::<R>(self.bundle);
    }
}

impl<R: Relationship, B: Bundle> DynamicBundle for SpawnOneRelated<R, B> {
    type Effect = Self;

    fn get_components(
        self,
        func: &mut impl FnMut(crate::component::StorageType, bevy_ptr::OwningPtr<'_>),
    ) -> Self::Effect {
        <R::RelationshipTarget as RelationshipTarget>::with_capacity(1).get_components(func);
        self
    }
}

// SAFETY: This internally relies on the RelationshipTarget's Bundle implementation, which is sound.
unsafe impl<R: Relationship, B: Bundle> Bundle for SpawnOneRelated<R, B> {
    fn component_ids(
        components: &mut crate::component::ComponentsRegistrator,
        ids: &mut impl FnMut(crate::component::ComponentId),
    ) {
        <R::RelationshipTarget as Bundle>::component_ids(components, ids);
    }

    fn get_component_ids(
        components: &crate::component::Components,
        ids: &mut impl FnMut(Option<crate::component::ComponentId>),
    ) {
        <R::RelationshipTarget as Bundle>::get_component_ids(components, ids);
    }

    fn register_required_components(
        components: &mut crate::component::ComponentsRegistrator,
        required_components: &mut crate::component::RequiredComponents,
    ) {
        <R::RelationshipTarget as Bundle>::register_required_components(
            components,
            required_components,
        );
    }
}

/// [`RelationshipTarget`] methods that create a [`Bundle`] with a [`DynamicBundle::Effect`] that:
///
/// 1. Contains the [`RelationshipTarget`] component, pre-allocated with the necessary space for spawned entities.
/// 2. Spawns an entity (or a list of entities) that relate to the entity the [`Bundle`] is added to via the [`RelationshipTarget::Relationship`].
pub trait SpawnRelated: RelationshipTarget {
    /// Returns a [`Bundle`] containing this [`RelationshipTarget`] component. It also spawns a [`SpawnableList`] of entities, each related to the bundle's entity
    /// via [`RelationshipTarget::Relationship`]. The [`RelationshipTarget`] (when possible) will pre-allocate space for the related entities.
    ///
    /// See [`Spawn`], [`SpawnIter`], and [`SpawnWith`] for usage examples.
    fn spawn<L: SpawnableList<Self::Relationship>>(
        list: L,
    ) -> SpawnRelatedBundle<Self::Relationship, L>;

    /// Returns a [`Bundle`] containing this [`RelationshipTarget`] component. It also spawns a single entity containing [`Bundle`] that is related to the bundle's entity
    /// via [`RelationshipTarget::Relationship`].
    ///
    /// ```
    /// # use bevy_ecs::hierarchy::Children;
    /// # use bevy_ecs::spawn::SpawnRelated;
    /// # use bevy_ecs::name::Name;
    /// # use bevy_ecs::world::World;
    /// let mut world = World::new();
    /// world.spawn((
    ///     Name::new("Root"),
    ///     Children::spawn_one(Name::new("Child")),
    /// ));
    /// ```
    fn spawn_one<B: Bundle>(bundle: B) -> SpawnOneRelated<Self::Relationship, B>;
}

impl<T: RelationshipTarget> SpawnRelated for T {
    fn spawn<L: SpawnableList<Self::Relationship>>(
        list: L,
    ) -> SpawnRelatedBundle<Self::Relationship, L> {
        SpawnRelatedBundle {
            list,
            marker: PhantomData,
        }
    }

    fn spawn_one<B: Bundle>(bundle: B) -> SpawnOneRelated<Self::Relationship, B> {
        SpawnOneRelated {
            bundle,
            marker: PhantomData,
        }
    }
}

/// Returns a [`SpawnRelatedBundle`] that will insert the given [`RelationshipTarget`], spawn a [`SpawnableList`] of entities with given bundles that
/// relate to the [`RelationshipTarget`] entity via the [`RelationshipTarget::Relationship`] component, and reserve space in the [`RelationshipTarget`] for each spawned entity.
///
/// The first argument is the [`RelationshipTarget`] type. Any additional arguments will be interpreted as bundles to be spawned.
///
/// Also see [`children`](crate::children) for a [`Children`](crate::hierarchy::Children)-specific equivalent.
///
/// ```
/// # use bevy_ecs::hierarchy::Children;
/// # use bevy_ecs::name::Name;
/// # use bevy_ecs::world::World;
/// # use bevy_ecs::related;
/// # use bevy_ecs::spawn::{Spawn, SpawnRelated};
/// let mut world = World::new();
/// world.spawn((
///     Name::new("Root"),
///     related!(Children[
///         Name::new("Child1"),
///         (
///             Name::new("Child2"),
///             related!(Children[
///                 Name::new("Grandchild"),
///             ])
///         )
///     ])
/// ));
/// ```
#[macro_export]
macro_rules! related {
    ($relationship_target:ty [$($child:expr),*$(,)?]) => {
       <$relationship_target>::spawn($crate::recursive_spawn!($($child),*))
    };
}

// A tail-recursive spawn utility.
//
// Since `SpawnableList` is only implemented for tuples
// up to twelve elements long, this macro will nest
// longer sequences recursively. By default, this recursion
// will top out at around 1400 elements, but it would be
// ill-advised to spawn that many entities with this method.
//
// For spawning large batches of entities at a time,
// consider `SpawnIter` or eagerly spawning with `Commands`.
#[macro_export]
#[doc(hidden)]
macro_rules! recursive_spawn {
    // direct expansion
    ($a:expr) => {
        $crate::spawn::Spawn($a)
    };
    ($a:expr, $b:expr) => {
        (
            $crate::spawn::Spawn($a),
            $crate::spawn::Spawn($b),
        )
    };
    ($a:expr, $b:expr, $c:expr) => {
        (
            $crate::spawn::Spawn($a),
            $crate::spawn::Spawn($b),
            $crate::spawn::Spawn($c),
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr) => {
        (
            $crate::spawn::Spawn($a),
            $crate::spawn::Spawn($b),
            $crate::spawn::Spawn($c),
            $crate::spawn::Spawn($d),
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr) => {
        (
            $crate::spawn::Spawn($a),
            $crate::spawn::Spawn($b),
            $crate::spawn::Spawn($c),
            $crate::spawn::Spawn($d),
            $crate::spawn::Spawn($e),
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr) => {
        (
            $crate::spawn::Spawn($a),
            $crate::spawn::Spawn($b),
            $crate::spawn::Spawn($c),
            $crate::spawn::Spawn($d),
            $crate::spawn::Spawn($e),
            $crate::spawn::Spawn($f),
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr) => {
        (
            $crate::spawn::Spawn($a),
            $crate::spawn::Spawn($b),
            $crate::spawn::Spawn($c),
            $crate::spawn::Spawn($d),
            $crate::spawn::Spawn($e),
            $crate::spawn::Spawn($f),
            $crate::spawn::Spawn($g),
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr) => {
        (
            $crate::spawn::Spawn($a),
            $crate::spawn::Spawn($b),
            $crate::spawn::Spawn($c),
            $crate::spawn::Spawn($d),
            $crate::spawn::Spawn($e),
            $crate::spawn::Spawn($f),
            $crate::spawn::Spawn($g),
            $crate::spawn::Spawn($h),
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr, $i:expr) => {
        (
            $crate::spawn::Spawn($a),
            $crate::spawn::Spawn($b),
            $crate::spawn::Spawn($c),
            $crate::spawn::Spawn($d),
            $crate::spawn::Spawn($e),
            $crate::spawn::Spawn($f),
            $crate::spawn::Spawn($g),
            $crate::spawn::Spawn($h),
            $crate::spawn::Spawn($i),
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr, $i:expr, $j:expr) => {
        (
            $crate::spawn::Spawn($a),
            $crate::spawn::Spawn($b),
            $crate::spawn::Spawn($c),
            $crate::spawn::Spawn($d),
            $crate::spawn::Spawn($e),
            $crate::spawn::Spawn($f),
            $crate::spawn::Spawn($g),
            $crate::spawn::Spawn($h),
            $crate::spawn::Spawn($i),
            $crate::spawn::Spawn($j),
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr, $i:expr, $j:expr, $k:expr) => {
        (
            $crate::spawn::Spawn($a),
            $crate::spawn::Spawn($b),
            $crate::spawn::Spawn($c),
            $crate::spawn::Spawn($d),
            $crate::spawn::Spawn($e),
            $crate::spawn::Spawn($f),
            $crate::spawn::Spawn($g),
            $crate::spawn::Spawn($h),
            $crate::spawn::Spawn($i),
            $crate::spawn::Spawn($j),
            $crate::spawn::Spawn($k),
        )
    };

    // recursive expansion
    (
        $a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr,
        $g:expr, $h:expr, $i:expr, $j:expr, $k:expr, $($rest:expr),*
    ) => {
        (
            $crate::spawn::Spawn($a),
            $crate::spawn::Spawn($b),
            $crate::spawn::Spawn($c),
            $crate::spawn::Spawn($d),
            $crate::spawn::Spawn($e),
            $crate::spawn::Spawn($f),
            $crate::spawn::Spawn($g),
            $crate::spawn::Spawn($h),
            $crate::spawn::Spawn($i),
            $crate::spawn::Spawn($j),
            $crate::spawn::Spawn($k),
            $crate::recursive_spawn!($($rest),*)
        )
    };
}
