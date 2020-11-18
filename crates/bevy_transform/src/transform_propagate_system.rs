use crate::components::*;
use bevy_ecs::prelude::*;

pub fn transform_propagate_system(
    mut root_query: Query<
        (Option<&Children>, &Transform, &mut GlobalTransform),
        (Without<Parent>, With<GlobalTransform>),
    >,
    mut transform_query: Query<(&Transform, &mut GlobalTransform), With<Parent>>,
    children_query: Query<Option<&Children>, (With<Parent>, With<GlobalTransform>)>,
) {
    for (children, transform, mut global_transform) in root_query.iter_mut() {
        *global_transform = GlobalTransform::from(*transform);

        if let Some(children) = children {
            for child in children.0.iter() {
                propagate_recursive(
                    &global_transform,
                    &mut transform_query,
                    &children_query,
                    *child,
                );
            }
        }
    }
}

fn propagate_recursive(
    parent: &GlobalTransform,
    transform_query: &mut Query<(&Transform, &mut GlobalTransform), With<Parent>>,
    children_query: &Query<Option<&Children>, (With<Parent>, With<GlobalTransform>)>,
    entity: Entity,
) {
    let global_matrix = {
        if let Ok((transform, mut global_transform)) = transform_query.get_mut(entity) {
            *global_transform = parent.mul_transform(*transform);
            *global_transform
        } else {
            return;
        }
    };

    if let Ok(Some(children)) = children_query.get(entity) {
        for child in children.0.iter() {
            propagate_recursive(&global_matrix, transform_query, children_query, *child);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::hierarchy::{parent_update_system, BuildChildren};
    use bevy_ecs::{Resources, Schedule, World};
    use bevy_math::Vec3;

    #[test]
    fn did_propagate() {
        let mut world = World::default();
        let mut resources = Resources::default();

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        schedule.add_system_to_stage("update", parent_update_system);
        schedule.add_system_to_stage("update", transform_propagate_system);

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
        // we need to run the schedule two times because components need to be filled in
        // to resolve this problem in code, just add the correct components, or use Commands
        // which adds all of the components needed with the correct state (see next test)
        schedule.initialize(&mut world, &mut resources);
        schedule.run(&mut world, &mut resources);
        schedule.run(&mut world, &mut resources);

        assert_eq!(
            *world.get::<GlobalTransform>(children[0]).unwrap(),
            GlobalTransform::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Transform::from_translation(Vec3::new(0.0, 2.0, 0.0))
        );

        assert_eq!(
            *world.get::<GlobalTransform>(children[1]).unwrap(),
            GlobalTransform::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Transform::from_translation(Vec3::new(0.0, 0.0, 3.0))
        );
    }

    #[test]
    fn did_propagate_command_buffer() {
        let mut world = World::default();
        let mut resources = Resources::default();

        let mut schedule = Schedule::default();
        schedule.add_stage("update");
        schedule.add_system_to_stage("update", parent_update_system);
        schedule.add_system_to_stage("update", transform_propagate_system);

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
        schedule.initialize(&mut world, &mut resources);
        schedule.run(&mut world, &mut resources);

        assert_eq!(
            *world.get::<GlobalTransform>(children[0]).unwrap(),
            GlobalTransform::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Transform::from_translation(Vec3::new(0.0, 2.0, 0.0))
        );

        assert_eq!(
            *world.get::<GlobalTransform>(children[1]).unwrap(),
            GlobalTransform::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Transform::from_translation(Vec3::new(0.0, 0.0, 3.0))
        );
    }
}
