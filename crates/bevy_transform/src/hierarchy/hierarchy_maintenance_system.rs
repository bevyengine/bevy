use crate::components::*;
use bevy_ecs::{
    entity::Entity,
    query::Without,
    system::{Commands, Query},
};

pub fn parent_update_system(
    mut commands: Commands,
    removed_parent_query: Query<(Entity, &PreviousParent), Without<Parent>>,
    // The next query could be run with a Changed<Parent> filter. However, this would mean that
    // modifications later in the frame are lost. See issue 891: https://github.com/bevyengine/bevy/issues/891
    mut parent_query: Query<(Entity, &Parent, Option<&mut PreviousParent>)>,
    mut children_query: Query<&mut Children>,
) {
    // Entities with a missing `Parent` (ie. ones that have a `PreviousParent`), remove
    // them from the `Children` of the `PreviousParent`.
    for (entity, previous_parent) in removed_parent_query.iter() {
        if let Ok(mut previous_parent_children) = children_query.get_mut(previous_parent.0) {
            previous_parent_children.0.retain(|e| *e != entity);
            commands.remove::<PreviousParent>(entity);
        }
    }

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
            commands.insert(entity, PreviousParent(parent.0));
        };
    }
}
#[cfg(test)]
mod test {
    use bevy_ecs::{
        schedule::{Schedule, Stage, SystemStage},
        system::{CommandQueue, IntoSystem},
        world::World,
    };

    use super::*;
    use crate::{hierarchy::BuildChildren, transform_propagate_system::transform_propagate_system};

    #[test]
    fn correct_children() {
        let mut world = World::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(parent_update_system.system());
        update_stage.add_system(transform_propagate_system.system());

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        // Add parent entities
        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, &world);
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
        command_queue.apply(&mut world);
        schedule.run(&mut world);

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
        let mut commands = Commands::new(&mut command_queue, &world);
        commands.push_children(children[1], &[children[0]]);

        command_queue.apply(&mut world);
        schedule.run(&mut world);

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

        assert!(world.despawn(children[0]));

        schedule.run(&mut world);

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
