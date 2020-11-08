use crate::components::*;
use bevy_ecs::{Commands, Entity, IntoSystem, Query, System, Without};
use bevy_utils::HashMap;
use smallvec::SmallVec;

pub fn parent_update_system(
    commands: &mut Commands,
    removed_parent_query: Query<Without<Parent, (Entity, &PreviousParent)>>,
    // TODO: ideally this only runs when the Parent component has changed
    mut changed_parent_query: Query<(Entity, &Parent, Option<&mut PreviousParent>)>,
    mut children_query: Query<&mut Children>,
) {
    // Entities with a missing `Parent` (ie. ones that have a `PreviousParent`), remove
    // them from the `Children` of the `PreviousParent`.
    for (entity, previous_parent) in removed_parent_query.iter() {
        log::trace!("Parent was removed from {:?}", entity);
        if let Ok(mut previous_parent_children) = children_query.get_mut(previous_parent.0) {
            log::trace!(" > Removing {:?} from it's prev parent's children", entity);
            previous_parent_children.0.retain(|e| *e != entity);
            commands.remove_one::<PreviousParent>(entity);
        }
    }

    // Tracks all newly created `Children` Components this frame.
    let mut children_additions = HashMap::<Entity, SmallVec<[Entity; 8]>>::default();

    // Entities with a changed Parent (that also have a PreviousParent, even if None)
    for (entity, parent, possible_previous_parent) in changed_parent_query.iter_mut() {
        log::trace!("Parent changed for {:?}", entity);
        if let Some(mut previous_parent) = possible_previous_parent {
            // New and previous point to the same Entity, carry on, nothing to see here.
            if previous_parent.0 == parent.0 {
                log::trace!(" > But the previous parent is the same, ignoring...");
                continue;
            }

            // Remove from `PreviousParent.Children`.
            if let Ok(mut previous_parent_children) = children_query.get_mut(previous_parent.0) {
                log::trace!(" > Removing {:?} from prev parent's children", entity);
                (*previous_parent_children).0.retain(|e| *e != entity);
            }

            // Set `PreviousParent = Parent`.
            *previous_parent = PreviousParent(parent.0);
        } else {
            log::trace!("Adding missing PreviousParent to {:?}", entity);
            commands.insert_one(entity, PreviousParent(parent.0));
        };

        // Add to the parent's `Children` (either the real component, or
        // `children_additions`).
        log::trace!("Adding {:?} to it's new parent {:?}", entity, parent.0);
        if let Ok(mut new_parent_children) = children_query.get_mut(parent.0) {
            // This is the parent
            log::trace!(
                " > The new parent {:?} already has a `Children`, adding to it.",
                parent.0
            );
            (*new_parent_children).0.push(entity);
        } else {
            // The parent doesn't have a children entity, lets add it
            log::trace!(
                "The new parent {:?} doesn't yet have `Children` component.",
                parent.0
            );
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
        log::trace!(
            "Flushing: Entity {:?} adding `Children` component {:?}",
            k,
            v
        );
        commands.insert_one(*k, Children::with(v));
    });
}

pub fn hierarchy_maintenance_systems() -> Vec<Box<dyn System>> {
    vec![parent_update_system.system()]
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{hierarchy::BuildChildren, transform_systems};
    use bevy_ecs::{Resources, Schedule, World};
    use bevy_math::Vec3;

    #[test]
    fn correct_children() {
        let mut world = World::default();
        let mut resources = Resources::default();

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        for system in transform_systems() {
            schedule.add_system_to_stage("update", system);
        }

        // Add parent entities
        let mut commands = Commands::default();
        commands.set_entity_reserver(world.get_entity_reserver());
        let mut parent = None;
        let mut children = Vec::new();
        commands
            .spawn((Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),))
            .for_current_entity(|entity| parent = Some(entity))
            .with_children(|parent| {
                parent
                    .spawn((Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),))
                    .for_current_entity(|entity| children.push(entity))
                    .spawn((Transform::from_translation(Vec3::new(0.0, 0.0, 3.0)),))
                    .for_current_entity(|entity| children.push(entity));
            });
        let parent = parent.unwrap();
        commands.apply(&mut world, &mut resources);
        schedule.initialize(&mut world, &mut resources);
        schedule.run(&mut world, &mut resources);

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

        schedule.run(&mut world, &mut resources);

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

        schedule.run(&mut world, &mut resources);

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
