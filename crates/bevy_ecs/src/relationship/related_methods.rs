use crate::{
    bundle::Bundle,
    entity::{hash_set::EntityHashSet, Entity},
    prelude::Children,
    relationship::{
        Relationship, RelationshipHookMode, RelationshipSourceCollection, RelationshipTarget,
    },
    system::{Commands, EntityCommands},
    world::{DeferredWorld, EntityWorldMut, World},
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
                world
                    .entity_mut(*related)
                    .modify_or_insert_relation_with_relationship_hook_mode::<R>(
                        id,
                        RelationshipHookMode::Run,
                    );
            }
        });
        self
    }

    /// Removes the relation `R` between this entity and all its related entities.
    pub fn clear_related<R: Relationship>(&mut self) -> &mut Self {
        self.remove::<R::RelationshipTarget>()
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
                let index = index.saturating_add(offset);
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
                    world
                        .entity_mut(*related)
                        .modify_or_insert_relation_with_relationship_hook_mode::<R>(
                            id,
                            RelationshipHookMode::Run,
                        );
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

        let Some(existing_relations) = self.get_mut::<R::RelationshipTarget>() else {
            return self.add_related::<R>(related);
        };

        // We replace the component here with a dummy value so we can modify it without taking it (this would create archetype move).
        // SAFETY: We eventually return the correctly initialized collection into the target.
        let mut relations = mem::replace(
            existing_relations.into_inner(),
            <R as Relationship>::RelationshipTarget::from_collection_risky(
                Collection::<R>::with_capacity(0),
            ),
        );

        let collection = relations.collection_mut_risky();

        let mut potential_relations = EntityHashSet::from_iter(related.iter().copied());

        let id = self.id();
        self.world_scope(|world| {
            for related in collection.iter() {
                if !potential_relations.remove(related) {
                    world.entity_mut(related).remove::<R>();
                }
            }

            for related in potential_relations {
                // SAFETY: We'll manually be adjusting the contents of the `RelationshipTarget` to fit the final state.
                world
                    .entity_mut(related)
                    .modify_or_insert_relation_with_relationship_hook_mode::<R>(
                        id,
                        RelationshipHookMode::Skip,
                    );
            }
        });

        // SAFETY: The entities we're inserting will be the entities that were either already there or entities that we've just inserted.
        collection.clear();
        collection.extend_from_iter(related.iter().copied());
        self.insert(relations);

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

        match self.get_mut::<R::RelationshipTarget>() {
            None => {
                self.add_related::<R>(entities_to_relate);

                return self;
            }
            Some(mut target) => {
                // SAFETY: The invariants expected by this function mean we'll only be inserting entities that are already related.
                let collection = target.collection_mut_risky();
                collection.clear();

                collection.extend_from_iter(entities_to_relate.iter().copied());
            }
        }

        let this = self.id();
        self.world_scope(|world| {
            for unrelate in entities_to_unrelate {
                world.entity_mut(*unrelate).remove::<R>();
            }

            for new_relation in newly_related_entities {
                // We changed the target collection manually so don't run the insert hook
                world
                    .entity_mut(*new_relation)
                    .modify_or_insert_relation_with_relationship_hook_mode::<R>(
                        this,
                        RelationshipHookMode::Skip,
                    );
            }
        });

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
        if let Some(sources) = self.get::<S>() {
            // We have to collect here to defer removal, allowing observers and hooks to see this data
            // before it is finally removed.
            let sources = sources.iter().collect::<Vec<_>>();
            self.world_scope(|world| {
                for entity in sources {
                    if let Ok(entity_mut) = world.get_entity_mut(entity) {
                        entity_mut.despawn();
                    };
                }
            });
        }
        self
    }

    /// Despawns the children of this entity.
    /// This entity will not be despawned.
    ///
    /// This is a specialization of [`despawn_related`](EntityWorldMut::despawn_related), a more general method for despawning via relationships.
    pub fn despawn_children(&mut self) -> &mut Self {
        self.despawn_related::<Children>();
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

    fn modify_or_insert_relation_with_relationship_hook_mode<R: Relationship>(
        &mut self,
        entity: Entity,
        relationship_hook_mode: RelationshipHookMode,
    ) {
        // Check if the relation edge holds additional data
        if size_of::<R>() > size_of::<Entity>() {
            self.assert_not_despawned();

            let this = self.id();

            let modified = self.world_scope(|world| {
                let modified = DeferredWorld::from(&mut *world)
                    .modify_component_with_relationship_hook_mode::<R, _>(
                        this,
                        relationship_hook_mode,
                        |r| r.set_risky(entity),
                    )
                    .expect("entity access must be valid")
                    .is_some();

                world.flush();

                modified
            });

            if modified {
                return;
            }
        }

        self.insert_with_relationship_hook_mode(R::from(entity), relationship_hook_mode);
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

    /// Removes the relation `R` between this entity and all its related entities.
    pub fn clear_related<R: Relationship>(&mut self) -> &mut Self {
        self.queue(|mut entity: EntityWorldMut| {
            entity.clear_related::<R>();
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

    /// Despawns the children of this entity.
    /// This entity will not be despawned.
    ///
    /// This is a specialization of [`despawn_related`](EntityCommands::despawn_related), a more general method for despawning via relationships.
    pub fn despawn_children(&mut self) -> &mut Self {
        self.despawn_related::<Children>()
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

    /// Returns a reference to the underlying [`World`].
    pub fn world(&self) -> &World {
        self.world
    }

    /// Returns a mutable reference to the underlying [`World`].
    pub fn world_mut(&mut self) -> &mut World {
        self.world
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
    pub fn commands(&mut self) -> Commands<'_, '_> {
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

    #[test]
    fn remove_all_related() {
        let mut world = World::new();

        let a = world.spawn_empty().id();
        let b = world.spawn(ChildOf(a)).id();
        let c = world.spawn(ChildOf(a)).id();

        world.entity_mut(a).clear_related::<ChildOf>();

        assert_eq!(world.entity(a).get::<Children>(), None);
        assert_eq!(world.entity(b).get::<ChildOf>(), None);
        assert_eq!(world.entity(c).get::<ChildOf>(), None);
    }

    #[test]
    fn replace_related_works() {
        let mut world = World::new();
        let child1 = world.spawn_empty().id();
        let child2 = world.spawn_empty().id();
        let child3 = world.spawn_empty().id();

        let mut parent = world.spawn_empty();
        parent.add_children(&[child1, child2]);
        let child_value = ChildOf(parent.id());
        let some_child = Some(&child_value);

        parent.replace_children(&[child2, child3]);
        let children = parent.get::<Children>().unwrap().collection();
        assert_eq!(children, &[child2, child3]);
        assert_eq!(parent.world().get::<ChildOf>(child1), None);
        assert_eq!(parent.world().get::<ChildOf>(child2), some_child);
        assert_eq!(parent.world().get::<ChildOf>(child3), some_child);

        parent.replace_children_with_difference(&[child3], &[child1, child2], &[child1]);
        let children = parent.get::<Children>().unwrap().collection();
        assert_eq!(children, &[child1, child2]);
        assert_eq!(parent.world().get::<ChildOf>(child1), some_child);
        assert_eq!(parent.world().get::<ChildOf>(child2), some_child);
        assert_eq!(parent.world().get::<ChildOf>(child3), None);
    }

    #[test]
    fn add_related_keeps_relationship_data() {
        #[derive(Component, PartialEq, Debug)]
        #[relationship(relationship_target = Parent)]
        struct Child {
            #[relationship]
            parent: Entity,
            data: u8,
        }

        #[derive(Component)]
        #[relationship_target(relationship = Child)]
        struct Parent(Vec<Entity>);

        let mut world = World::new();
        let parent1 = world.spawn_empty().id();
        let parent2 = world.spawn_empty().id();
        let child = world
            .spawn(Child {
                parent: parent1,
                data: 42,
            })
            .id();

        world.entity_mut(parent2).add_related::<Child>(&[child]);
        assert_eq!(
            world.get::<Child>(child),
            Some(&Child {
                parent: parent2,
                data: 42
            })
        );
    }

    #[test]
    fn insert_related_keeps_relationship_data() {
        #[derive(Component, PartialEq, Debug)]
        #[relationship(relationship_target = Parent)]
        struct Child {
            #[relationship]
            parent: Entity,
            data: u8,
        }

        #[derive(Component)]
        #[relationship_target(relationship = Child)]
        struct Parent(Vec<Entity>);

        let mut world = World::new();
        let parent1 = world.spawn_empty().id();
        let parent2 = world.spawn_empty().id();
        let child = world
            .spawn(Child {
                parent: parent1,
                data: 42,
            })
            .id();

        world
            .entity_mut(parent2)
            .insert_related::<Child>(0, &[child]);
        assert_eq!(
            world.get::<Child>(child),
            Some(&Child {
                parent: parent2,
                data: 42
            })
        );
    }

    #[test]
    fn replace_related_keeps_relationship_data() {
        #[derive(Component, PartialEq, Debug)]
        #[relationship(relationship_target = Parent)]
        struct Child {
            #[relationship]
            parent: Entity,
            data: u8,
        }

        #[derive(Component)]
        #[relationship_target(relationship = Child)]
        struct Parent(Vec<Entity>);

        let mut world = World::new();
        let parent1 = world.spawn_empty().id();
        let parent2 = world.spawn_empty().id();
        let child = world
            .spawn(Child {
                parent: parent1,
                data: 42,
            })
            .id();

        world
            .entity_mut(parent2)
            .replace_related_with_difference::<Child>(&[], &[child], &[child]);
        assert_eq!(
            world.get::<Child>(child),
            Some(&Child {
                parent: parent2,
                data: 42
            })
        );

        world.entity_mut(parent1).replace_related::<Child>(&[child]);
        assert_eq!(
            world.get::<Child>(child),
            Some(&Child {
                parent: parent1,
                data: 42
            })
        );
    }

    #[test]
    fn replace_related_keeps_relationship_target_data() {
        #[derive(Component)]
        #[relationship(relationship_target = Parent)]
        struct Child(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Child)]
        struct Parent {
            #[relationship]
            children: Vec<Entity>,
            data: u8,
        }

        let mut world = World::new();
        let child1 = world.spawn_empty().id();
        let child2 = world.spawn_empty().id();
        let mut parent = world.spawn_empty();
        parent.add_related::<Child>(&[child1]);
        parent.get_mut::<Parent>().unwrap().data = 42;

        parent.replace_related_with_difference::<Child>(&[child1], &[child2], &[child2]);
        let data = parent.get::<Parent>().unwrap().data;
        assert_eq!(data, 42);

        parent.replace_related::<Child>(&[child1]);
        let data = parent.get::<Parent>().unwrap().data;
        assert_eq!(data, 42);
    }

    #[test]
    fn despawn_related_observers_can_access_relationship_data() {
        use crate::lifecycle::Replace;
        use crate::observer::On;
        use crate::prelude::Has;
        use crate::system::Query;

        #[derive(Component)]
        struct MyComponent;

        #[derive(Component, Default)]
        struct ObserverResult {
            success: bool,
        }

        let mut world = World::new();
        let result_entity = world.spawn(ObserverResult::default()).id();

        world.add_observer(
            move |event: On<Replace, MyComponent>,
                  has_relationship: Query<Has<ChildOf>>,
                  mut results: Query<&mut ObserverResult>| {
                let entity = event.entity();
                if has_relationship.get(entity).unwrap_or(false) {
                    results.get_mut(result_entity).unwrap().success = true;
                }
            },
        );

        let parent = world.spawn_empty().id();
        let _child = world.spawn((MyComponent, ChildOf(parent))).id();

        world.entity_mut(parent).despawn_related::<Children>();

        assert!(world.get::<ObserverResult>(result_entity).unwrap().success);
    }
}
