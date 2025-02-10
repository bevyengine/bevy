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
    component::{Component, HookContext},
    entity::Entity,
    relationship::{RelatedSpawner, RelatedSpawnerCommands},
    system::EntityCommands,
    world::{DeferredWorld, EntityWorldMut, FromWorld, World},
};
use alloc::{format, string::String, vec::Vec};
use core::ops::Deref;
use core::slice;
use disqualified::ShortName;
use log::warn;

/// A [`Relationship`](crate::relationship::Relationship) component that creates the canonical
/// "parent / child" hierarchy. This is the "source of truth" component, and it pairs with
/// the [`Children`] [`RelationshipTarget`](crate::relationship::RelationshipTarget).
///
/// This relationship should be used for things like:
///
/// 1. Organizing entities in a scene
/// 2. Propagating configuration or data inherited from a parent, such as "visibility" or "world-space global transforms".
/// 3. Ensuring a hierarchy is despawned when an entity is despawned.
/// 4.
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
#[derive(Component, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(
    feature = "bevy_reflect",
    reflect(Component, PartialEq, Debug, FromWorld)
)]
#[relationship(relationship_target = Children)]
pub struct ChildOf(pub Entity);

impl ChildOf {
    /// Returns the "target" entity.
    pub fn get(&self) -> Entity {
        self.0
    }
}

impl Deref for ChildOf {
    type Target = Entity;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
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

/// A [`RelationshipTarget`](crate::relationship::RelationshipTarget) collection component that is populated
/// with entities that "target" this entity with the [`ChildOf`] [`Relationship`](crate::relationship::Relationship) component.
///
/// Together, these components form the "canonical parent-child hierarchy". See the [`ChildOf`] component for all full
/// description of this relationship and instructions on how to use it.
#[derive(Component, Default, Debug, PartialEq, Eq)]
#[relationship_target(relationship = ChildOf, linked_spawn)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Component, FromWorld))]
pub struct Children(Vec<Entity>);

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
    pub fn with_children(&mut self, func: impl FnOnce(&mut ChildSpawner)) -> &mut Self {
        self.with_related(func);
        self
    }

    /// Adds the given children to this entity
    pub fn add_children(&mut self, children: &[Entity]) -> &mut Self {
        self.add_related::<ChildOf>(children)
    }

    /// Adds the given child to this entity
    pub fn add_child(&mut self, child: Entity) -> &mut Self {
        self.add_related::<ChildOf>(&[child])
    }

    /// Spawns the passed bundle and adds it to this entity as a child.
    ///
    /// For efficient spawning of multiple children, use [`with_children`].
    ///
    /// [`with_children`]: EntityWorldMut::with_children
    pub fn with_child(&mut self, bundle: impl Bundle) -> &mut Self {
        let id = self.id();
        self.world_scope(|world| {
            world.spawn((bundle, ChildOf(id)));
        });
        self
    }

    /// Removes the [`ChildOf`] component, if it exists.
    #[deprecated(since = "0.16.0", note = "Use entity_mut.remove::<ChildOf>()")]
    pub fn remove_parent(&mut self) -> &mut Self {
        self.remove::<ChildOf>();
        self
    }

    /// Inserts the [`ChildOf`] component with the given `parent` entity, if it exists.
    #[deprecated(since = "0.16.0", note = "Use entity_mut.insert(ChildOf(entity))")]
    pub fn set_parent(&mut self, parent: Entity) -> &mut Self {
        self.insert(ChildOf(parent));
        self
    }
}

impl<'a> EntityCommands<'a> {
    /// Spawns children of this entity (with a [`ChildOf`] relationship) by taking a function that operates on a [`ChildSpawner`].
    pub fn with_children(
        &mut self,
        func: impl FnOnce(&mut RelatedSpawnerCommands<ChildOf>),
    ) -> &mut Self {
        self.with_related(func);
        self
    }

    /// Adds the given children to this entity
    pub fn add_children(&mut self, children: &[Entity]) -> &mut Self {
        self.add_related::<ChildOf>(children)
    }

    /// Adds the given child to this entity
    pub fn add_child(&mut self, child: Entity) -> &mut Self {
        self.add_related::<ChildOf>(&[child])
    }

    /// Spawns the passed bundle and adds it to this entity as a child.
    ///
    /// For efficient spawning of multiple children, use [`with_children`].
    ///
    /// [`with_children`]: EntityCommands::with_children
    pub fn with_child(&mut self, bundle: impl Bundle) -> &mut Self {
        let id = self.id();
        self.commands.spawn((bundle, ChildOf(id)));
        self
    }

    /// Removes the [`ChildOf`] component, if it exists.
    #[deprecated(since = "0.16.0", note = "Use entity_commands.remove::<ChildOf>()")]
    pub fn remove_parent(&mut self) -> &mut Self {
        self.remove::<ChildOf>();
        self
    }

    /// Inserts the [`ChildOf`] component with the given `parent` entity, if it exists.
    #[deprecated(since = "0.16.0", note = "Use entity_commands.insert(ChildOf(entity))")]
    pub fn set_parent(&mut self, parent: Entity) -> &mut Self {
        self.insert(ChildOf(parent));
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
    if !world
        .get_entity(child_of.get())
        .is_ok_and(|e| e.contains::<C>())
    {
        // TODO: print name here once Name lives in bevy_ecs
        let name: Option<String> = None;
        warn!(
            "warning[B0004]: {}{name} with the {ty_name} component has a parent without {ty_name}.\n\
            This will cause inconsistent behaviors! See: https://bevyengine.org/learn/errors/b0004",
            caller.map(|c| format!("{c}: ")).unwrap_or_default(),
            ty_name = ShortName::of::<C>(),
            name = name.map_or_else(
                || format!("Entity {}", entity),
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
       $crate::hierarchy::Children::spawn(($($crate::spawn::Spawn($child)),*))
    };
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::Entity,
        hierarchy::{ChildOf, Children},
        relationship::RelationshipTarget,
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
}
