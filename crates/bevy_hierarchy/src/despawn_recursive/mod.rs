mod command;
pub use command::DespawnRecursive;

mod ext;
pub use ext::DespawnRecursiveExt;

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component,
        system::Commands,
        world::{CommandQueue, World},
    };

    use super::DespawnRecursiveExt;
    use crate::{
        child_builder::{BuildChildren, ChildBuild},
        Children,
    };

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
            let children = world.get::<Children>(grandparent_entity);
            assert!(
                children.is_none(),
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
        assert_eq!(children.unwrap().len(), 2_usize);
        // The original child should be despawned.
        assert!(world.get_entity(child).is_none());
    }
}
