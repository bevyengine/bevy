use crate::components::{GlobalTransform, Transform};
use bevy_ecs::{
    entity::Entity,
    prelude::Changed,
    query::{With, Without},
    system::Query,
};
use bevy_hierarchy::{Children, Parent};

/// Update [`GlobalTransform`] component of entities based on entity hierarchy and
/// [`Transform`] component.
pub fn transform_propagate_system(
    mut root_query: Query<
        (
            Option<&Children>,
            &Transform,
            Changed<Transform>,
            &mut GlobalTransform,
        ),
        Without<Parent>,
    >,
    mut transform_query: Query<
        (&Transform, Changed<Transform>, &mut GlobalTransform),
        With<Parent>,
    >,
    children_query: Query<Option<&Children>, (With<Parent>, With<GlobalTransform>)>,
) {
    for (children, transform, transform_changed, mut global_transform) in root_query.iter_mut() {
        let mut changed = false;
        if transform_changed {
            *global_transform = GlobalTransform::from(*transform);
            changed = true;
        }

        if let Some(children) = children {
            for child in children.iter() {
                propagate_recursive(
                    &global_transform,
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
    transform_query: &mut Query<
        (&Transform, Changed<Transform>, &mut GlobalTransform),
        With<Parent>,
    >,
    children_query: &Query<Option<&Children>, (With<Parent>, With<GlobalTransform>)>,
    entity: Entity,
    mut changed: bool,
) {
    let global_matrix = {
        if let Ok((transform, transform_changed, mut global_transform)) =
            transform_query.get_mut(entity)
        {
            changed |= transform_changed;
            if changed {
                *global_transform = parent.mul_transform(*transform);
            }
            *global_transform
        } else {
            return;
        }
    };

    if let Ok(Some(children)) = children_query.get(entity) {
        for child in children.iter() {
            propagate_recursive(
                &global_matrix,
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

    use crate::components::{GlobalTransform, Transform};
    use crate::systems::transform_propagate_system;
    use crate::TransformBundle;
    use bevy_hierarchy::{
        parent_update_system, BuildChildren, BuildWorldChildren, Children, Parent,
    };

    #[test]
    fn did_propagate() {
        let mut world = World::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(parent_update_system);
        update_stage.add_system(transform_propagate_system);

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        // Root entity
        world
            .spawn()
            .insert_bundle(TransformBundle::from(Transform::from_xyz(1.0, 0.0, 0.0)));

        let mut children = Vec::new();
        world
            .spawn()
            .insert_bundle(TransformBundle::from(Transform::from_xyz(1.0, 0.0, 0.0)))
            .with_children(|parent| {
                children.push(
                    parent
                        .spawn_bundle(TransformBundle::from(Transform::from_xyz(0.0, 2.0, 0.)))
                        .id(),
                );
                children.push(
                    parent
                        .spawn_bundle(TransformBundle::from(Transform::from_xyz(0.0, 0.0, 3.)))
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
            .spawn_bundle(TransformBundle::from(Transform::from_xyz(1.0, 0.0, 0.0)))
            .with_children(|parent| {
                children.push(
                    parent
                        .spawn_bundle(TransformBundle::from(Transform::from_xyz(0.0, 2.0, 0.0)))
                        .id(),
                );
                children.push(
                    parent
                        .spawn_bundle(TransformBundle::from(Transform::from_xyz(0.0, 0.0, 3.0)))
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

    #[test]
    fn correct_children() {
        let mut world = World::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(parent_update_system);
        update_stage.add_system(transform_propagate_system);

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        // Add parent entities
        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, &world);
        let mut children = Vec::new();
        let parent = commands
            .spawn()
            .insert(Transform::from_xyz(1.0, 0.0, 0.0))
            .id();
        commands.entity(parent).with_children(|parent| {
            children.push(
                parent
                    .spawn()
                    .insert(Transform::from_xyz(0.0, 2.0, 0.0))
                    .id(),
            );
            children.push(
                parent
                    .spawn()
                    .insert(Transform::from_xyz(0.0, 3.0, 0.0))
                    .id(),
            );
        });
        command_queue.apply(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            world
                .get::<Children>(parent)
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            children,
        );

        // Parent `e1` to `e2`.
        (*world.get_mut::<Parent>(children[0]).unwrap()).0 = children[1];

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
