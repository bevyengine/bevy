//! This module provides functionality to link entities to each other using specialized components called "relationships". See the [`Relationship`] trait for more info.

mod related_methods;
mod relationship_query;
mod relationship_source_collection;

use core::marker::PhantomData;

use alloc::format;

use bevy_utils::prelude::DebugName;
pub use related_methods::*;
pub use relationship_query::*;
pub use relationship_source_collection::*;

use crate::{
    component::{Component, ComponentCloneBehavior, Mutable},
    entity::{ComponentCloneCtx, Entity},
    error::CommandWithEntity,
    lifecycle::HookContext,
    world::{DeferredWorld, EntityWorldMut},
};
use log::warn;

/// A [`Component`] on a "source" [`Entity`] that references another target [`Entity`], creating a "relationship" between them. Every [`Relationship`]
/// has a corresponding [`RelationshipTarget`] type (and vice-versa), which exists on the "target" entity of a relationship and contains the list of all
/// "source" entities that relate to the given "target"
///
/// The [`Relationship`] component is the "source of truth" and the [`RelationshipTarget`] component reflects that source of truth. When a [`Relationship`]
/// component is inserted on an [`Entity`], the corresponding [`RelationshipTarget`] component is immediately inserted on the target component if it does
/// not already exist, and the "source" entity is automatically added to the [`RelationshipTarget`] collection (this is done via "component hooks").
///
/// A common example of a [`Relationship`] is the parent / child relationship. Bevy ECS includes a canonical form of this via the [`ChildOf`](crate::hierarchy::ChildOf)
/// [`Relationship`] and the [`Children`](crate::hierarchy::Children) [`RelationshipTarget`].
///
/// [`Relationship`] and [`RelationshipTarget`] should always be derived via the [`Component`] trait to ensure the hooks are set up properly.
///
/// ## Derive
///
/// [`Relationship`] and [`RelationshipTarget`] can only be derived for structs with a single unnamed field, single named field
/// or for named structs where one field is annotated with `#[relationship]`.
/// If there are additional fields, they must all implement [`Default`].
///
/// [`RelationshipTarget`] also requires that the relationship field is private to prevent direct mutation,
/// ensuring the correctness of relationships.
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::entity::Entity;
/// #[derive(Component)]
/// #[relationship(relationship_target = Children)]
/// pub struct ChildOf {
///     #[relationship]
///     pub parent: Entity,
///     internal: u8,
/// };
///
/// #[derive(Component)]
/// #[relationship_target(relationship = ChildOf)]
/// pub struct Children(Vec<Entity>);
/// ```
///
/// When deriving [`RelationshipTarget`] you can specify the `#[relationship_target(linked_spawn)]` attribute to
/// automatically despawn entities stored in an entity's [`RelationshipTarget`] when that entity is despawned:
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::entity::Entity;
/// #[derive(Component)]
/// #[relationship(relationship_target = Children)]
/// pub struct ChildOf(pub Entity);
///
/// #[derive(Component)]
/// #[relationship_target(relationship = ChildOf, linked_spawn)]
/// pub struct Children(Vec<Entity>);
/// ```
pub trait Relationship: Component + Sized {
    /// The [`Component`] added to the "target" entities of this [`Relationship`], which contains the list of all "source"
    /// entities that relate to the "target".
    type RelationshipTarget: RelationshipTarget<Relationship = Self>;

    /// Gets the [`Entity`] ID of the related entity.
    fn get(&self) -> Entity;

    /// Creates this [`Relationship`] from the given `entity`.
    fn from(entity: Entity) -> Self;

    /// Changes the current [`Entity`] ID of the entity containing the [`RelationshipTarget`] to another one.
    ///
    /// This is useful for updating the relationship without overwriting other fields stored in `Self`.
    ///
    /// # Warning
    ///
    /// This should generally not be called by user code, as modifying the related entity could invalidate the
    /// relationship. If this method is used, then the hooks [`on_replace`](Relationship::on_replace) have to
    /// run before and [`on_insert`](Relationship::on_insert) after it.
    /// This happens automatically when this method is called with [`EntityWorldMut::modify_component`].
    ///
    /// Prefer to use regular means of insertions when possible.
    fn set_risky(&mut self, entity: Entity);

    /// The `on_insert` component hook that maintains the [`Relationship`] / [`RelationshipTarget`] connection.
    fn on_insert(
        mut world: DeferredWorld,
        HookContext {
            entity,
            caller,
            relationship_hook_mode,
            ..
        }: HookContext,
    ) {
        match relationship_hook_mode {
            RelationshipHookMode::Run => {}
            RelationshipHookMode::Skip => return,
            RelationshipHookMode::RunIfNotLinked => {
                if <Self::RelationshipTarget as RelationshipTarget>::LINKED_SPAWN {
                    return;
                }
            }
        }
        let target_entity = world.entity(entity).get::<Self>().unwrap().get();
        if target_entity == entity {
            warn!(
                "{}The {}({target_entity:?}) relationship on entity {entity:?} points to itself. The invalid {} relationship has been removed.",
                caller.map(|location|format!("{location}: ")).unwrap_or_default(),
                DebugName::type_name::<Self>(),
                DebugName::type_name::<Self>()
            );
            world.commands().entity(entity).remove::<Self>();
            return;
        }
        // For one-to-one relationships, remove existing relationship before adding new one
        let current_source_to_remove = world
            .get_entity(target_entity)
            .ok()
            .and_then(|target_entity_ref| target_entity_ref.get::<Self::RelationshipTarget>())
            .and_then(|relationship_target| {
                relationship_target
                    .collection()
                    .source_to_remove_before_add()
            });

        if let Some(current_source) = current_source_to_remove {
            world.commands().entity(current_source).try_remove::<Self>();
        }

        if let Ok(mut entity_commands) = world.commands().get_entity(target_entity) {
            // Deferring is necessary for batch mode
            entity_commands
                .entry::<Self::RelationshipTarget>()
                .and_modify(move |mut relationship_target| {
                    relationship_target.collection_mut_risky().add(entity);
                })
                .or_insert_with(move || {
                    let mut target = Self::RelationshipTarget::with_capacity(1);
                    target.collection_mut_risky().add(entity);
                    target
                });
        } else {
            warn!(
                "{}The {}({target_entity:?}) relationship on entity {entity:?} relates to an entity that does not exist. The invalid {} relationship has been removed.",
                caller.map(|location|format!("{location}: ")).unwrap_or_default(),
                DebugName::type_name::<Self>(),
                DebugName::type_name::<Self>()
            );
            world.commands().entity(entity).remove::<Self>();
        }
    }

    /// The `on_replace` component hook that maintains the [`Relationship`] / [`RelationshipTarget`] connection.
    // note: think of this as "on_drop"
    fn on_replace(
        mut world: DeferredWorld,
        HookContext {
            entity,
            relationship_hook_mode,
            ..
        }: HookContext,
    ) {
        match relationship_hook_mode {
            RelationshipHookMode::Run => {}
            RelationshipHookMode::Skip => return,
            RelationshipHookMode::RunIfNotLinked => {
                if <Self::RelationshipTarget as RelationshipTarget>::LINKED_SPAWN {
                    return;
                }
            }
        }
        let target_entity = world.entity(entity).get::<Self>().unwrap().get();
        if let Ok(mut target_entity_mut) = world.get_entity_mut(target_entity) {
            if let Some(mut relationship_target) =
                target_entity_mut.get_mut::<Self::RelationshipTarget>()
            {
                relationship_target.collection_mut_risky().remove(entity);
                if relationship_target.len() == 0 {
                    let command = |mut entity: EntityWorldMut| {
                        // this "remove" operation must check emptiness because in the event that an identical
                        // relationship is inserted on top, this despawn would result in the removal of that identical
                        // relationship ... not what we want!
                        if entity
                            .get::<Self::RelationshipTarget>()
                            .is_some_and(RelationshipTarget::is_empty)
                        {
                            entity.remove::<Self::RelationshipTarget>();
                        }
                    };

                    world
                        .commands()
                        .queue_silenced(command.with_entity(target_entity));
                }
            }
        }
    }
}

/// The iterator type for the source entities in a [`RelationshipTarget`] collection,
/// as defined in the [`RelationshipSourceCollection`] trait.
pub type SourceIter<'w, R> =
    <<R as RelationshipTarget>::Collection as RelationshipSourceCollection>::SourceIter<'w>;

/// A [`Component`] containing the collection of entities that relate to this [`Entity`] via the associated `Relationship` type.
/// See the [`Relationship`] documentation for more information.
pub trait RelationshipTarget: Component<Mutability = Mutable> + Sized {
    /// If this is true, when despawning or cloning (when [linked cloning is enabled](crate::entity::EntityClonerBuilder::linked_cloning)), the related entities targeting this entity will also be despawned or cloned.
    ///
    /// For example, this is set to `true` for Bevy's built-in parent-child relation, defined by [`ChildOf`](crate::prelude::ChildOf) and [`Children`](crate::prelude::Children).
    /// This means that when a parent is despawned, any children targeting that parent are also despawned (and the same applies to cloning).
    ///
    /// To get around this behavior, you can first break the relationship between entities, and *then* despawn or clone.
    /// This defaults to false when derived.
    const LINKED_SPAWN: bool;
    /// The [`Relationship`] that populates this [`RelationshipTarget`] collection.
    type Relationship: Relationship<RelationshipTarget = Self>;
    /// The collection type that stores the "source" entities for this [`RelationshipTarget`] component.
    ///
    /// Check the list of types which implement [`RelationshipSourceCollection`] for the data structures that can be used inside of your component.
    /// If you need a new collection type, you can implement the [`RelationshipSourceCollection`] trait
    /// for a type you own which wraps the collection you want to use (to avoid the orphan rule),
    /// or open an issue on the Bevy repository to request first-party support for your collection type.
    type Collection: RelationshipSourceCollection;

    /// Returns a reference to the stored [`RelationshipTarget::Collection`].
    fn collection(&self) -> &Self::Collection;
    /// Returns a mutable reference to the stored [`RelationshipTarget::Collection`].
    ///
    /// # Warning
    /// This should generally not be called by user code, as modifying the internal collection could invalidate the relationship.
    /// The collection should not contain duplicates.
    fn collection_mut_risky(&mut self) -> &mut Self::Collection;

    /// Creates a new [`RelationshipTarget`] from the given [`RelationshipTarget::Collection`].
    ///
    /// # Warning
    /// This should generally not be called by user code, as constructing the internal collection could invalidate the relationship.
    /// The collection should not contain duplicates.
    fn from_collection_risky(collection: Self::Collection) -> Self;

    /// The `on_replace` component hook that maintains the [`Relationship`] / [`RelationshipTarget`] connection.
    // note: think of this as "on_drop"
    fn on_replace(
        mut world: DeferredWorld,
        HookContext {
            entity,
            relationship_hook_mode,
            ..
        }: HookContext,
    ) {
        match relationship_hook_mode {
            RelationshipHookMode::Run => {}
            // For RelationshipTarget we don't want to run this hook even if it isn't linked, but for Relationship we do.
            RelationshipHookMode::Skip | RelationshipHookMode::RunIfNotLinked => return,
        }
        let (entities, mut commands) = world.entities_and_commands();
        let relationship_target = entities.get(entity).unwrap().get::<Self>().unwrap();
        for source_entity in relationship_target.iter() {
            commands
                .entity(source_entity)
                .try_remove::<Self::Relationship>();
        }
    }

    /// The `on_despawn` component hook that despawns entities stored in an entity's [`RelationshipTarget`] when
    /// that entity is despawned.
    // note: think of this as "on_drop"
    fn on_despawn(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
        let (entities, mut commands) = world.entities_and_commands();
        let relationship_target = entities.get(entity).unwrap().get::<Self>().unwrap();
        for source_entity in relationship_target.iter() {
            commands.entity(source_entity).try_despawn();
        }
    }

    /// Creates this [`RelationshipTarget`] with the given pre-allocated entity capacity.
    fn with_capacity(capacity: usize) -> Self {
        let collection =
            <Self::Collection as RelationshipSourceCollection>::with_capacity(capacity);
        Self::from_collection_risky(collection)
    }

    /// Iterates the entities stored in this collection.
    #[inline]
    fn iter(&self) -> SourceIter<'_, Self> {
        self.collection().iter()
    }

    /// Returns the number of entities in this collection.
    #[inline]
    fn len(&self) -> usize {
        self.collection().len()
    }

    /// Returns true if this entity collection is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.collection().is_empty()
    }
}

/// The "clone behavior" for [`RelationshipTarget`]. The [`RelationshipTarget`] will be populated with the proper components
/// when the corresponding [`Relationship`] sources of truth are inserted. Cloning the actual entities
/// in the original [`RelationshipTarget`] would result in duplicates, so we don't do that!
///
/// This will also queue up clones of the relationship sources if the [`EntityCloner`](crate::entity::EntityCloner) is configured
/// to spawn recursively.
pub fn clone_relationship_target<T: RelationshipTarget>(
    component: &T,
    cloned: &mut T,
    context: &mut ComponentCloneCtx,
) {
    if context.linked_cloning() && T::LINKED_SPAWN {
        let collection = cloned.collection_mut_risky();
        for entity in component.iter() {
            collection.add(entity);
            context.queue_entity_clone(entity);
        }
    } else if context.moving() {
        let target = context.target();
        let collection = cloned.collection_mut_risky();
        for entity in component.iter() {
            collection.add(entity);
            context.queue_deferred(move |world, _mapper| {
                // We don't want relationships hooks to run because we are manually constructing the collection here
                _ = DeferredWorld::from(world)
                    .modify_component_with_relationship_hook_mode::<T::Relationship, ()>(
                        entity,
                        RelationshipHookMode::Skip,
                        |r| r.set_risky(target),
                    );
            });
        }
    }
}

/// Configures the conditions under which the Relationship insert/replace hooks will be run.
#[derive(Copy, Clone, Debug)]
pub enum RelationshipHookMode {
    /// Relationship insert/replace hooks will always run
    Run,
    /// Relationship insert/replace hooks will run if [`RelationshipTarget::LINKED_SPAWN`] is false
    RunIfNotLinked,
    /// Relationship insert/replace hooks will always be skipped
    Skip,
}

/// Wrapper for components clone specialization using autoderef.
#[doc(hidden)]
pub struct RelationshipCloneBehaviorSpecialization<T>(PhantomData<T>);

impl<T> Default for RelationshipCloneBehaviorSpecialization<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Base trait for relationship clone specialization using autoderef.
#[doc(hidden)]
pub trait RelationshipCloneBehaviorBase {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior;
}

impl<C> RelationshipCloneBehaviorBase for RelationshipCloneBehaviorSpecialization<C> {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior {
        // Relationships currently must have `Clone`/`Reflect`-based handler for cloning/moving logic to properly work.
        ComponentCloneBehavior::Ignore
    }
}

/// Specialized trait for relationship clone specialization using autoderef.
#[doc(hidden)]
pub trait RelationshipCloneBehaviorViaReflect {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior;
}

#[cfg(feature = "bevy_reflect")]
impl<C: Relationship + bevy_reflect::Reflect> RelationshipCloneBehaviorViaReflect
    for &RelationshipCloneBehaviorSpecialization<C>
{
    fn default_clone_behavior(&self) -> ComponentCloneBehavior {
        ComponentCloneBehavior::reflect()
    }
}

/// Specialized trait for relationship clone specialization using autoderef.
#[doc(hidden)]
pub trait RelationshipCloneBehaviorViaClone {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior;
}

impl<C: Relationship + Clone> RelationshipCloneBehaviorViaClone
    for &&RelationshipCloneBehaviorSpecialization<C>
{
    fn default_clone_behavior(&self) -> ComponentCloneBehavior {
        ComponentCloneBehavior::clone::<C>()
    }
}

/// Specialized trait for relationship target clone specialization using autoderef.
#[doc(hidden)]
pub trait RelationshipTargetCloneBehaviorViaReflect {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior;
}

#[cfg(feature = "bevy_reflect")]
impl<C: RelationshipTarget + bevy_reflect::Reflect + bevy_reflect::TypePath>
    RelationshipTargetCloneBehaviorViaReflect for &&&RelationshipCloneBehaviorSpecialization<C>
{
    fn default_clone_behavior(&self) -> ComponentCloneBehavior {
        ComponentCloneBehavior::Custom(|source, context| {
            if let Some(component) = source.read::<C>()
                && let Ok(mut cloned) = component.reflect_clone_and_take::<C>()
            {
                cloned.collection_mut_risky().clear();
                clone_relationship_target(component, &mut cloned, context);
                context.write_target_component(cloned);
            }
        })
    }
}

/// Specialized trait for relationship target clone specialization using autoderef.
#[doc(hidden)]
pub trait RelationshipTargetCloneBehaviorViaClone {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior;
}

impl<C: RelationshipTarget + Clone> RelationshipTargetCloneBehaviorViaClone
    for &&&&RelationshipCloneBehaviorSpecialization<C>
{
    fn default_clone_behavior(&self) -> ComponentCloneBehavior {
        ComponentCloneBehavior::Custom(|source, context| {
            if let Some(component) = source.read::<C>() {
                let mut cloned = component.clone();
                cloned.collection_mut_risky().clear();
                clone_relationship_target(component, &mut cloned, context);
                context.write_target_component(cloned);
            }
        })
    }
}

/// We know there's no additional data on Children, so this handler is an optimization to avoid cloning the entire Collection.
#[doc(hidden)]
pub trait RelationshipTargetCloneBehaviorHierarchy {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior;
}

impl RelationshipTargetCloneBehaviorHierarchy
    for &&&&&RelationshipCloneBehaviorSpecialization<crate::hierarchy::Children>
{
    fn default_clone_behavior(&self) -> ComponentCloneBehavior {
        ComponentCloneBehavior::Custom(|source, context| {
            if let Some(component) = source.read::<crate::hierarchy::Children>() {
                let mut cloned = crate::hierarchy::Children::with_capacity(component.len());
                clone_relationship_target(component, &mut cloned, context);
                context.write_target_component(cloned);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;

    use crate::prelude::{ChildOf, Children};
    use crate::world::World;
    use crate::{component::Component, entity::Entity};
    use alloc::vec::Vec;

    #[test]
    fn custom_relationship() {
        #[derive(Component)]
        #[relationship(relationship_target = LikedBy)]
        struct Likes(pub Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Likes)]
        struct LikedBy(Vec<Entity>);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn(Likes(a)).id();
        let c = world.spawn(Likes(a)).id();
        assert_eq!(world.entity(a).get::<LikedBy>().unwrap().0, &[b, c]);
    }

    #[test]
    fn self_relationship_fails() {
        #[derive(Component)]
        #[relationship(relationship_target = RelTarget)]
        struct Rel(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Rel)]
        struct RelTarget(Vec<Entity>);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        world.entity_mut(a).insert(Rel(a));
        assert!(!world.entity(a).contains::<Rel>());
        assert!(!world.entity(a).contains::<RelTarget>());
    }

    #[test]
    fn relationship_with_missing_target_fails() {
        #[derive(Component)]
        #[relationship(relationship_target = RelTarget)]
        struct Rel(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Rel)]
        struct RelTarget(Vec<Entity>);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        world.despawn(a);
        let b = world.spawn(Rel(a)).id();
        assert!(!world.entity(b).contains::<Rel>());
        assert!(!world.entity(b).contains::<RelTarget>());
    }

    #[test]
    fn relationship_with_multiple_non_target_fields_compiles() {
        #[derive(Component)]
        #[relationship(relationship_target=Target)]
        #[expect(dead_code, reason = "test struct")]
        struct Source {
            #[relationship]
            target: Entity,
            foo: u8,
            bar: u8,
        }

        #[derive(Component)]
        #[relationship_target(relationship=Source)]
        struct Target(Vec<Entity>);

        // No assert necessary, looking to make sure compilation works with the macros
    }
    #[test]
    fn relationship_target_with_multiple_non_target_fields_compiles() {
        #[derive(Component)]
        #[relationship(relationship_target=Target)]
        struct Source(Entity);

        #[derive(Component)]
        #[relationship_target(relationship=Source)]
        #[expect(dead_code, reason = "test struct")]
        struct Target {
            #[relationship]
            target: Vec<Entity>,
            foo: u8,
            bar: u8,
        }

        // No assert necessary, looking to make sure compilation works with the macros
    }

    #[test]
    fn relationship_with_multiple_unnamed_non_target_fields_compiles() {
        #[derive(Component)]
        #[relationship(relationship_target=Target<T>)]
        struct Source<T: Send + Sync + 'static>(#[relationship] Entity, PhantomData<T>);

        #[derive(Component)]
        #[relationship_target(relationship=Source<T>)]
        struct Target<T: Send + Sync + 'static>(#[relationship] Vec<Entity>, PhantomData<T>);

        // No assert necessary, looking to make sure compilation works with the macros
    }

    #[test]
    fn parent_child_relationship_with_custom_relationship() {
        #[derive(Component)]
        #[relationship(relationship_target = RelTarget)]
        struct Rel(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Rel)]
        struct RelTarget(Entity);

        let mut world = World::new();

        // Rel on Parent
        // Despawn Parent
        let mut commands = world.commands();
        let child = commands.spawn_empty().id();
        let parent = commands.spawn(Rel(child)).add_child(child).id();
        commands.entity(parent).despawn();
        world.flush();

        assert!(world.get_entity(child).is_err());
        assert!(world.get_entity(parent).is_err());

        // Rel on Parent
        // Despawn Child
        let mut commands = world.commands();
        let child = commands.spawn_empty().id();
        let parent = commands.spawn(Rel(child)).add_child(child).id();
        commands.entity(child).despawn();
        world.flush();

        assert!(world.get_entity(child).is_err());
        assert!(!world.entity(parent).contains::<Rel>());

        // Rel on Child
        // Despawn Parent
        let mut commands = world.commands();
        let parent = commands.spawn_empty().id();
        let child = commands.spawn((ChildOf(parent), Rel(parent))).id();
        commands.entity(parent).despawn();
        world.flush();

        assert!(world.get_entity(child).is_err());
        assert!(world.get_entity(parent).is_err());

        // Rel on Child
        // Despawn Child
        let mut commands = world.commands();
        let parent = commands.spawn_empty().id();
        let child = commands.spawn((ChildOf(parent), Rel(parent))).id();
        commands.entity(child).despawn();
        world.flush();

        assert!(world.get_entity(child).is_err());
        assert!(!world.entity(parent).contains::<RelTarget>());
    }

    #[test]
    fn spawn_batch_with_relationship() {
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        let children = world
            .spawn_batch((0..10).map(|_| ChildOf(parent)))
            .collect::<Vec<_>>();

        for &child in &children {
            assert!(world
                .get::<ChildOf>(child)
                .is_some_and(|child_of| child_of.parent() == parent));
        }
        assert!(world
            .get::<Children>(parent)
            .is_some_and(|children| children.len() == 10));
    }

    #[test]
    fn insert_batch_with_relationship() {
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        let child = world.spawn_empty().id();
        world.insert_batch([(child, ChildOf(parent))]);
        world.flush();

        assert!(world.get::<ChildOf>(child).is_some());
        assert!(world.get::<Children>(parent).is_some());
    }
}
