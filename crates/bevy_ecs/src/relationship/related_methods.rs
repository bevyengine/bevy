use crate::{
    bundle::Bundle,
    entity::{hash_set::EntityHashSet, Entity},
    relationship::{
        Relationship, RelationshipHookMode, RelationshipSourceCollection, RelationshipTarget,
    },
    system::{Commands, EntityCommands},
    world::{EntityWorldMut, World},
};
use bevy_platform::prelude::{Box, Vec};
use core::{marker::PhantomData, mem};

use super::OrderedRelationshipSourceCollection;

impl<'w> EntityWorldMut<'w> {
    /// Spawns a entity related to this entity (with the `R` relationship) by taking a bundle
    pub fn with_related<R: Relationship>(&mut self, bundle: impl Bundle) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            world.spawn((bundle, R::from(parent)));
        });
        self
    }

    /// Spawns entities related to this entity (with the `R` relationship) by taking a function that operates on a [`RelatedSpawner`].
    pub fn with_related_entities<R: Relationship>(
        &mut self,
        func: impl FnOnce(&mut RelatedSpawner<R>),
    ) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            func(&mut RelatedSpawner::new(world, parent));
        });
        self
    }

    /// Relates the given entities to this entity with the relation `R`.
    ///
    /// See [`add_one_related`](Self::add_one_related) if you want relate only one entity.
    pub fn add_related<R: Relationship>(&mut self, related: &[Entity]) -> &mut Self {
        let id = self.id();
        self.world_scope(|world| {
            for related in related {
                world.entity_mut(*related).insert(R::from(id));
            }
        });
        self
    }

    /// Relates the given entities to this entity with the relation `R`, starting at this particular index.
    ///
    /// If the `related` has duplicates, a related entity will take the index of its last occurrence in `related`.
    /// If the indices go out of bounds, they will be clamped into bounds.
    /// This will not re-order existing related entities unless they are in `related`.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    /// let e0 = world.spawn_empty().id();
    /// let e1 = world.spawn_empty().id();
    /// let e2 = world.spawn_empty().id();
    /// let e3 = world.spawn_empty().id();
    /// let e4 = world.spawn_empty().id();
    ///
    /// let mut main_entity = world.spawn_empty();
    /// main_entity.add_related::<ChildOf>(&[e0, e1, e2, e2]);
    /// main_entity.insert_related::<ChildOf>(1, &[e0, e3, e4, e4]);
    /// let main_id = main_entity.id();
    ///
    /// let relationship_source = main_entity.get::<Children>().unwrap().collection();
    /// assert_eq!(relationship_source, &[e1, e0, e3, e2, e4]);
    /// ```
    pub fn insert_related<R: Relationship>(&mut self, index: usize, related: &[Entity]) -> &mut Self
    where
        <R::RelationshipTarget as RelationshipTarget>::Collection:
            OrderedRelationshipSourceCollection,
    {
        let id = self.id();
        self.world_scope(|world| {
            for (offset, related) in related.iter().enumerate() {
                let index = index + offset;
                if world
                    .get::<R>(*related)
                    .is_some_and(|relationship| relationship.get() == id)
                {
                    world
                        .get_mut::<R::RelationshipTarget>(id)
                        .expect("hooks should have added relationship target")
                        .collection_mut_risky()
                        .place(*related, index);
                } else {
                    world.entity_mut(*related).insert(R::from(id));
                    world
                        .get_mut::<R::RelationshipTarget>(id)
                        .expect("hooks should have added relationship target")
                        .collection_mut_risky()
                        .place_most_recent(index);
                }
            }
        });

        self
    }

    /// Removes the relation `R` between this entity and the given entities.
    pub fn remove_related<R: Relationship>(&mut self, related: &[Entity]) -> &mut Self {
        let id = self.id();
        self.world_scope(|world| {
            for related in related {
                if world
                    .get::<R>(*related)
                    .is_some_and(|relationship| relationship.get() == id)
                {
                    world.entity_mut(*related).remove::<R>();
                }
            }
        });

        self
    }

    /// Replaces all the related entities with a new set of entities.
    pub fn replace_related<R: Relationship>(&mut self, related: &[Entity]) -> &mut Self {
        type Collection<R> =
            <<R as Relationship>::RelationshipTarget as RelationshipTarget>::Collection;

        if related.is_empty() {
            self.remove::<R::RelationshipTarget>();

            return self;
        }

        let Some(mut existing_relations) = self.get_mut::<R::RelationshipTarget>() else {
            return self.add_related::<R>(related);
        };

        // We take the collection here so we can modify it without taking the component itself (this would create archetype move).
        // SAFETY: We eventually return the correctly initialized collection into the target.
        let mut existing_relations = mem::replace(
            existing_relations.collection_mut_risky(),
            Collection::<R>::with_capacity(0),
        );

        let mut potential_relations = EntityHashSet::from_iter(related.iter().copied());

        let id = self.id();
        self.world_scope(|world| {
            for related in existing_relations.iter() {
                if !potential_relations.remove(related) {
                    world.entity_mut(related).remove::<R>();
                }
            }

            for related in potential_relations {
                // SAFETY: We'll manually be adjusting the contents of the parent to fit the final state.
                world
                    .entity_mut(related)
                    .insert_with_relationship_hook_mode(R::from(id), RelationshipHookMode::Skip);
            }
        });

        // SAFETY: The entities we're inserting will be the entities that were either already there or entities that we've just inserted.
        existing_relations.clear();
        existing_relations.extend_from_iter(related.iter().copied());
        self.insert(R::RelationshipTarget::from_collection_risky(
            existing_relations,
        ));

        self
    }

    /// Replaces all the related entities with a new set of entities.
    ///
    /// This is a more efficient of [`Self::replace_related`] which doesn't allocate.
    /// The passed in arguments must adhere to these invariants:
    /// - `entities_to_unrelate`: A slice of entities to remove from the relationship source.
    ///   Entities need not be related to this entity, but must not appear in `entities_to_relate`
    /// - `entities_to_relate`: A slice of entities to relate to this entity.
    ///   This must contain all entities that will remain related (i.e. not those in `entities_to_unrelate`) plus the newly related entities.
    /// - `newly_related_entities`: A subset of `entities_to_relate` containing only entities not already related to this entity.
    /// - Slices **must not** contain any duplicates
    ///
    /// # Warning
    ///
    /// Violating these invariants may lead to panics, crashes or unpredictable engine behavior.
    ///
    /// # Panics
    ///
    /// Panics when debug assertions are enabled and any invariants are broken.
    ///
    // TODO: Consider making these iterators so users aren't required to allocate a separate buffers for the different slices.
    pub fn replace_related_with_difference<R: Relationship>(
        &mut self,
        entities_to_unrelate: &[Entity],
        entities_to_relate: &[Entity],
        newly_related_entities: &[Entity],
    ) -> &mut Self {
        #[cfg(debug_assertions)]
        {
            let entities_to_relate = EntityHashSet::from_iter(entities_to_relate.iter().copied());
            let entities_to_unrelate =
                EntityHashSet::from_iter(entities_to_unrelate.iter().copied());
            let mut newly_related_entities =
                EntityHashSet::from_iter(newly_related_entities.iter().copied());
            assert!(
                entities_to_relate.is_disjoint(&entities_to_unrelate),
                "`entities_to_relate` ({entities_to_relate:?}) shared entities with `entities_to_unrelate` ({entities_to_unrelate:?})"
            );
            assert!(
                newly_related_entities.is_disjoint(&entities_to_unrelate),
                "`newly_related_entities` ({newly_related_entities:?}) shared entities with `entities_to_unrelate ({entities_to_unrelate:?})`"
            );
            assert!(
                newly_related_entities.is_subset(&entities_to_relate),
                "`newly_related_entities` ({newly_related_entities:?}) wasn't a subset of `entities_to_relate` ({entities_to_relate:?})"
            );

            if let Some(target) = self.get::<R::RelationshipTarget>() {
                let existing_relationships: EntityHashSet = target.collection().iter().collect();

                assert!(
                    existing_relationships.is_disjoint(&newly_related_entities),
                    "`newly_related_entities` contains an entity that wouldn't be newly related"
                );

                newly_related_entities.extend(existing_relationships);
                newly_related_entities -= &entities_to_unrelate;
            }

            assert_eq!(newly_related_entities, entities_to_relate, "`entities_to_relate` ({entities_to_relate:?}) didn't contain all entities that would end up related");
        };

        if !self.contains::<R::RelationshipTarget>() {
            self.add_related::<R>(entities_to_relate);

            return self;
        };

        let this = self.id();
        self.world_scope(|world| {
            for unrelate in entities_to_unrelate {
                world.entity_mut(*unrelate).remove::<R>();
            }

            for new_relation in newly_related_entities {
                // We're changing the target collection manually so don't run the insert hook
                world
                    .entity_mut(*new_relation)
                    .insert_with_relationship_hook_mode(R::from(this), RelationshipHookMode::Skip);
            }
        });

        if !entities_to_relate.is_empty() {
            if let Some(mut target) = self.get_mut::<R::RelationshipTarget>() {
                // SAFETY: The invariants expected by this function mean we'll only be inserting entities that are already related.
                let collection = target.collection_mut_risky();
                collection.clear();

                collection.extend_from_iter(entities_to_relate.iter().copied());
            } else {
                let mut empty =
                    <R::RelationshipTarget as RelationshipTarget>::Collection::with_capacity(
                        entities_to_relate.len(),
                    );
                empty.extend_from_iter(entities_to_relate.iter().copied());

                // SAFETY: We've just initialized this collection and we know there's no `RelationshipTarget` on `self`
                self.insert(R::RelationshipTarget::from_collection_risky(empty));
            }
        }

        self
    }

    /// Relates the given entity to this with the relation `R`.
    ///
    /// See [`add_related`](Self::add_related) if you want to relate more than one entity.
    pub fn add_one_related<R: Relationship>(&mut self, entity: Entity) -> &mut Self {
        self.add_related::<R>(&[entity])
    }

    /// Despawns entities that relate to this one via the given [`RelationshipTarget`].
    /// This entity will not be despawned.
    pub fn despawn_related<S: RelationshipTarget>(&mut self) -> &mut Self {
        if let Some(sources) = self.take::<S>() {
            self.world_scope(|world| {
                for entity in sources.iter() {
                    if let Ok(entity_mut) = world.get_entity_mut(entity) {
                        entity_mut.despawn();
                    }
                }
            });
        }
        self
    }

    /// Inserts a component or bundle of components into the entity and all related entities,
    /// traversing the relationship tracked in `S` in a breadth-first manner.
    ///
    /// # Warning
    ///
    /// This method should only be called on relationships that form a tree-like structure.
    /// Any cycles will cause this method to loop infinitely.
    // We could keep track of a list of visited entities and track cycles,
    // but this is not a very well-defined operation (or hard to write) for arbitrary relationships.
    pub fn insert_recursive<S: RelationshipTarget>(
        &mut self,
        bundle: impl Bundle + Clone,
    ) -> &mut Self {
        self.insert(bundle.clone());
        if let Some(relationship_target) = self.get::<S>() {
            let related_vec: Vec<Entity> = relationship_target.iter().collect();
            for related in related_vec {
                self.world_scope(|world| {
                    world
                        .entity_mut(related)
                        .insert_recursive::<S>(bundle.clone());
                });
            }
        }

        self
    }

    /// Removes a component or bundle of components of type `B` from the entity and all related entities,
    /// traversing the relationship tracked in `S` in a breadth-first manner.
    ///
    /// # Warning
    ///
    /// This method should only be called on relationships that form a tree-like structure.
    /// Any cycles will cause this method to loop infinitely.
    pub fn remove_recursive<S: RelationshipTarget, B: Bundle>(&mut self) -> &mut Self {
        self.remove::<B>();
        if let Some(relationship_target) = self.get::<S>() {
            let related_vec: Vec<Entity> = relationship_target.iter().collect();
            for related in related_vec {
                self.world_scope(|world| {
                    world.entity_mut(related).remove_recursive::<S, B>();
                });
            }
        }

        self
    }
}

impl<'a> EntityCommands<'a> {
    /// Spawns a entity related to this entity (with the `R` relationship) by taking a bundle
    pub fn with_related<R: Relationship>(&mut self, bundle: impl Bundle) -> &mut Self {
        let parent = self.id();
        self.commands.spawn((bundle, R::from(parent)));
        self
    }

    /// Spawns entities related to this entity (with the `R` relationship) by taking a function that operates on a [`RelatedSpawner`].
    pub fn with_related_entities<R: Relationship>(
        &mut self,
        func: impl FnOnce(&mut RelatedSpawnerCommands<R>),
    ) -> &mut Self {
        let id = self.id();
        func(&mut RelatedSpawnerCommands::new(self.commands(), id));
        self
    }

    /// Relates the given entities to this entity with the relation `R`.
    ///
    /// See [`add_one_related`](Self::add_one_related) if you want relate only one entity.
    pub fn add_related<R: Relationship>(&mut self, related: &[Entity]) -> &mut Self {
        let related: Box<[Entity]> = related.into();

        self.queue(move |mut entity: EntityWorldMut| {
            entity.add_related::<R>(&related);
        })
    }

    /// Relates the given entities to this entity with the relation `R`, starting at this particular index.
    ///
    /// If the `related` has duplicates, a related entity will take the index of its last occurrence in `related`.
    /// If the indices go out of bounds, they will be clamped into bounds.
    /// This will not re-order existing related entities unless they are in `related`.
    pub fn insert_related<R: Relationship>(&mut self, index: usize, related: &[Entity]) -> &mut Self
    where
        <R::RelationshipTarget as RelationshipTarget>::Collection:
            OrderedRelationshipSourceCollection,
    {
        let related: Box<[Entity]> = related.into();

        self.queue(move |mut entity: EntityWorldMut| {
            entity.insert_related::<R>(index, &related);
        })
    }

    /// Relates the given entity to this with the relation `R`.
    ///
    /// See [`add_related`](Self::add_related) if you want to relate more than one entity.
    pub fn add_one_related<R: Relationship>(&mut self, entity: Entity) -> &mut Self {
        self.add_related::<R>(&[entity])
    }

    /// Removes the relation `R` between this entity and the given entities.
    pub fn remove_related<R: Relationship>(&mut self, related: &[Entity]) -> &mut Self {
        let related: Box<[Entity]> = related.into();

        self.queue(move |mut entity: EntityWorldMut| {
            entity.remove_related::<R>(&related);
        })
    }

    /// Replaces all the related entities with the given set of new related entities.
    pub fn replace_related<R: Relationship>(&mut self, related: &[Entity]) -> &mut Self {
        let related: Box<[Entity]> = related.into();

        self.queue(move |mut entity: EntityWorldMut| {
            entity.replace_related::<R>(&related);
        })
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
    /// Panics when debug assertions are enable, an invariant is are broken and the command is executed.
    pub fn replace_related_with_difference<R: Relationship>(
        &mut self,
        entities_to_unrelate: &[Entity],
        entities_to_relate: &[Entity],
        newly_related_entities: &[Entity],
    ) -> &mut Self {
        let entities_to_unrelate: Box<[Entity]> = entities_to_unrelate.into();
        let entities_to_relate: Box<[Entity]> = entities_to_relate.into();
        let newly_related_entities: Box<[Entity]> = newly_related_entities.into();

        self.queue(move |mut entity: EntityWorldMut| {
            entity.replace_related_with_difference::<R>(
                &entities_to_unrelate,
                &entities_to_relate,
                &newly_related_entities,
            );
        })
    }

    /// Despawns entities that relate to this one via the given [`RelationshipTarget`].
    /// This entity will not be despawned.
    pub fn despawn_related<S: RelationshipTarget>(&mut self) -> &mut Self {
        self.queue(move |mut entity: EntityWorldMut| {
            entity.despawn_related::<S>();
        })
    }

    /// Inserts a component or bundle of components into the entity and all related entities,
    /// traversing the relationship tracked in `S` in a breadth-first manner.
    ///
    /// # Warning
    ///
    /// This method should only be called on relationships that form a tree-like structure.
    /// Any cycles will cause this method to loop infinitely.
    pub fn insert_recursive<S: RelationshipTarget>(
        &mut self,
        bundle: impl Bundle + Clone,
    ) -> &mut Self {
        self.queue(move |mut entity: EntityWorldMut| {
            entity.insert_recursive::<S>(bundle);
        })
    }

    /// Removes a component or bundle of components of type `B` from the entity and all related entities,
    /// traversing the relationship tracked in `S` in a breadth-first manner.
    ///
    /// # Warning
    ///
    /// This method should only be called on relationships that form a tree-like structure.
    /// Any cycles will cause this method to loop infinitely.
    pub fn remove_recursive<S: RelationshipTarget, B: Bundle>(&mut self) -> &mut Self {
        self.queue(move |mut entity: EntityWorldMut| {
            entity.remove_recursive::<S, B>();
        })
    }
}

/// Directly spawns related "source" entities with the given [`Relationship`], targeting
/// a specific entity.
pub struct RelatedSpawner<'w, R: Relationship> {
    target: Entity,
    world: &'w mut World,
    _marker: PhantomData<R>,
}

impl<'w, R: Relationship> RelatedSpawner<'w, R> {
    /// Creates a new instance that will spawn entities targeting the `target` entity.
    pub fn new(world: &'w mut World, target: Entity) -> Self {
        Self {
            world,
            target,
            _marker: PhantomData,
        }
    }

    /// Spawns an entity with the given `bundle` and an `R` relationship targeting the `target`
    /// entity this spawner was initialized with.
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityWorldMut<'_> {
        self.world.spawn((R::from(self.target), bundle))
    }

    /// Spawns an entity with an `R` relationship targeting the `target`
    /// entity this spawner was initialized with.
    pub fn spawn_empty(&mut self) -> EntityWorldMut<'_> {
        self.world.spawn(R::from(self.target))
    }

    /// Returns the "target entity" used when spawning entities with an `R` [`Relationship`].
    pub fn target_entity(&self) -> Entity {
        self.target
    }
}

/// Uses commands to spawn related "source" entities with the given [`Relationship`], targeting
/// a specific entity.
pub struct RelatedSpawnerCommands<'w, R: Relationship> {
    target: Entity,
    commands: Commands<'w, 'w>,
    _marker: PhantomData<R>,
}

impl<'w, R: Relationship> RelatedSpawnerCommands<'w, R> {
    /// Creates a new instance that will spawn entities targeting the `target` entity.
    pub fn new(commands: Commands<'w, 'w>, target: Entity) -> Self {
        Self {
            commands,
            target,
            _marker: PhantomData,
        }
    }

    /// Spawns an entity with the given `bundle` and an `R` relationship targeting the `target`
    /// entity this spawner was initialized with.
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityCommands<'_> {
        self.commands.spawn((R::from(self.target), bundle))
    }

    /// Spawns an entity with an `R` relationship targeting the `target`
    /// entity this spawner was initialized with.
    pub fn spawn_empty(&mut self) -> EntityCommands<'_> {
        self.commands.spawn(R::from(self.target))
    }

    /// Returns the "target entity" used when spawning entities with an `R` [`Relationship`].
    pub fn target_entity(&self) -> Entity {
        self.target
    }

    /// Returns the underlying [`Commands`].
    pub fn commands(&mut self) -> Commands {
        self.commands.reborrow()
    }

    /// Returns a mutable reference to the underlying [`Commands`].
    pub fn commands_mut(&mut self) -> &mut Commands<'w, 'w> {
        &mut self.commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::{ChildOf, Children, Component};

    #[derive(Component, Clone, Copy)]
    struct TestComponent;

    #[test]
    fn insert_and_remove_recursive() {
        let mut world = World::new();

        let a = world.spawn_empty().id();
        let b = world.spawn(ChildOf(a)).id();
        let c = world.spawn(ChildOf(a)).id();
        let d = world.spawn(ChildOf(b)).id();

        world
            .entity_mut(a)
            .insert_recursive::<Children>(TestComponent);

        for entity in [a, b, c, d] {
            assert!(world.entity(entity).contains::<TestComponent>());
        }

        world
            .entity_mut(b)
            .remove_recursive::<Children, TestComponent>();

        // Parent
        assert!(world.entity(a).contains::<TestComponent>());
        // Target
        assert!(!world.entity(b).contains::<TestComponent>());
        // Sibling
        assert!(world.entity(c).contains::<TestComponent>());
        // Child
        assert!(!world.entity(d).contains::<TestComponent>());

        world
            .entity_mut(a)
            .remove_recursive::<Children, TestComponent>();

        for entity in [a, b, c, d] {
            assert!(!world.entity(entity).contains::<TestComponent>());
        }
    }
}
