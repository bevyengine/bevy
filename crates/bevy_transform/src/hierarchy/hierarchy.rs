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

impl Command for DespawnRecursive {
    fn write(self, world: &mut World) {
        despawn_with_children_recursive(world, self.entity);
    }
}

pub trait DespawnRecursiveExt {
    /// Despawns the provided entity and its children.
    fn despawn_recursive(self);
}

impl<'w, 's, 'a> DespawnRecursiveExt for EntityCommands<'w, 's, 'a> {
    /// Despawns the provided entity and its children.
    fn despawn_recursive(mut self) {
        let entity = self.id();
        self.commands().add(DespawnRecursive { entity });
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
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        system::{CommandQueue, Commands},
        world::World,
    };

    use super::DespawnRecursiveExt;
    use crate::{components::Children, hierarchy::BuildChildren};

    #[test]
    fn despawn_recursive() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let grandparent_entity;
        {
            let mut commands = Commands::new(&mut queue, &world);

            commands
                .spawn_bundle(("Another parent".to_owned(), 0u32))
                .with_children(|parent| {
                    parent.spawn_bundle(("Another child".to_owned(), 1u32));
                });

            // Create a grandparent entity which will _not_ be deleted
            grandparent_entity = commands.spawn_bundle(("Grandparent".to_owned(), 2u32)).id();
            commands.entity(grandparent_entity).with_children(|parent| {
                // Add a child to the grandparent (the "parent"), which will get deleted
                parent
                    .spawn_bundle(("Parent, to be deleted".to_owned(), 3u32))
                    // All descendents of the "parent" should also be deleted.
                    .with_children(|parent| {
                        parent
                            .spawn_bundle(("First Child, to be deleted".to_owned(), 4u32))
                            .with_children(|parent| {
                                // child
                                parent.spawn_bundle((
                                    "First grand child, to be deleted".to_owned(),
                                    5u32,
                                ));
                            });
                        parent.spawn_bundle(("Second child, to be deleted".to_owned(), 6u32));
                    });
            });

            commands.spawn_bundle(("An innocent bystander".to_owned(), 7u32));
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
            .query::<(&String, &u32)>()
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
                ("Another parent".to_owned(), 0u32),
                ("Another child".to_owned(), 1u32),
                ("Grandparent".to_owned(), 2u32),
                ("An innocent bystander".to_owned(), 7u32)
            ]
        );
    }
}
