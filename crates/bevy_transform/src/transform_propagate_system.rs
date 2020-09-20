use crate::components::*;
use bevy_ecs::prelude::*;
use bevy_math::Mat4;

pub fn transform_propagate_system(
    mut root_query: Query<Without<Parent, (Option<&Children>, &Transform, &mut GlobalTransform)>>,
    mut transform_query: Query<(&Transform, &mut GlobalTransform, Option<&Children>)>,
) {
    for (children, transform, mut global_transform) in &mut root_query.iter() {
        *global_transform.value_mut() = *transform.value();

        if let Some(children) = children {
            for child in children.0.iter() {
                propagate_recursive(*global_transform.value(), &mut transform_query, *child);
            }
        }
    }
}

fn propagate_recursive(
    parent: Mat4,
    transform_query: &mut Query<(&Transform, &mut GlobalTransform, Option<&Children>)>,
    entity: Entity,
) {
    log::trace!("Updating Transform for {:?}", entity);

    let global_matrix = {
        if let (Ok(transform), Ok(mut global_transform)) = (
            transform_query.get::<Transform>(entity),
            transform_query.get_mut::<GlobalTransform>(entity),
        ) {
            *global_transform.value_mut() = parent * *transform.value();
            *global_transform.value()
        } else {
            return;
        }
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
        let parent = world.spawn((
            Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
            GlobalTransform::identity(),
        ));
        let children = world
            .spawn_batch(vec![
                (
                    Transform::from_translation(Vec3::new(0.0, 2.0, 0.)),
                    Parent(parent),
                    GlobalTransform::identity(),
                ),
                (
                    Transform::from_translation(Vec3::new(0.0, 0.0, 3.)),
                    Parent(parent),
                    GlobalTransform::identity(),
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
            *world.get::<GlobalTransform>(children[0]).unwrap().value(),
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0))
        );

        assert_eq!(
            *world.get::<GlobalTransform>(children[1]).unwrap().value(),
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
        commands.set_entity_reserver(world.get_entity_reserver());
        let mut children = Vec::new();
        commands
            .spawn((
                Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
                GlobalTransform::identity(),
            ))
            .with_children(|parent| {
                parent
                    .spawn((
                        Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
                        GlobalTransform::identity(),
                    ))
                    .for_current_entity(|entity| children.push(entity))
                    .spawn((
                        Transform::from_translation(Vec3::new(0.0, 0.0, 3.0)),
                        GlobalTransform::identity(),
                    ))
                    .for_current_entity(|entity| children.push(entity));
            });
        commands.apply(&mut world, &mut resources);
        schedule.run(&mut world, &mut resources);

        assert_eq!(
            *world.get::<GlobalTransform>(children[0]).unwrap().value(),
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0))
        );

        assert_eq!(
            *world.get::<GlobalTransform>(children[1]).unwrap().value(),
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Mat4::from_translation(Vec3::new(0.0, 0.0, 3.0))
        );
    }
}
