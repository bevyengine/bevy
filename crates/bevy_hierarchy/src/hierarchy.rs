use crate::components::{Children, Parent};
use bevy_ecs::{
    entity::Entity,
    system::{Command, EntityCommands},
    world::{EntityWorldMut, World},
};
use bevy_utils::tracing::debug;

/// Despawns the given entity and all its children recursively
#[derive(Debug)]
pub struct DespawnRecursive {
    /// Target entity
    pub entity: Entity,
}

/// Despawns the given entity's children recursively
#[derive(Debug)]
pub struct DespawnChildrenRecursive {
    /// Target entity
    pub entity: Entity,
}

/// Function for despawning an entity and all its children
pub fn despawn_with_children_recursive(world: &mut World, entity: Entity) {
    // first, make the entity's own parent forget about it
    if let Some(parent) = world.get::<Parent>(entity).map(|parent| parent.0) {
        if let Some(mut children) = world.get_mut::<Children>(parent) {
            children.0.retain(|c| *c != entity);
        }
    }

    // then despawn the entity and all of its children
    despawn_with_children_recursive_inner(world, entity);
}

// Should only be called by `despawn_with_children_recursive`!
fn despawn_with_children_recursive_inner(world: &mut World, entity: Entity) {
    if let Some(mut children) = world.get_mut::<Children>(entity) {
        for e in std::mem::take(&mut children.0) {
            despawn_with_children_recursive_inner(world, e);
        }
    }

    if !world.despawn(entity) {
        debug!("Failed to despawn entity {:?}", entity);
    }
}

fn despawn_children_recursive(world: &mut World, entity: Entity) {
    if let Some(children) = world.entity_mut(entity).take::<Children>() {
        for e in children.0 {
            despawn_with_children_recursive_inner(world, e);
        }
    }
}

impl Command for DespawnRecursive {
    fn apply(self, world: &mut World) {
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!(
            "command",
            name = "DespawnRecursive",
            entity = bevy_utils::tracing::field::debug(self.entity)
        )
        .entered();
        despawn_with_children_recursive(world, self.entity);
    }
}

impl Command for DespawnChildrenRecursive {
    fn apply(self, world: &mut World) {
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!(
            "command",
            name = "DespawnChildrenRecursive",
            entity = bevy_utils::tracing::field::debug(self.entity)
        )
        .entered();
        despawn_children_recursive(world, self.entity);
    }
}

/// Trait that holds functions for despawning recursively down the transform hierarchy
pub trait DespawnRecursiveExt {
    /// Despawns the provided entity alongside all descendants.
    fn despawn_recursive(self);

    /// Despawns all descendants of the given entity.
    fn despawn_descendants(&mut self) -> &mut Self;
}

impl<'w, 's, 'a> DespawnRecursiveExt for EntityCommands<'w, 's, 'a> {
    /// Despawns the provided entity and its children.
    fn despawn_recursive(mut self) {
        let entity = self.id();
        self.commands().add(DespawnRecursive { entity });
    }

    fn despawn_descendants(&mut self) -> &mut Self {
        let entity = self.id();
        self.commands().add(DespawnChildrenRecursive { entity });
        self
    }
}

impl<'w> DespawnRecursiveExt for EntityWorldMut<'w> {
    /// Despawns the provided entity and its children.
    fn despawn_recursive(self) {
        let entity = self.id();

        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!(
            "despawn_recursive",
            entity = bevy_utils::tracing::field::debug(entity)
        )
        .entered();

        despawn_with_children_recursive(self.into_world_mut(), entity);
    }

    fn despawn_descendants(&mut self) -> &mut Self {
        let entity = self.id();

        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!(
            "despawn_descendants",
            entity = bevy_utils::tracing::field::debug(entity)
        )
        .entered();

        self.world_scope(|world| {
            despawn_children_recursive(world, entity);
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component,
        system::{CommandQueue, Commands},
        world::World,
    };

    use super::DespawnRecursiveExt;
    use crate::{child_builder::BuildChildren, components::Children};

    #[derive(Component, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Debug)]
    struct Idx(u32);

    #[derive(Component, Clone, PartialEq, Eq, Ord, PartialOrd, Debug)]
    struct N(String);

    #[test]
    fn despawn_recursive() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let grandparent_entity;
        {
            let mut commands = Commands::new(&mut queue, &world);

            commands
                .spawn((N("Another parent".to_owned()), Idx(0)))
                .with_children(|parent| {
                    parent.spawn((N("Another child".to_owned()), Idx(1)));
                });

            // Create a grandparent entity which will _not_ be deleted
            grandparent_entity = commands.spawn((N("Grandparent".to_owned()), Idx(2))).id();
            commands.entity(grandparent_entity).with_children(|parent| {
                // Add a child to the grandparent (the "parent"), which will get deleted
                parent
                    .spawn((N("Parent, to be deleted".to_owned()), Idx(3)))
                    // All descendants of the "parent" should also be deleted.
                    .with_children(|parent| {
                        parent
                            .spawn((N("First Child, to be deleted".to_owned()), Idx(4)))
                            .with_children(|parent| {
                                // child
                                parent.spawn((
                                    N("First grand child, to be deleted".to_owned()),
                                    Idx(5),
                                ));
                            });
                        parent.spawn((N("Second child, to be deleted".to_owned()), Idx(6)));
                    });
            });

            commands.spawn((N("An innocent bystander".to_owned()), Idx(7)));
        }
        queue.apply(&mut world);

        let parent_entity = world.get::<Children>(grandparent_entity).unwrap()[0];

        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent_entity).despawn_recursive();
            // despawning the same entity twice should not panic
            commands.entity(parent_entity).despawn_recursive();
        }
        queue.apply(&mut world);

        let mut results = world
            .query::<(&N, &Idx)>()
            .iter(&world)
            .map(|(a, b)| (a.clone(), *b))
            .collect::<Vec<_>>();
        results.sort_unstable_by_key(|(_, index)| *index);

        {
            let children = world.get::<Children>(grandparent_entity).unwrap();
            assert!(
                !children.iter().any(|&i| i == parent_entity),
                "grandparent should no longer know about its child which has been removed"
            );
        }

        assert_eq!(
            results,
            vec![
                (N("Another parent".to_owned()), Idx(0)),
                (N("Another child".to_owned()), Idx(1)),
                (N("Grandparent".to_owned()), Idx(2)),
                (N("An innocent bystander".to_owned()), Idx(7))
            ]
        );
    }

    #[test]
    fn despawn_descendants() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let parent = commands.spawn_empty().id();
        let child = commands.spawn_empty().id();

        commands
            .entity(parent)
            .add_child(child)
            .despawn_descendants();

        queue.apply(&mut world);

        // The parent's Children component should be removed.
        assert!(world.entity(parent).get::<Children>().is_none());
        // The child should be despawned.
        assert!(world.get_entity(child).is_none());
    }

    #[test]
    fn spawn_children_after_despawn_descendants() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let parent = commands.spawn_empty().id();
        let child = commands.spawn_empty().id();

        commands
            .entity(parent)
            .add_child(child)
            .despawn_descendants()
            .with_children(|parent| {
                parent.spawn_empty();
                parent.spawn_empty();
            });

        queue.apply(&mut world);

        // The parent's Children component should still have two children.
        let children = world.entity(parent).get::<Children>();
        assert!(children.is_some());
        assert!(children.unwrap().len() == 2_usize);
        // The original child should be despawned.
        assert!(world.get_entity(child).is_none());
    }
}
