//! The canonical "parent-child" [`Relationship`] for entities, driven by
//! the [`ChildOf`] [`Relationship`] and the [`Children`] [`RelationshipTarget`].
//!
//! See [`ChildOf`] for a full description of the relationship and how to use it.
//!
//! [`Relationship`]: crate::relationship::Relationship
//! [`RelationshipTarget`]: crate::relationship::RelationshipTarget

#[cfg(feature = "bevy_reflect")]
use crate::reflect::{ReflectComponent, ReflectFromWorld};
use crate::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    lifecycle::HookContext,
    relationship::{RelatedSpawner, RelatedSpawnerCommands},
    system::EntityCommands,
    world::{DeferredWorld, EntityWorldMut, FromWorld, World},
};
use alloc::{format, string::String, vec::Vec};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::std_traits::ReflectDefault;
#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
use bevy_utils::prelude::DebugName;
use core::ops::Deref;
use core::slice;
use log::warn;

/// Stores the parent entity of this child entity with this component.
///
/// This is a [`Relationship`] component, and creates the canonical
/// "parent / child" hierarchy. This is the "source of truth" component, and it pairs with
/// the [`Children`] [`RelationshipTarget`](crate::relationship::RelationshipTarget).
///
/// This relationship should be used for things like:
///
/// 1. Organizing entities in a scene
/// 2. Propagating configuration or data inherited from a parent, such as "visibility" or "world-space global transforms".
/// 3. Ensuring a hierarchy is despawned when an entity is despawned.
///
/// [`ChildOf`] contains a single "target" [`Entity`]. When [`ChildOf`] is inserted on a "source" entity,
/// the "target" entity will automatically (and immediately, via a component hook) have a [`Children`]
/// component inserted, and the "source" entity will be added to that [`Children`] instance.
///
/// If the [`ChildOf`] component is replaced with a different "target" entity, the old target's [`Children`]
/// will be automatically (and immediately, via a component hook) be updated to reflect that change.
///
/// Likewise, when the [`ChildOf`] component is removed, the "source" entity will be removed from the old
/// target's [`Children`]. If this results in [`Children`] being empty, [`Children`] will be automatically removed.
///
/// When a parent is despawned, all children (and their descendants) will _also_ be despawned.
///
/// You can create parent-child relationships in a variety of ways. The most direct way is to insert a [`ChildOf`] component:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::new();
/// let root = world.spawn_empty().id();
/// let child1 = world.spawn(ChildOf(root)).id();
/// let child2 = world.spawn(ChildOf(root)).id();
/// let grandchild = world.spawn(ChildOf(child1)).id();
///
/// assert_eq!(&**world.entity(root).get::<Children>().unwrap(), &[child1, child2]);
/// assert_eq!(&**world.entity(child1).get::<Children>().unwrap(), &[grandchild]);
///
/// world.entity_mut(child2).remove::<ChildOf>();
/// assert_eq!(&**world.entity(root).get::<Children>().unwrap(), &[child1]);
///
/// world.entity_mut(root).despawn();
/// assert!(world.get_entity(root).is_err());
/// assert!(world.get_entity(child1).is_err());
/// assert!(world.get_entity(grandchild).is_err());
/// ```
///
/// However if you are spawning many children, you might want to use the [`EntityWorldMut::with_children`] helper instead:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::new();
/// let mut child1 = Entity::PLACEHOLDER;
/// let mut child2 = Entity::PLACEHOLDER;
/// let mut grandchild = Entity::PLACEHOLDER;
/// let root = world.spawn_empty().with_children(|p| {
///     child1 = p.spawn_empty().with_children(|p| {
///         grandchild = p.spawn_empty().id();
///     }).id();
///     child2 = p.spawn_empty().id();
/// }).id();
///
/// assert_eq!(&**world.entity(root).get::<Children>().unwrap(), &[child1, child2]);
/// assert_eq!(&**world.entity(child1).get::<Children>().unwrap(), &[grandchild]);
/// ```
///
/// [`Relationship`]: crate::relationship::Relationship
#[derive(Component, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(
    feature = "bevy_reflect",
    reflect(Component, PartialEq, Debug, FromWorld, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
#[relationship(relationship_target = Children)]
#[doc(alias = "IsChild", alias = "Parent")]
pub struct ChildOf(#[entities] pub Entity);

impl ChildOf {
    /// The parent entity of this child entity.
    #[inline]
    pub fn parent(&self) -> Entity {
        self.0
    }
}

// TODO: We need to impl either FromWorld or Default so ChildOf can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However ChildOf should only ever be set with a real user-defined entity.  Its worth looking into
// better ways to handle cases like this.
impl FromWorld for ChildOf {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        ChildOf(Entity::PLACEHOLDER)
    }
}

/// Tracks which entities are children of this parent entity.
///
/// A [`RelationshipTarget`] collection component that is populated
/// with entities that "target" this entity with the [`ChildOf`] [`Relationship`] component.
///
/// Together, these components form the "canonical parent-child hierarchy". See the [`ChildOf`] component for the full
/// description of this relationship and instructions on how to use it.
///
/// # Usage
///
/// Like all [`RelationshipTarget`] components, this data should not be directly manipulated to avoid desynchronization.
/// Instead, modify the [`ChildOf`] components on the "source" entities.
///
/// To access the children of an entity, you can iterate over the [`Children`] component,
/// using the [`IntoIterator`] trait.
/// For more complex access patterns, see the [`RelationshipTarget`] trait.
///
/// [`Relationship`]: crate::relationship::Relationship
/// [`RelationshipTarget`]: crate::relationship::RelationshipTarget
#[derive(Component, Default, Debug, PartialEq, Eq)]
#[relationship_target(relationship = ChildOf, linked_spawn)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Component, FromWorld, Default))]
#[doc(alias = "IsParent")]
pub struct Children(Vec<Entity>);

impl Children {
    /// Swaps the child at `a_index` with the child at `b_index`.
    #[inline]
    pub fn swap(&mut self, a_index: usize, b_index: usize) {
        self.0.swap(a_index, b_index);
    }

    /// Sorts children [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided comparator function.
    ///
    /// For the underlying implementation, see [`slice::sort_by`].
    ///
    /// For the unstable version, see [`sort_unstable_by`](Children::sort_unstable_by).
    ///
    /// See also [`sort_by_key`](Children::sort_by_key), [`sort_by_cached_key`](Children::sort_by_cached_key).
    #[inline]
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Entity, &Entity) -> core::cmp::Ordering,
    {
        self.0.sort_by(compare);
    }

    /// Sorts children [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function.
    ///
    /// For the underlying implementation, see [`slice::sort_by_key`].
    ///
    /// For the unstable version, see [`sort_unstable_by_key`](Children::sort_unstable_by_key).
    ///
    /// See also [`sort_by`](Children::sort_by), [`sort_by_cached_key`](Children::sort_by_cached_key).
    #[inline]
    pub fn sort_by_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.0.sort_by_key(compare);
    }

    /// Sorts children [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function. Only evaluates each key at most
    /// once per sort, caching the intermediate results in memory.
    ///
    /// For the underlying implementation, see [`slice::sort_by_cached_key`].
    ///
    /// See also [`sort_by`](Children::sort_by), [`sort_by_key`](Children::sort_by_key).
    #[inline]
    pub fn sort_by_cached_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.0.sort_by_cached_key(compare);
    }

    /// Sorts children [unstably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided comparator function.
    ///
    /// For the underlying implementation, see [`slice::sort_unstable_by`].
    ///
    /// For the stable version, see [`sort_by`](Children::sort_by).
    ///
    /// See also [`sort_unstable_by_key`](Children::sort_unstable_by_key).
    #[inline]
    pub fn sort_unstable_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Entity, &Entity) -> core::cmp::Ordering,
    {
        self.0.sort_unstable_by(compare);
    }

    /// Sorts children [unstably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function.
    ///
    /// For the underlying implementation, see [`slice::sort_unstable_by_key`].
    ///
    /// For the stable version, see [`sort_by_key`](Children::sort_by_key).
    ///
    /// See also [`sort_unstable_by`](Children::sort_unstable_by).
    #[inline]
    pub fn sort_unstable_by_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.0.sort_unstable_by_key(compare);
    }
}

impl<'a> IntoIterator for &'a Children {
    type Item = <Self::IntoIter as Iterator>::Item;

    type IntoIter = slice::Iter<'a, Entity>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Deref for Children {
    type Target = [Entity];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A type alias over [`RelatedSpawner`] used to spawn child entities containing a [`ChildOf`] relationship.
pub type ChildSpawner<'w> = RelatedSpawner<'w, ChildOf>;

/// A type alias over [`RelatedSpawnerCommands`] used to spawn child entities containing a [`ChildOf`] relationship.
pub type ChildSpawnerCommands<'w> = RelatedSpawnerCommands<'w, ChildOf>;

impl<'w> EntityWorldMut<'w> {
    /// Spawns children of this entity (with a [`ChildOf`] relationship) by taking a function that operates on a [`ChildSpawner`].
    /// See also [`with_related`](Self::with_related).
    pub fn with_children(&mut self, func: impl FnOnce(&mut ChildSpawner)) -> &mut Self {
        self.with_related_entities(func);
        self
    }

    /// Adds the given children to this entity
    /// See also [`add_related`](Self::add_related).
    pub fn add_children(&mut self, children: &[Entity]) -> &mut Self {
        self.add_related::<ChildOf>(children)
    }

    /// Removes all the children from this entity.
    /// See also [`clear_related`](Self::clear_related)
    pub fn clear_children(&mut self) -> &mut Self {
        self.clear_related::<ChildOf>()
    }

    /// Insert children at specific index.
    /// See also [`insert_related`](Self::insert_related).
    pub fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self {
        self.insert_related::<ChildOf>(index, children)
    }

    /// Insert child at specific index.
    /// See also [`insert_related`](Self::insert_related).
    pub fn insert_child(&mut self, index: usize, child: Entity) -> &mut Self {
        self.insert_related::<ChildOf>(index, &[child])
    }

    /// Adds the given child to this entity
    /// See also [`add_related`](Self::add_related).
    pub fn add_child(&mut self, child: Entity) -> &mut Self {
        self.add_related::<ChildOf>(&[child])
    }

    /// Removes the relationship between this entity and the given entities.
    pub fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        self.remove_related::<ChildOf>(children)
    }

    /// Removes the relationship between this entity and the given entity.
    pub fn remove_child(&mut self, child: Entity) -> &mut Self {
        self.remove_related::<ChildOf>(&[child])
    }

    /// Replaces all the related children with a new set of children.
    pub fn replace_children(&mut self, children: &[Entity]) -> &mut Self {
        self.replace_related::<ChildOf>(children)
    }

    /// Replaces all the related children with a new set of children.
    ///
    /// # Warning
    ///
    /// Failing to maintain the functions invariants may lead to erratic engine behavior including random crashes.
    /// Refer to [`Self::replace_related_with_difference`] for a list of these invariants.
    ///
    /// # Panics
    ///
    /// Panics when debug assertions are enabled if an invariant is broken and the command is executed.
    pub fn replace_children_with_difference(
        &mut self,
        entities_to_unrelate: &[Entity],
        entities_to_relate: &[Entity],
        newly_related_entities: &[Entity],
    ) -> &mut Self {
        self.replace_related_with_difference::<ChildOf>(
            entities_to_unrelate,
            entities_to_relate,
            newly_related_entities,
        )
    }

    /// Spawns the passed bundle and adds it to this entity as a child.
    ///
    /// For efficient spawning of multiple children, use [`with_children`].
    ///
    /// [`with_children`]: EntityWorldMut::with_children
    pub fn with_child(&mut self, bundle: impl Bundle) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            world.spawn((bundle, ChildOf(parent)));
        });
        self
    }
}

impl<'a> EntityCommands<'a> {
    /// Spawns children of this entity (with a [`ChildOf`] relationship) by taking a function that operates on a [`ChildSpawner`].
    pub fn with_children(
        &mut self,
        func: impl FnOnce(&mut RelatedSpawnerCommands<ChildOf>),
    ) -> &mut Self {
        self.with_related_entities(func);
        self
    }

    /// Adds the given children to this entity
    pub fn add_children(&mut self, children: &[Entity]) -> &mut Self {
        self.add_related::<ChildOf>(children)
    }

    /// Removes all the children from this entity.
    /// See also [`clear_related`](Self::clear_related)
    pub fn clear_children(&mut self) -> &mut Self {
        self.clear_related::<ChildOf>()
    }

    /// Insert children at specific index.
    /// See also [`insert_related`](Self::insert_related).
    pub fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self {
        self.insert_related::<ChildOf>(index, children)
    }

    /// Insert children at specific index.
    /// See also [`insert_related`](Self::insert_related).
    pub fn insert_child(&mut self, index: usize, child: Entity) -> &mut Self {
        self.insert_related::<ChildOf>(index, &[child])
    }

    /// Adds the given child to this entity
    pub fn add_child(&mut self, child: Entity) -> &mut Self {
        self.add_related::<ChildOf>(&[child])
    }

    /// Removes the relationship between this entity and the given entities.
    pub fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        self.remove_related::<ChildOf>(children)
    }

    /// Removes the relationship between this entity and the given entity.
    pub fn remove_child(&mut self, child: Entity) -> &mut Self {
        self.remove_related::<ChildOf>(&[child])
    }

    /// Replaces the children on this entity with a new list of children.
    pub fn replace_children(&mut self, children: &[Entity]) -> &mut Self {
        self.replace_related::<ChildOf>(children)
    }

    /// Replaces all the related entities with a new set of entities.
    ///
    /// # Warning
    ///
    /// Failing to maintain the functions invariants may lead to erratic engine behavior including random crashes.
    /// Refer to [`EntityWorldMut::replace_related_with_difference`] for a list of these invariants.
    ///
    /// # Panics
    ///
    /// Panics when debug assertions are enabled if an invariant is broken and the command is executed.
    pub fn replace_children_with_difference(
        &mut self,
        entities_to_unrelate: &[Entity],
        entities_to_relate: &[Entity],
        newly_related_entities: &[Entity],
    ) -> &mut Self {
        self.replace_related_with_difference::<ChildOf>(
            entities_to_unrelate,
            entities_to_relate,
            newly_related_entities,
        )
    }

    /// Spawns the passed bundle and adds it to this entity as a child.
    ///
    /// For efficient spawning of multiple children, use [`with_children`].
    ///
    /// [`with_children`]: EntityCommands::with_children
    pub fn with_child(&mut self, bundle: impl Bundle) -> &mut Self {
        self.with_related::<ChildOf>(bundle);
        self
    }
}

/// An `on_insert` component hook that when run, will validate that the parent of a given entity
/// contains component `C`. This will print a warning if the parent does not contain `C`.
pub fn validate_parent_has_component<C: Component>(
    world: DeferredWorld,
    HookContext { entity, caller, .. }: HookContext,
) {
    let entity_ref = world.entity(entity);
    let Some(child_of) = entity_ref.get::<ChildOf>() else {
        return;
    };
    let parent = child_of.parent();
    if !world.get_entity(parent).is_ok_and(|e| e.contains::<C>()) {
        // TODO: print name here once Name lives in bevy_ecs
        let name: Option<String> = None;
        let debug_name = DebugName::type_name::<C>();
        warn!(
            "warning[B0004]: {}{name} with the {ty_name} component has a parent ({parent}) without {ty_name}.\n\
            This will cause inconsistent behaviors! See: https://bevy.org/learn/errors/b0004",
            caller.map(|c| format!("{c}: ")).unwrap_or_default(),
            ty_name = debug_name.shortname(),
            name = name.map_or_else(
                || format!("Entity {entity}"),
                |s| format!("The {s} entity")
            ),
        );
    }
}

/// Returns a [`SpawnRelatedBundle`] that will insert the [`Children`] component, spawn a [`SpawnableList`] of entities with given bundles that
/// relate to the [`Children`] entity via the [`ChildOf`] component, and reserve space in the [`Children`] for each spawned entity.
///
/// Any additional arguments will be interpreted as bundles to be spawned.
///
/// Also see [`related`](crate::related) for a version of this that works with any [`RelationshipTarget`] type.
///
/// ```
/// # use bevy_ecs::hierarchy::Children;
/// # use bevy_ecs::name::Name;
/// # use bevy_ecs::world::World;
/// # use bevy_ecs::children;
/// # use bevy_ecs::spawn::{Spawn, SpawnRelated};
/// let mut world = World::new();
/// world.spawn((
///     Name::new("Root"),
///     children![
///         Name::new("Child1"),
///         (
///             Name::new("Child2"),
///             children![Name::new("Grandchild")]
///         )
///     ]
/// ));
/// ```
///
/// [`RelationshipTarget`]: crate::relationship::RelationshipTarget
/// [`SpawnRelatedBundle`]: crate::spawn::SpawnRelatedBundle
/// [`SpawnableList`]: crate::spawn::SpawnableList
#[macro_export]
macro_rules! children {
    [$($child:expr),*$(,)?] => {
       $crate::hierarchy::Children::spawn($crate::recursive_spawn!($($child),*))
    };
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::Entity,
        hierarchy::{ChildOf, Children},
        relationship::{RelationshipHookMode, RelationshipTarget},
        spawn::{Spawn, SpawnRelated},
        world::World,
    };
    use alloc::{vec, vec::Vec};

    #[derive(PartialEq, Eq, Debug)]
    struct Node {
        entity: Entity,
        children: Vec<Node>,
    }

    impl Node {
        fn new(entity: Entity) -> Self {
            Self {
                entity,
                children: Vec::new(),
            }
        }

        fn new_with(entity: Entity, children: Vec<Node>) -> Self {
            Self { entity, children }
        }
    }

    fn get_hierarchy(world: &World, entity: Entity) -> Node {
        Node {
            entity,
            children: world
                .entity(entity)
                .get::<Children>()
                .map_or_else(Default::default, |c| {
                    c.iter().map(|e| get_hierarchy(world, e)).collect()
                }),
        }
    }

    #[test]
    fn hierarchy() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let child1 = world.spawn(ChildOf(root)).id();
        let grandchild = world.spawn(ChildOf(child1)).id();
        let child2 = world.spawn(ChildOf(root)).id();

        // Spawn
        let hierarchy = get_hierarchy(&world, root);
        assert_eq!(
            hierarchy,
            Node::new_with(
                root,
                vec![
                    Node::new_with(child1, vec![Node::new(grandchild)]),
                    Node::new(child2)
                ]
            )
        );

        // Removal
        world.entity_mut(child1).remove::<ChildOf>();
        let hierarchy = get_hierarchy(&world, root);
        assert_eq!(hierarchy, Node::new_with(root, vec![Node::new(child2)]));

        // Insert
        world.entity_mut(child1).insert(ChildOf(root));
        let hierarchy = get_hierarchy(&world, root);
        assert_eq!(
            hierarchy,
            Node::new_with(
                root,
                vec![
                    Node::new(child2),
                    Node::new_with(child1, vec![Node::new(grandchild)])
                ]
            )
        );

        // Recursive Despawn
        world.entity_mut(root).despawn();
        assert!(world.get_entity(root).is_err());
        assert!(world.get_entity(child1).is_err());
        assert!(world.get_entity(child2).is_err());
        assert!(world.get_entity(grandchild).is_err());
    }

    #[test]
    fn with_children() {
        let mut world = World::new();
        let mut child1 = Entity::PLACEHOLDER;
        let mut child2 = Entity::PLACEHOLDER;
        let root = world
            .spawn_empty()
            .with_children(|p| {
                child1 = p.spawn_empty().id();
                child2 = p.spawn_empty().id();
            })
            .id();

        let hierarchy = get_hierarchy(&world, root);
        assert_eq!(
            hierarchy,
            Node::new_with(root, vec![Node::new(child1), Node::new(child2)])
        );
    }

    #[test]
    fn add_children() {
        let mut world = World::new();
        let child1 = world.spawn_empty().id();
        let child2 = world.spawn_empty().id();
        let root = world.spawn_empty().add_children(&[child1, child2]).id();

        let hierarchy = get_hierarchy(&world, root);
        assert_eq!(
            hierarchy,
            Node::new_with(root, vec![Node::new(child1), Node::new(child2)])
        );
    }

    #[test]
    fn insert_children() {
        let mut world = World::new();
        let child1 = world.spawn_empty().id();
        let child2 = world.spawn_empty().id();
        let child3 = world.spawn_empty().id();
        let child4 = world.spawn_empty().id();

        let mut entity_world_mut = world.spawn_empty();

        let first_children = entity_world_mut.add_children(&[child1, child2]);

        let root = first_children.insert_children(1, &[child3, child4]).id();

        let hierarchy = get_hierarchy(&world, root);
        assert_eq!(
            hierarchy,
            Node::new_with(
                root,
                vec![
                    Node::new(child1),
                    Node::new(child3),
                    Node::new(child4),
                    Node::new(child2)
                ]
            )
        );
    }

    #[test]
    fn insert_child() {
        let mut world = World::new();
        let child1 = world.spawn_empty().id();
        let child2 = world.spawn_empty().id();
        let child3 = world.spawn_empty().id();

        let mut entity_world_mut = world.spawn_empty();

        let first_children = entity_world_mut.add_children(&[child1, child2]);

        let root = first_children.insert_child(1, child3).id();

        let hierarchy = get_hierarchy(&world, root);
        assert_eq!(
            hierarchy,
            Node::new_with(
                root,
                vec![Node::new(child1), Node::new(child3), Node::new(child2)]
            )
        );
    }

    // regression test for https://github.com/bevyengine/bevy/pull/19134
    #[test]
    fn insert_children_index_bound() {
        let mut world = World::new();
        let child1 = world.spawn_empty().id();
        let child2 = world.spawn_empty().id();
        let child3 = world.spawn_empty().id();
        let child4 = world.spawn_empty().id();

        let mut entity_world_mut = world.spawn_empty();

        let first_children = entity_world_mut.add_children(&[child1, child2]).id();
        let hierarchy = get_hierarchy(&world, first_children);
        assert_eq!(
            hierarchy,
            Node::new_with(first_children, vec![Node::new(child1), Node::new(child2)])
        );

        let root = world
            .entity_mut(first_children)
            .insert_children(usize::MAX, &[child3, child4])
            .id();
        let hierarchy = get_hierarchy(&world, root);
        assert_eq!(
            hierarchy,
            Node::new_with(
                root,
                vec![
                    Node::new(child1),
                    Node::new(child2),
                    Node::new(child3),
                    Node::new(child4),
                ]
            )
        );
    }

    #[test]
    fn remove_children() {
        let mut world = World::new();
        let child1 = world.spawn_empty().id();
        let child2 = world.spawn_empty().id();
        let child3 = world.spawn_empty().id();
        let child4 = world.spawn_empty().id();

        let mut root = world.spawn_empty();
        root.add_children(&[child1, child2, child3, child4]);
        root.remove_children(&[child2, child3]);
        let root = root.id();

        let hierarchy = get_hierarchy(&world, root);
        assert_eq!(
            hierarchy,
            Node::new_with(root, vec![Node::new(child1), Node::new(child4)])
        );
    }

    #[test]
    fn remove_child() {
        let mut world = World::new();
        let child1 = world.spawn_empty().id();
        let child2 = world.spawn_empty().id();
        let child3 = world.spawn_empty().id();

        let mut root = world.spawn_empty();
        root.add_children(&[child1, child2, child3]);
        root.remove_child(child2);
        let root = root.id();

        let hierarchy = get_hierarchy(&world, root);
        assert_eq!(
            hierarchy,
            Node::new_with(root, vec![Node::new(child1), Node::new(child3)])
        );
    }

    #[test]
    fn self_parenting_invalid() {
        let mut world = World::new();
        let id = world.spawn_empty().id();
        world.entity_mut(id).insert(ChildOf(id));
        assert!(
            world.entity(id).get::<ChildOf>().is_none(),
            "invalid ChildOf relationships should self-remove"
        );
    }

    #[test]
    fn missing_parent_invalid() {
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        world.entity_mut(parent).despawn();
        let id = world.spawn(ChildOf(parent)).id();
        assert!(
            world.entity(id).get::<ChildOf>().is_none(),
            "invalid ChildOf relationships should self-remove"
        );
    }

    #[test]
    fn reinsert_same_parent() {
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        let id = world.spawn(ChildOf(parent)).id();
        world.entity_mut(id).insert(ChildOf(parent));
        assert_eq!(
            Some(&ChildOf(parent)),
            world.entity(id).get::<ChildOf>(),
            "ChildOf should still be there"
        );
    }

    #[test]
    fn spawn_children() {
        let mut world = World::new();
        let id = world.spawn(Children::spawn((Spawn(()), Spawn(())))).id();
        assert_eq!(world.entity(id).get::<Children>().unwrap().len(), 2,);
    }

    #[test]
    fn spawn_many_children() {
        let mut world = World::new();

        // 12 children should result in a flat tuple
        let id = world
            .spawn(children![(), (), (), (), (), (), (), (), (), (), (), ()])
            .id();

        assert_eq!(world.entity(id).get::<Children>().unwrap().len(), 12,);

        // 13 will start nesting, but should nonetheless produce a flat hierarchy
        let id = world
            .spawn(children![
                (),
                (),
                (),
                (),
                (),
                (),
                (),
                (),
                (),
                (),
                (),
                (),
                (),
            ])
            .id();

        assert_eq!(world.entity(id).get::<Children>().unwrap().len(), 13,);
    }

    #[test]
    fn replace_children() {
        let mut world = World::new();
        let parent = world.spawn(Children::spawn((Spawn(()), Spawn(())))).id();
        let &[child_a, child_b] = &world.entity(parent).get::<Children>().unwrap().0[..] else {
            panic!("Tried to spawn 2 children on an entity and didn't get 2 children");
        };

        let child_c = world.spawn_empty().id();

        world
            .entity_mut(parent)
            .replace_children(&[child_a, child_c]);

        let children = world.entity(parent).get::<Children>().unwrap();

        assert!(children.contains(&child_a));
        assert!(children.contains(&child_c));
        assert!(!children.contains(&child_b));

        assert_eq!(
            world.entity(child_a).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
        assert_eq!(
            world.entity(child_c).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
        assert!(world.entity(child_b).get::<ChildOf>().is_none());
    }

    #[test]
    fn replace_children_with_nothing() {
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        let child_a = world.spawn_empty().id();
        let child_b = world.spawn_empty().id();

        world.entity_mut(parent).add_children(&[child_a, child_b]);

        assert_eq!(world.entity(parent).get::<Children>().unwrap().len(), 2);

        world.entity_mut(parent).replace_children(&[]);

        assert!(world.entity(child_a).get::<ChildOf>().is_none());
        assert!(world.entity(child_b).get::<ChildOf>().is_none());
    }

    #[test]
    fn insert_same_child_twice() {
        let mut world = World::new();

        let parent = world.spawn_empty().id();
        let child = world.spawn_empty().id();

        world.entity_mut(parent).add_child(child);
        world.entity_mut(parent).add_child(child);

        let children = world.get::<Children>(parent).unwrap();
        assert_eq!(children.0, [child]);
        assert_eq!(
            world.entity(child).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
    }

    #[test]
    fn replace_with_difference() {
        let mut world = World::new();

        let parent = world.spawn_empty().id();
        let child_a = world.spawn_empty().id();
        let child_b = world.spawn_empty().id();
        let child_c = world.spawn_empty().id();
        let child_d = world.spawn_empty().id();

        // Test inserting new relations
        world.entity_mut(parent).replace_children_with_difference(
            &[],
            &[child_a, child_b],
            &[child_a, child_b],
        );

        assert_eq!(
            world.entity(child_a).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
        assert_eq!(
            world.entity(child_b).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
        assert_eq!(
            world.entity(parent).get::<Children>().unwrap().0,
            [child_a, child_b]
        );

        // Test replacing relations and changing order
        world.entity_mut(parent).replace_children_with_difference(
            &[child_b],
            &[child_d, child_c, child_a],
            &[child_c, child_d],
        );
        assert_eq!(
            world.entity(child_a).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
        assert_eq!(
            world.entity(child_c).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
        assert_eq!(
            world.entity(child_d).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
        assert_eq!(
            world.entity(parent).get::<Children>().unwrap().0,
            [child_d, child_c, child_a]
        );
        assert!(!world.entity(child_b).contains::<ChildOf>());

        // Test removing relationships
        world.entity_mut(parent).replace_children_with_difference(
            &[child_a, child_d, child_c],
            &[],
            &[],
        );
        assert!(!world.entity(parent).contains::<Children>());
        assert!(!world.entity(child_a).contains::<ChildOf>());
        assert!(!world.entity(child_b).contains::<ChildOf>());
        assert!(!world.entity(child_c).contains::<ChildOf>());
        assert!(!world.entity(child_d).contains::<ChildOf>());
    }

    #[test]
    fn replace_with_difference_on_empty() {
        let mut world = World::new();

        let parent = world.spawn_empty().id();
        let child_a = world.spawn_empty().id();

        world
            .entity_mut(parent)
            .replace_children_with_difference(&[child_a], &[], &[]);

        assert!(!world.entity(parent).contains::<Children>());
        assert!(!world.entity(child_a).contains::<ChildOf>());
    }

    #[test]
    fn replace_with_difference_totally_new_children() {
        let mut world = World::new();

        let parent = world.spawn_empty().id();
        let child_a = world.spawn_empty().id();
        let child_b = world.spawn_empty().id();
        let child_c = world.spawn_empty().id();
        let child_d = world.spawn_empty().id();

        // Test inserting new relations
        world.entity_mut(parent).replace_children_with_difference(
            &[],
            &[child_a, child_b],
            &[child_a, child_b],
        );

        assert_eq!(
            world.entity(child_a).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
        assert_eq!(
            world.entity(child_b).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
        assert_eq!(
            world.entity(parent).get::<Children>().unwrap().0,
            [child_a, child_b]
        );

        // Test replacing relations and changing order
        world.entity_mut(parent).replace_children_with_difference(
            &[child_b, child_a],
            &[child_d, child_c],
            &[child_c, child_d],
        );
        assert_eq!(
            world.entity(child_c).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
        assert_eq!(
            world.entity(child_d).get::<ChildOf>().unwrap(),
            &ChildOf(parent)
        );
        assert_eq!(
            world.entity(parent).get::<Children>().unwrap().0,
            [child_d, child_c]
        );
        assert!(!world.entity(child_a).contains::<ChildOf>());
        assert!(!world.entity(child_b).contains::<ChildOf>());
    }

    #[test]
    fn replace_children_order() {
        let mut world = World::new();

        let parent = world.spawn_empty().id();
        let child_a = world.spawn_empty().id();
        let child_b = world.spawn_empty().id();
        let child_c = world.spawn_empty().id();
        let child_d = world.spawn_empty().id();

        let initial_order = [child_a, child_b, child_c, child_d];
        world.entity_mut(parent).add_children(&initial_order);

        assert_eq!(
            world.entity_mut(parent).get::<Children>().unwrap().0,
            initial_order
        );

        let new_order = [child_d, child_b, child_a, child_c];
        world.entity_mut(parent).replace_children(&new_order);

        assert_eq!(world.entity(parent).get::<Children>().unwrap().0, new_order);
    }

    #[test]
    #[should_panic]
    #[cfg_attr(
        not(debug_assertions),
        ignore = "we don't check invariants if debug assertions are off"
    )]
    fn replace_diff_invariant_overlapping_unrelate_with_relate() {
        let mut world = World::new();

        let parent = world.spawn_empty().id();
        let child_a = world.spawn_empty().id();

        world
            .entity_mut(parent)
            .replace_children_with_difference(&[], &[child_a], &[child_a]);

        // This should panic
        world
            .entity_mut(parent)
            .replace_children_with_difference(&[child_a], &[child_a], &[]);
    }

    #[test]
    #[should_panic]
    #[cfg_attr(
        not(debug_assertions),
        ignore = "we don't check invariants if debug assertions are off"
    )]
    fn replace_diff_invariant_overlapping_unrelate_with_newly() {
        let mut world = World::new();

        let parent = world.spawn_empty().id();
        let child_a = world.spawn_empty().id();
        let child_b = world.spawn_empty().id();

        world
            .entity_mut(parent)
            .replace_children_with_difference(&[], &[child_a], &[child_a]);

        // This should panic
        world.entity_mut(parent).replace_children_with_difference(
            &[child_b],
            &[child_a, child_b],
            &[child_b],
        );
    }

    #[test]
    #[should_panic]
    #[cfg_attr(
        not(debug_assertions),
        ignore = "we don't check invariants if debug assertions are off"
    )]
    fn replace_diff_invariant_newly_not_subset() {
        let mut world = World::new();

        let parent = world.spawn_empty().id();
        let child_a = world.spawn_empty().id();
        let child_b = world.spawn_empty().id();

        // This should panic
        world.entity_mut(parent).replace_children_with_difference(
            &[],
            &[child_a, child_b],
            &[child_a],
        );
    }

    #[test]
    fn child_replace_hook_skip() {
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        let other = world.spawn_empty().id();
        let child = world.spawn(ChildOf(parent)).id();
        world
            .entity_mut(child)
            .insert_with_relationship_hook_mode(ChildOf(other), RelationshipHookMode::Skip);
        assert_eq!(
            &**world.entity(parent).get::<Children>().unwrap(),
            &[child],
            "Children should still have the old value, as on_insert/on_replace didn't run"
        );
    }
}
