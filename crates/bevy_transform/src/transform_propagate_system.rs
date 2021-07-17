use crate::components::{Children, GlobalTransform, Parent, Transform};
use bevy_ecs::{
    entity::Entity,
    query::{Changed, With, Without},
    system::Query,
};

/// Update [`GlobalTransform`] component of entities based on entity hierarchy and
/// [`Transform`] component.
pub fn transform_propagate_system(
    mut root_query: Query<
        (Entity, Option<&Children>, &Transform, &mut GlobalTransform),
        Without<Parent>,
    >,
    mut transform_query: Query<(&Transform, &mut GlobalTransform), With<Parent>>,
    changed_transform_query: Query<Entity, Changed<Transform>>,
    children_query: Query<Option<&Children>, (With<Parent>, With<GlobalTransform>)>,
) {
    for (entity, children, transform, mut global_transform) in root_query.iter_mut() {
        let mut changed = false;
        if changed_transform_query.get(entity).is_ok() {
            *global_transform = GlobalTransform::from(*transform);
            changed = true;
        }

        if let Some(children) = children {
            for child in children.0.iter() {
                propagate_recursive(
                    &global_transform,
                    &changed_transform_query,
                    &mut transform_query,
                    &children_query,
                    *child,
                    changed,
                );
            }
        }
    }
}

fn propagate_recursive(
    parent: &GlobalTransform,
    changed_transform_query: &Query<Entity, Changed<Transform>>,
    transform_query: &mut Query<(&Transform, &mut GlobalTransform), With<Parent>>,
    children_query: &Query<Option<&Children>, (With<Parent>, With<GlobalTransform>)>,
    entity: Entity,
    mut changed: bool,
) {
    changed |= changed_transform_query.get(entity).is_ok();

    let global_matrix = {
        if let Ok((transform, mut global_transform)) = transform_query.get_mut(entity) {
            if changed {
                *global_transform = parent.mul_transform(*transform);
            }
            *global_transform
        } else {
            return;
        }
    };

    if let Ok(Some(children)) = children_query.get(entity) {
        for child in children.0.iter() {
            propagate_recursive(
                &global_matrix,
                changed_transform_query,
                transform_query,
                children_query,
                *child,
                changed,
            );
        }
    }
}

#[cfg(test)]
mod test {
    use bevy_ecs::{
        schedule::{Schedule, Stage, SystemStage},
        system::{CommandQueue, Commands},
        world::World,
    };

    use super::*;
    use crate::hierarchy::{parent_update_system, BuildChildren, BuildWorldChildren};

    #[test]
    fn did_propagate() {
        let mut world = World::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(parent_update_system);
        update_stage.add_system(transform_propagate_system);

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        // Root entity
        world.spawn().insert_bundle((
            Transform::from_xyz(1.0, 0.0, 0.0),
            GlobalTransform::identity(),
        ));

        let mut children = Vec::new();
        world
            .spawn()
            .insert_bundle((
                Transform::from_xyz(1.0, 0.0, 0.0),
                GlobalTransform::identity(),
            ))
            .with_children(|parent| {
                children.push(
                    parent
                        .spawn_bundle((
                            Transform::from_xyz(0.0, 2.0, 0.),
                            GlobalTransform::identity(),
                        ))
                        .id(),
                );
                children.push(
                    parent
                        .spawn_bundle((
                            Transform::from_xyz(0.0, 0.0, 3.),
                            GlobalTransform::identity(),
                        ))
                        .id(),
                );
            });
        schedule.run(&mut world);

        assert_eq!(
            *world.get::<GlobalTransform>(children[0]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 2.0, 0.0)
        );

        assert_eq!(
            *world.get::<GlobalTransform>(children[1]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 0.0, 3.0)
        );
    }

    #[test]
    fn did_propagate_command_buffer() {
        let mut world = World::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(parent_update_system);
        update_stage.add_system(transform_propagate_system);

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        // Root entity
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut children = Vec::new();
        commands
            .spawn_bundle((
                Transform::from_xyz(1.0, 0.0, 0.0),
                GlobalTransform::identity(),
            ))
            .with_children(|parent| {
                children.push(
                    parent
                        .spawn_bundle((
                            Transform::from_xyz(0.0, 2.0, 0.0),
                            GlobalTransform::identity(),
                        ))
                        .id(),
                );
                children.push(
                    parent
                        .spawn_bundle((
                            Transform::from_xyz(0.0, 0.0, 3.0),
                            GlobalTransform::identity(),
                        ))
                        .id(),
                );
            });
        queue.apply(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            *world.get::<GlobalTransform>(children[0]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 2.0, 0.0)
        );

        assert_eq!(
            *world.get::<GlobalTransform>(children[1]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 0.0, 3.0)
        );
    }
}
