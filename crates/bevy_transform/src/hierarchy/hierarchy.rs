use crate::components::Children;
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

pub struct DespawnRecursive {
    entity: Entity,
}

fn despawn_with_children_recursive(world: &mut World, entity: Entity) {
    if let Some(children) = world
        .get::<Children>(entity)
        .ok()
        .map(|children| children.0.iter().cloned().collect::<Vec<Entity>>())
    {
        for e in children {
            despawn_with_children_recursive(world, e);
        }
    }

    world.despawn(entity).unwrap();
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
    use crate::hierarchy::BuildChildren;
    use bevy_ecs::{Commands, Entity, Resources, World};

    #[test]
    fn despawn_recursive() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let mut command_buffer = Commands::default();
        let parent_entity = Entity::new();

        command_buffer.spawn((0u32, 0u64)).with_children(|parent| {
            parent.spawn((0u32, 0u64));
        });

        command_buffer
            .spawn_as_entity(parent_entity, (1u32, 2u64))
            .with_children(|parent| {
                parent.spawn((1u32, 2u64)).with_children(|parent| {
                    parent.spawn((1u32, 2u64));
                });
                parent.spawn((1u32, 2u64));
            });

        command_buffer.spawn((0u32, 0u64));
        command_buffer.apply(&mut world, &mut resources);

        command_buffer.despawn_recursive(parent_entity);
        command_buffer.apply(&mut world, &mut resources);

        let results = world
            .query::<(&u32, &u64)>()
            .iter()
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<_>>();

        // parent_entity and its children should be deleted,
        // the (0, 0) tuples remaining.
        assert_eq!(results, vec![(0u32, 0u64), (0u32, 0u64), (0u32, 0u64)]);
    }
}
