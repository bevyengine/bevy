use crate::components::{Children, Parent};
use bevy_ecs::{Commands, Entity, Query, World, WorldWriter};

pub fn run_on_hierarchy<T, S>(
    children_query: &Query<&Children>,
    state: &mut S,
    entity: Entity,
    parent_result: Option<T>,
    mut previous_result: Option<T>,
    run: &mut dyn FnMut(&mut S, Entity, Option<T>, Option<T>) -> Option<T>,
) -> Option<T>
where
    T: Clone,
{
    // TODO: not a huge fan of this pattern. are there ways to do recursive updates in legion without allocations?
    // TODO: the problem above might be resolvable with world splitting
    let children = children_query
        .get::<Children>(entity)
        .ok()
        .map(|children| children.0.iter().cloned().collect::<Vec<Entity>>());

    let parent_result = run(state, entity, parent_result, previous_result);
    previous_result = None;
    if let Some(children) = children {
        for child in children {
            previous_result = run_on_hierarchy(
                children_query,
                state,
                child,
                parent_result.clone(),
                previous_result,
                run,
            );
        }
    } else {
        previous_result = parent_result;
    }

    previous_result
}

#[derive(Debug)]
pub struct DespawnRecursive {
    entity: Entity,
}

fn despawn_with_children_recursive(world: &mut World, entity: Entity) {
    // first, make the entity's own parent forget about it
    if let Ok(parent) = world.get::<Parent>(entity).map(|parent| parent.0) {
        if let Ok(mut children) = world.get_mut::<Children>(parent) {
            children.retain(|c| *c != entity);
        }
    }
    // then despawn the entity and all of its children
    despawn_with_children_recursive_inner(world, entity);
}

// Should only be called by `despawn_with_children_recursive`!
fn despawn_with_children_recursive_inner(world: &mut World, entity: Entity) {
    if let Some(children) = world
        .get::<Children>(entity)
        .ok()
        .map(|children| children.0.iter().cloned().collect::<Vec<Entity>>())
    {
        for e in children {
            despawn_with_children_recursive(world, e);
        }
    }

    if let Err(e) = world.despawn(entity) {
        log::debug!("Failed to despawn entity {:?}: {}", entity, e);
    }
}

impl WorldWriter for DespawnRecursive {
    fn write(self: Box<Self>, world: &mut World) {
        despawn_with_children_recursive(world, self.entity);
    }
}

pub trait DespawnRecursiveExt {
    /// Despawns the provided entity and its children.
    fn despawn_recursive(&mut self, entity: Entity) -> &mut Self;
}

impl DespawnRecursiveExt for Commands {
    /// Despawns the provided entity and its children.
    fn despawn_recursive(&mut self, entity: Entity) -> &mut Self {
        self.write_world(DespawnRecursive { entity })
    }
}

#[cfg(test)]
mod tests {
    use super::DespawnRecursiveExt;
    use crate::{components::Children, hierarchy::BuildChildren};
    use bevy_ecs::{Commands, Resources, World};

    #[test]
    fn despawn_recursive() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let mut command_buffer = Commands::default();
        command_buffer.set_entity_reserver(world.get_entity_reserver());

        command_buffer.spawn((0u32, 0u64)).with_children(|parent| {
            parent.spawn((0u32, 0u64));
        });

        // Create a grandparent entity which will _not_ be deleted
        command_buffer.spawn((1u32, 1u64));
        let grandparent_entity = command_buffer.current_entity().unwrap();

        command_buffer.with_children(|parent| {
            // Add a child to the grandparent (the "parent"), which will get deleted
            parent.spawn((2u32, 2u64));
            // All descendents of the "parent" should also be deleted.
            parent.with_children(|parent| {
                parent.spawn((3u32, 3u64)).with_children(|parent| {
                    // child
                    parent.spawn((4u32, 4u64));
                });
                parent.spawn((5u32, 5u64));
            });
        });

        command_buffer.spawn((0u32, 0u64));
        command_buffer.apply(&mut world, &mut resources);

        let parent_entity = world.get::<Children>(grandparent_entity).unwrap()[0];

        command_buffer.despawn_recursive(parent_entity);
        command_buffer.despawn_recursive(parent_entity); // despawning the same entity twice should not panic
        command_buffer.apply(&mut world, &mut resources);

        let results = world
            .query::<(&u32, &u64)>()
            .iter()
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();

        {
            let children = world.get::<Children>(grandparent_entity).unwrap();
            assert_eq!(
                children.iter().any(|&i| i == parent_entity),
                false,
                "grandparent should no longer know about its child which has been removed"
            );
        }

        // parent_entity and its children should be deleted,
        // the grandparent tuple (1, 1) and (0, 0) tuples remaining.
        assert_eq!(
            results,
            vec![(0u32, 0u64), (0u32, 0u64), (0u32, 0u64), (1u32, 1u64)]
        );
    }
}
