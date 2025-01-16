use crate::{
    bundle::Bundle,
    entity::Entity,
    relationship::{Relationship, RelationshipSources},
    system::{Commands, EntityCommands},
    world::{EntityWorldMut, World},
};
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

    /// Despawns entities that relate to this one via the given [`RelationshipSources`].
    /// This entity will not be despawned.
    pub fn despawn_related<S: RelationshipSources>(&mut self) -> &mut Self {
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

    /// Despawns entities that relate to this one via the given [`RelationshipSources`].
    /// This entity will not be despawned.
    pub fn despawn_related<S: RelationshipSources>(&mut self) -> &mut Self {
        let id = self.id();
        self.commands.queue(move |world: &mut World| {
            world.entity_mut(id).despawn_related::<S>();
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
