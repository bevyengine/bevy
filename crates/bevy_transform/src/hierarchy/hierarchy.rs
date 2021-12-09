use crate::components::{Children, Parent};
use bevy_ecs::{
    entity::Entity,
    system::{Command, EntityCommands},
    world::{EntityMut, World},
};
use bevy_utils::tracing::debug;

#[derive(Debug)]
pub struct DespawnRecursive {
    entity: Entity,
}

#[derive(Debug)]
pub struct DespawnChildrenRecursive {
    entity: Entity,
}

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

fn despawn_children(world: &mut World, entity: Entity) {
    if let Some(mut children) = world.get_mut::<Children>(entity) {
        for e in std::mem::take(&mut children.0) {
            despawn_with_children_recursive_inner(world, e);
        }
    }
}

impl Command for DespawnRecursive {
    fn write(self, world: &mut World) {
        despawn_with_children_recursive(world, self.entity);
    }
}

impl Command for DespawnChildrenRecursive {
    fn write(self, world: &mut World) {
        despawn_children(world, self.entity);
    }
}

pub trait DespawnRecursiveExt {
    /// Despawns the provided entity alongside all descendants.
    fn despawn_recursive(self);

    /// Despawns all descendants of the given entity.
    fn despawn_descendants(&mut self);
}

impl<'w, 's, 'a> DespawnRecursiveExt for EntityCommands<'w, 's, 'a> {
    /// Despawns the provided entity and its children.
    fn despawn_recursive(mut self) {
        let entity = self.id();
        self.commands().add(DespawnRecursive { entity });
    }

    fn despawn_descendants(&mut self) {
        let entity = self.id();
        self.commands().add(DespawnChildrenRecursive { entity });
    }
}

impl<'w> DespawnRecursiveExt for EntityMut<'w> {
    /// Despawns the provided entity and its children.
    fn despawn_recursive(mut self) {
        let entity = self.id();
        // SAFE: EntityMut is consumed so even though the location is no longer
        // valid, it cannot be accessed again with the invalid location.
        unsafe {
            despawn_with_children_recursive(self.world_mut(), entity);
        }
    }

    fn despawn_descendants(&mut self) {
        let entity = self.id();
        // SAFE: The location is updated.
        unsafe {
            despawn_children(self.world_mut(), entity);
            self.update_location();
        }
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
    use crate::{components::Children, hierarchy::BuildChildren};

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
                .spawn_bundle((N("Another parent".to_owned()), Idx(0)))
                .with_children(|parent| {
                    parent.spawn_bundle((N("Another child".to_owned()), Idx(1)));
                });

            // Create a grandparent entity which will _not_ be deleted
            grandparent_entity = commands
                .spawn_bundle((N("Grandparent".to_owned()), Idx(2)))
                .id();
            commands.entity(grandparent_entity).with_children(|parent| {
                // Add a child to the grandparent (the "parent"), which will get deleted
                parent
                    .spawn_bundle((N("Parent, to be deleted".to_owned()), Idx(3)))
                    // All descendents of the "parent" should also be deleted.
                    .with_children(|parent| {
                        parent
                            .spawn_bundle((N("First Child, to be deleted".to_owned()), Idx(4)))
                            .with_children(|parent| {
                                // child
                                parent.spawn_bundle((
                                    N("First grand child, to be deleted".to_owned()),
                                    Idx(5),
                                ));
                            });
                        parent.spawn_bundle((N("Second child, to be deleted".to_owned()), Idx(6)));
                    });
            });

            commands.spawn_bundle((N("An innocent bystander".to_owned()), Idx(7)));
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
}
