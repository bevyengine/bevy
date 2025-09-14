//! Entity spawning abstractions, largely focused on spawning related hierarchies of entities. See [`related`](crate::related) and [`SpawnRelated`]
//! for the best entry points into these APIs and examples of how to use them.

use crate::{
    bundle::{Bundle, DynamicBundle, InsertMode, NoBundleEffect},
    change_detection::MaybeLocation,
    entity::Entity,
    query::DebugCheckedUnwrap,
    relationship::{RelatedSpawner, Relationship, RelationshipHookMode, RelationshipTarget},
    world::{EntityWorldMut, World},
};
use alloc::vec::Vec;
use bevy_ptr::{move_as_ptr, MovingPtr};
use core::{
    marker::PhantomData,
    mem::{self, MaybeUninit},
};
use variadics_please::all_tuples_enumerated;

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
pub trait SpawnableList<R>: Sized {
    /// Spawn this list of changes in a given [`World`] and relative to a given [`Entity`]. This is generally used
    /// for spawning "related" entities, such as children.
    // This function explicitly uses `MovingPtr` to avoid potentially large stack copies of the bundle
    // when inserting into ECS storage. See https://github.com/bevyengine/bevy/issues/20571 for more
    // information.
    fn spawn(this: MovingPtr<'_, Self>, world: &mut World, entity: Entity);

    /// Returns a size hint, which is used to reserve space for this list in a [`RelationshipTarget`]. This should be
    /// less than or equal to the actual size of the list. When in doubt, just use 0.
    fn size_hint(&self) -> usize;
}

impl<R: Relationship, B: Bundle<Effect: NoBundleEffect>> SpawnableList<R> for Vec<B> {
    fn spawn(ptr: MovingPtr<'_, Self>, world: &mut World, entity: Entity) {
        let mapped_bundles = ptr.read().into_iter().map(|b| (R::from(entity), b));
        world.spawn_batch(mapped_bundles);
    }

    fn size_hint(&self) -> usize {
        self.len()
    }
}

impl<R: Relationship, B: Bundle> SpawnableList<R> for Spawn<B> {
    fn spawn(this: MovingPtr<'_, Self>, world: &mut World, entity: Entity) {
        #[track_caller]
        fn spawn<B: Bundle, R: Relationship>(
            this: MovingPtr<'_, Spawn<B>>,
            world: &mut World,
            entity: Entity,
        ) {
            let caller = MaybeLocation::caller();

            // SAFETY:
            //  - `Spawn<B>` has one field at index 0.
            //  - if `this` is aligned, then its inner bundle must be as well.
            let bundle = unsafe {
                bevy_ptr::deconstruct_moving_ptr!(this => (
                    0 => bundle,
                ));
                bundle.try_into().debug_checked_unwrap()
            };

            let r = R::from(entity);
            move_as_ptr!(r);
            let mut entity = world.spawn_with_caller(r, caller);

            entity.insert_with_caller(
                bundle,
                InsertMode::Replace,
                caller,
                RelationshipHookMode::Run,
            );
        }

        spawn::<B, R>(this, world, entity);
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
    fn spawn(mut this: MovingPtr<'_, Self>, world: &mut World, entity: Entity) {
        for bundle in &mut this.0 {
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
    fn spawn(this: MovingPtr<'_, Self>, world: &mut World, entity: Entity) {
        world
            .entity_mut(entity)
            .with_related_entities(this.read().0);
    }

    fn size_hint(&self) -> usize {
        1
    }
}

/// A [`SpawnableList`] that links already spawned entities to the root entity via relations of type `I`.
///
/// This is useful if the entity has already been spawned earlier or if you spawn multiple relationships link to the same entity at the same time.
/// If you only need to do this for a single entity, consider using [`WithOneRelated`].
///
/// ```
/// # use bevy_ecs::hierarchy::Children;
/// # use bevy_ecs::spawn::{Spawn, WithRelated, SpawnRelated};
/// # use bevy_ecs::name::Name;
/// # use bevy_ecs::world::World;
/// let mut world = World::new();
///
/// let child2 = world.spawn(Name::new("Child2")).id();
/// let child3 = world.spawn(Name::new("Child3")).id();
///
/// world.spawn((
///     Name::new("Root"),
///     Children::spawn((
///         Spawn(Name::new("Child1")),
///         // This adds the already existing entities as children of Root.
///         WithRelated::new([child2, child3]),
///     )),
/// ));
/// ```
pub struct WithRelated<I>(pub I);

impl<I> WithRelated<I> {
    /// Creates a new [`WithRelated`] from a collection of entities.
    pub fn new(iter: impl IntoIterator<IntoIter = I>) -> Self {
        Self(iter.into_iter())
    }
}

impl<R: Relationship, I: Iterator<Item = Entity>> SpawnableList<R> for WithRelated<I> {
    fn spawn(mut this: MovingPtr<'_, Self>, world: &mut World, entity: Entity) {
        let related = (&mut this.0).collect::<Vec<_>>();
        world.entity_mut(entity).add_related::<R>(&related);
    }

    fn size_hint(&self) -> usize {
        self.0.size_hint().0
    }
}

/// A wrapper over an [`Entity`] indicating that an entity should be added.
/// This is intended to be used for hierarchical spawning via traits like [`SpawnableList`] and [`SpawnRelated`].
///
/// Unlike [`WithRelated`] this only adds one entity.
///
/// Also see the [`children`](crate::children) and [`related`](crate::related) macros that abstract over the [`Spawn`] API.
///
/// ```
/// # use bevy_ecs::hierarchy::Children;
/// # use bevy_ecs::spawn::{Spawn, WithOneRelated, SpawnRelated};
/// # use bevy_ecs::name::Name;
/// # use bevy_ecs::world::World;
/// let mut world = World::new();
///
/// let child1 = world.spawn(Name::new("Child1")).id();
///
/// world.spawn((
///     Name::new("Root"),
///     Children::spawn((
///         // This adds the already existing entity as a child of Root.
///         WithOneRelated(child1),
///     )),
/// ));
/// ```
pub struct WithOneRelated(pub Entity);

impl<R: Relationship> SpawnableList<R> for WithOneRelated {
    fn spawn(this: MovingPtr<'_, Self>, world: &mut World, entity: Entity) {
        world.entity_mut(entity).add_one_related::<R>(this.read().0);
    }

    fn size_hint(&self) -> usize {
        1
    }
}

macro_rules! spawnable_list_impl {
    ($(#[$meta:meta])* $(($index:tt, $list: ident, $alias: ident)),*) => {
        $(#[$meta])*
        impl<R: Relationship, $($list: SpawnableList<R>),*> SpawnableList<R> for ($($list,)*) {
            #[expect(
                clippy::allow_attributes,
                reason = "This is a tuple-related macro; as such, the lints below may not always apply."
            )]
            #[allow(unused_unsafe, reason = "The empty tuple will leave the unsafe blocks unused.")]
            fn spawn(_this: MovingPtr<'_, Self>, _world: &mut World, _entity: Entity)
            where
                Self: Sized,
            {
                // SAFETY:
                //  - The indices uniquely match the type definition and thus must point to the right fields.
                //  - Rust tuples can never be `repr(packed)` so if `_this` is properly aligned, then all of the individual field
                //    pointers must also be properly aligned.
                unsafe {
                    bevy_ptr::deconstruct_moving_ptr!(_this => ($($index => $alias,)*));
                    $( SpawnableList::<R>::spawn($alias.try_into().debug_checked_unwrap(), _world, _entity); )*
                }
            }

            fn size_hint(&self) -> usize {
                let ($($alias,)*) = self;
                0 $(+ $alias.size_hint())*
            }
       }
    }
}

all_tuples_enumerated!(
    #[doc(fake_variadic)]
    spawnable_list_impl,
    0,
    12,
    P,
    field_
);

/// A [`Bundle`] that:
/// 1. Contains a [`RelationshipTarget`] component (associated with the given [`Relationship`]). This reserves space for the [`SpawnableList`].
/// 2. Spawns a [`SpawnableList`] of related entities with a given [`Relationship`].
///
/// This is intended to be created using [`SpawnRelated`].
pub struct SpawnRelatedBundle<R: Relationship, L: SpawnableList<R>> {
    list: L,
    marker: PhantomData<R>,
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
}

impl<R: Relationship, L: SpawnableList<R>> DynamicBundle for SpawnRelatedBundle<R, L> {
    type Effect = Self;

    unsafe fn get_components(
        ptr: MovingPtr<'_, Self>,
        func: &mut impl FnMut(crate::component::StorageType, bevy_ptr::OwningPtr<'_>),
    ) {
        let target =
            <R::RelationshipTarget as RelationshipTarget>::with_capacity(ptr.list.size_hint());
        move_as_ptr!(target);
        // SAFETY:
        // - The caller must ensure that this is called exactly once before `apply_effect`.
        // - Assuming `DynamicBundle` is implemented correctly for `R::Relationship` target, `func` should be
        //   called exactly once for each component being fetched with the correct `StorageType`
        // - `Effect: !NoBundleEffect`, which means the caller is responsible for calling this type's `apply_effect`
        //   at least once before returning to safe code.
        <R::RelationshipTarget as DynamicBundle>::get_components(target, func);
        // Forget the pointer so that the value is available in `apply_effect`.
        mem::forget(ptr);
    }

    unsafe fn apply_effect(ptr: MovingPtr<'_, MaybeUninit<Self>>, entity: &mut EntityWorldMut) {
        // SAFETY: The value was not moved out in `get_components`, only borrowed, and thus should still
        // be valid and initialized.
        let effect = unsafe { ptr.assume_init() };
        let id = entity.id();

        // SAFETY:
        //  - `ptr` points to an instance of type `Self`
        //  - The field names and types match with the type definition.
        entity.world_scope(|world: &mut World| unsafe {
            bevy_ptr::deconstruct_moving_ptr!(effect => { list, });
            L::spawn(list.try_into().debug_checked_unwrap(), world, id);
        });
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

impl<R: Relationship, B: Bundle> DynamicBundle for SpawnOneRelated<R, B> {
    type Effect = Self;

    unsafe fn get_components(
        ptr: MovingPtr<'_, Self>,
        func: &mut impl FnMut(crate::component::StorageType, bevy_ptr::OwningPtr<'_>),
    ) {
        let target = <R::RelationshipTarget as RelationshipTarget>::with_capacity(1);
        move_as_ptr!(target);
        // SAFETY:
        // - The caller must ensure that this is called exactly once before `apply_effect`.
        // - Assuming `DynamicBundle` is implemented correctly for `R::Relationship` target, `func` should be
        //   called exactly once for each component being fetched with the correct `StorageType`
        // - `Effect: !NoBundleEffect`, which means the caller is responsible for calling this type's `apply_effect`
        //   at least once before returning to safe code.
        <R::RelationshipTarget as DynamicBundle>::get_components(target, func);
        // Forget the pointer so that the value is available in `apply_effect`.
        mem::forget(ptr);
    }

    unsafe fn apply_effect(ptr: MovingPtr<'_, MaybeUninit<Self>>, entity: &mut EntityWorldMut) {
        // SAFETY: The value was not moved out in `get_components`, only borrowed, and thus should still
        // be valid and initialized.
        let effect = unsafe { ptr.assume_init() };
        let effect = effect.read();
        entity.with_related::<R>(effect.bundle);
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
}

/// [`RelationshipTarget`] methods that create a [`Bundle`] with a [`DynamicBundle::Effect`] that:
///
/// 1. Contains the [`RelationshipTarget`] component, pre-allocated with the necessary space for spawned entities.
/// 2. Spawns an entity (or a list of entities) that relate to the entity the [`Bundle`] is added to via the [`RelationshipTarget::Relationship`].
pub trait SpawnRelated: RelationshipTarget {
    /// Returns a [`Bundle`] containing this [`RelationshipTarget`] component. It also spawns a [`SpawnableList`] of entities, each related to the bundle's entity
    /// via [`RelationshipTarget::Relationship`]. The [`RelationshipTarget`] (when possible) will pre-allocate space for the related entities.
    ///
    /// See [`Spawn`], [`SpawnIter`], [`SpawnWith`], [`WithRelated`] and [`WithOneRelated`] for usage examples.
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

#[cfg(test)]
mod tests {

    use crate::{
        name::Name,
        prelude::{ChildOf, Children, RelationshipTarget},
        relationship::RelatedSpawner,
        world::World,
    };

    use super::{Spawn, SpawnIter, SpawnRelated, SpawnWith, WithOneRelated, WithRelated};

    #[test]
    fn spawn() {
        let mut world = World::new();

        let parent = world
            .spawn((
                Name::new("Parent"),
                Children::spawn(Spawn(Name::new("Child1"))),
            ))
            .id();

        let children = world
            .query::<&Children>()
            .get(&world, parent)
            .expect("An entity with Children should exist");

        assert_eq!(children.iter().count(), 1);

        for ChildOf(child) in world.query::<&ChildOf>().iter(&world) {
            assert_eq!(child, &parent);
        }
    }

    #[test]
    fn spawn_iter() {
        let mut world = World::new();

        let parent = world
            .spawn((
                Name::new("Parent"),
                Children::spawn(SpawnIter(["Child1", "Child2"].into_iter().map(Name::new))),
            ))
            .id();

        let children = world
            .query::<&Children>()
            .get(&world, parent)
            .expect("An entity with Children should exist");

        assert_eq!(children.iter().count(), 2);

        for ChildOf(child) in world.query::<&ChildOf>().iter(&world) {
            assert_eq!(child, &parent);
        }
    }

    #[test]
    fn spawn_with() {
        let mut world = World::new();

        let parent = world
            .spawn((
                Name::new("Parent"),
                Children::spawn(SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
                    parent.spawn(Name::new("Child1"));
                })),
            ))
            .id();

        let children = world
            .query::<&Children>()
            .get(&world, parent)
            .expect("An entity with Children should exist");

        assert_eq!(children.iter().count(), 1);

        for ChildOf(child) in world.query::<&ChildOf>().iter(&world) {
            assert_eq!(child, &parent);
        }
    }

    #[test]
    fn with_related() {
        let mut world = World::new();

        let child1 = world.spawn(Name::new("Child1")).id();
        let child2 = world.spawn(Name::new("Child2")).id();

        let parent = world
            .spawn((
                Name::new("Parent"),
                Children::spawn(WithRelated::new([child1, child2])),
            ))
            .id();

        let children = world
            .query::<&Children>()
            .get(&world, parent)
            .expect("An entity with Children should exist");

        assert_eq!(children.iter().count(), 2);

        assert_eq!(
            world.entity(child1).get::<ChildOf>(),
            Some(&ChildOf(parent))
        );
        assert_eq!(
            world.entity(child2).get::<ChildOf>(),
            Some(&ChildOf(parent))
        );
    }

    #[test]
    fn with_one_related() {
        let mut world = World::new();

        let child1 = world.spawn(Name::new("Child1")).id();

        let parent = world
            .spawn((Name::new("Parent"), Children::spawn(WithOneRelated(child1))))
            .id();

        let children = world
            .query::<&Children>()
            .get(&world, parent)
            .expect("An entity with Children should exist");

        assert_eq!(children.iter().count(), 1);

        assert_eq!(
            world.entity(child1).get::<ChildOf>(),
            Some(&ChildOf(parent))
        );
    }
}
