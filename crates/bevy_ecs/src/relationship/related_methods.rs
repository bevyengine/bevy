use crate::{
    bundle::Bundle,
    entity::Entity,
    relationship::{Relationship, RelationshipTarget},
    system::{Commands, EntityCommands},
    world::{EntityWorldMut, World},
};
use alloc::vec::Vec;
use core::marker::PhantomData;

impl<'w> EntityWorldMut<'w> {
    /// Spawns entities related to this entity (with the `R` relationship) by taking a function that operates on a [`RelatedSpawner`].
    pub fn with_related<R: Relationship>(
        &mut self,
        func: impl FnOnce(&mut RelatedSpawner<R>),
    ) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            func(&mut RelatedSpawner::new(world, parent));
        });
        self
    }

    /// Relates the given entities to this entity with the relation `R`
    pub fn add_related<R: Relationship>(&mut self, related: &[Entity]) -> &mut Self {
        let id = self.id();
        self.world_scope(|world| {
            for related in related {
                world.entity_mut(*related).insert(R::from(id));
            }
        });
        self
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
    /// Spawns entities related to this entity (with the `R` relationship) by taking a function that operates on a [`RelatedSpawner`].
    pub fn with_related<R: Relationship>(
        &mut self,
        func: impl FnOnce(&mut RelatedSpawnerCommands<R>),
    ) -> &mut Self {
        let id = self.id();
        func(&mut RelatedSpawnerCommands::new(self.commands(), id));
        self
    }

    /// Relates the given entities to this entity with the relation `R`
    pub fn add_related<R: Relationship>(&mut self, related: &[Entity]) -> &mut Self {
        let id = self.id();
        let related = related.to_vec();
        self.commands().queue(move |world: &mut World| {
            for related in related {
                world.entity_mut(related).insert(R::from(id));
            }
        });
        self
    }

    /// Despawns entities that relate to this one via the given [`RelationshipTarget`].
    /// This entity will not be despawned.
    pub fn despawn_related<S: RelationshipTarget>(&mut self) -> &mut Self {
        let id = self.id();
        self.commands.queue(move |world: &mut World| {
            world.entity_mut(id).despawn_related::<S>();
        });
        self
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
        let id = self.id();
        self.commands.queue(move |world: &mut World| {
            world.entity_mut(id).insert_recursive::<S>(bundle);
        });
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
        let id = self.id();
        self.commands.queue(move |world: &mut World| {
            world.entity_mut(id).remove_recursive::<S, B>();
        });
        self
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
