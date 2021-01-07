use crate::components::*;
use bevy_ecs::{Commands, Entity, Query, Without};
use bevy_utils::HashMap;
use smallvec::SmallVec;

pub fn parent_update_system(
    commands: &mut Commands,
    removed_parent_query: Query<(Entity, &PreviousParent), Without<Parent>>,
    // The next query could be run with a Changed<Parent> filter. However, this would mean that modifications later in the frame are lost.
    // See issue 891: https://github.com/bevyengine/bevy/issues/891
    mut parent_query: Query<(Entity, &Parent, Option<&mut PreviousParent>)>,
    mut children_query: Query<&mut Children>,
) {
    // Entities with a missing `Parent` (ie. ones that have a `PreviousParent`), remove
    // them from the `Children` of the `PreviousParent`.
    for (entity, previous_parent) in removed_parent_query.iter() {
        if let Ok(mut previous_parent_children) = children_query.get_mut(previous_parent.0) {
            previous_parent_children.0.retain(|e| *e != entity);
            commands.remove_one::<PreviousParent>(entity);
        }
    }

    // Tracks all newly created `Children` Components this frame.
    let mut children_additions = HashMap::<Entity, SmallVec<[Entity; 8]>>::default();

    // Entities with a changed Parent (that also have a PreviousParent, even if None)
    for (entity, parent, possible_previous_parent) in parent_query.iter_mut() {
        if let Some(mut previous_parent) = possible_previous_parent {
            // New and previous point to the same Entity, carry on, nothing to see here.
            if previous_parent.0 == parent.0 {
                continue;
            }

            // Remove from `PreviousParent.Children`.
            if let Ok(mut previous_parent_children) = children_query.get_mut(previous_parent.0) {
                (*previous_parent_children).0.retain(|e| *e != entity);
            }

            // Set `PreviousParent = Parent`.
            *previous_parent = PreviousParent(parent.0);
        } else {
            commands.insert_one(entity, PreviousParent(parent.0));
        };

        // Add to the parent's `Children` (either the real component, or
        // `children_additions`).
        if let Ok(mut new_parent_children) = children_query.get_mut(parent.0) {
            // This is the parent
            debug_assert!(
                !(*new_parent_children).0.contains(&entity),
                "children already added"
            );
            (*new_parent_children).0.push(entity);
        } else {
            // The parent doesn't have a children entity, lets add it
            children_additions
                .entry(parent.0)
                .or_insert_with(Default::default)
                .push(entity);
        }
    }

    // Flush the `children_additions` to the command buffer. It is stored separate to
    // collect multiple new children that point to the same parent into the same
    // SmallVec, and to prevent redundant add+remove operations.
    children_additions.iter().for_each(|(k, v)| {
        commands.insert_one(*k, Children::with(v));
    });
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::{hierarchy::BuildChildren, transform_propagate_system::transform_propagate_system};
    use bevy_ecs::{IntoSystem, Resources, Schedule, SystemStage, World};

    #[test]
    fn correct_children() {
        let mut world = World::default();
        let mut resources = Resources::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(parent_update_system.system());
        update_stage.add_system(transform_propagate_system.system());

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        // Add parent entities
        let mut commands = Commands::default();
        commands.set_entity_reserver(world.get_entity_reserver());
        let mut parent = None;
        let mut children = Vec::new();
        commands
            .spawn((Transform::from_xyz(1.0, 0.0, 0.0),))
            .for_current_entity(|entity| parent = Some(entity))
            .with_children(|parent| {
                parent
                    .spawn((Transform::from_xyz(0.0, 2.0, 0.0),))
                    .for_current_entity(|entity| children.push(entity))
                    .spawn((Transform::from_xyz(0.0, 0.0, 3.0),))
                    .for_current_entity(|entity| children.push(entity));
            });
        let parent = parent.unwrap();
        commands.apply(&mut world, &mut resources);
        schedule.initialize_and_run(&mut world, &mut resources);

        assert_eq!(
            world
                .get::<Children>(parent)
                .unwrap()
                .0
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            children,
        );

        // Parent `e1` to `e2`.
        (*world.get_mut::<Parent>(children[0]).unwrap()).0 = children[1];

        schedule.initialize_and_run(&mut world, &mut resources);

        assert_eq!(
            world
                .get::<Children>(parent)
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![children[1]]
        );

        assert_eq!(
            world
                .get::<Children>(children[1])
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![children[0]]
        );

        world.despawn(children[0]).unwrap();

        schedule.initialize_and_run(&mut world, &mut resources);

        assert_eq!(
            world
                .get::<Children>(parent)
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![children[1]]
        );
    }
}
