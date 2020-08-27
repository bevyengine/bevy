use crate::components::*;
use bevy_ecs::prelude::*;
use bevy_math::Mat4;

// ANSWERME: make this take all transforms and perform in recursion (looking at compute shader)
pub fn transform_propagate_system(
    mut root_query: Query<Without<Parent, (Option<&Children>, &mut Transform)>>,
    mut transform_query: Query<(&mut Transform, Option<&Children>)>,
) {
    for (children, mut transform) in &mut root_query.iter() {
        transform.apply_parent_matrix(None);

        if let Some(children) = children {
            for child in children.0.iter() {
                propagate_recursive(*transform.global_matrix(), &mut transform_query, *child);
            }
        }
    }
}

// ANSWERME: maybe speed this up with compute
fn propagate_recursive(
    parent: Mat4,
    transform_query: &mut Query<(&mut Transform, Option<&Children>)>,
    entity: Entity,
) {
    log::trace!("Updating Transform for {:?}", entity);

    let global_matrix = {
        let mut transform = transform_query.get_mut::<Transform>(entity).unwrap();

        transform.apply_parent_matrix(Some(parent));
        *transform.global_matrix()
    };

    // Collect children
    let children = transform_query
        .get::<Children>(entity)
        .map(|e| e.0.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    for child in children {
        propagate_recursive(global_matrix, transform_query, child);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{hierarchy::BuildChildren, transform_systems};
    use bevy_ecs::{Resources, Schedule, World};
    use bevy_math::{Mat4, Vec3};

    #[test]
    fn did_propagate() {
        let mut world = World::default();
        let mut resources = Resources::default();

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        for system in transform_systems() {
            schedule.add_system_to_stage("update", system);
        }

        // Root entity
        let parent = world.spawn((Transform::from(Translation::new(1.0, 0.0, 0.0)), ())); //FIXME: shouldn't need () to be added
        let children = world
            .spawn_batch(vec![
                (
                    Transform::from(Translation::new(0.0, 2.0, 0.0)),
                    Parent(parent),
                ),
                (
                    Transform::from(Translation::new(0.0, 0.0, 3.0)),
                    Parent(parent),
                ),
            ])
            .collect::<Vec<Entity>>();
        // we need to run the schedule three times because components need to be filled in
        // to resolve this problem in code, just add the correct components, or use Commands
        // which adds all of the components needed with the correct state (see next test)
        schedule.run(&mut world, &mut resources);
        schedule.run(&mut world, &mut resources);
        schedule.run(&mut world, &mut resources);

        assert_eq!(
            *world.get::<Transform>(children[0]).unwrap().global_matrix(),
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0))
        );

        assert_eq!(
            *world.get::<Transform>(children[1]).unwrap().global_matrix(),
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Mat4::from_translation(Vec3::new(0.0, 0.0, 3.0))
        );
    }

    #[test]
    fn did_propagate_command_buffer() {
        let mut world = World::default();
        let mut resources = Resources::default();

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        for system in transform_systems() {
            schedule.add_system_to_stage("update", system);
        }

        // Root entity
        let mut commands = Commands::default();
        let mut children = Vec::new();
        commands
            .spawn((Transform::from(Translation::new(1.0, 0.0, 0.0)), ())) //FIXME: shouldn't need () to be added
            .with_children(|parent| {
                parent
                    .spawn((Transform::from(Translation::new(0.0, 2.0, 0.0)), ()))
                    .for_current_entity(|entity| children.push(entity))
                    .spawn((Transform::from(Translation::new(0.0, 0.0, 3.0)), ()))
                    .for_current_entity(|entity| children.push(entity));
            });
        commands.apply(&mut world, &mut resources);
        schedule.run(&mut world, &mut resources);

        assert_eq!(
            *world.get::<Transform>(children[0]).unwrap().global_matrix(),
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0))
        );

        assert_eq!(
            *world.get::<Transform>(children[1]).unwrap().global_matrix(),
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Mat4::from_translation(Vec3::new(0.0, 0.0, 3.0))
        );
    }
}
