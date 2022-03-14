use crate::components::{Children, GlobalTransform, Parent, Transform};
use bevy_ecs::{
    entity::Entity,
    query::{Changed, With, Without},
    system::{Local, Query},
};

/// Used for [`transform_propagate_system`]. Ignore otherwise.
pub(crate) struct Pending {
    parent: *const GlobalTransform,
    changed: bool,
    child: Entity,
}

// SAFE: Values are cleared after every frame and cannot otherwise be
// constructed without transmute.
unsafe impl Send for Pending {}
// SAFE: Values are cleared after every frame and cannot otherwise be
// constructed without transmute.
unsafe impl Sync for Pending {}

/// Update [`GlobalTransform`] component of entities based on entity hierarchy and
/// [`Transform`] component.
pub(crate) fn transform_propagate_flat_system(
    mut root_query: Query<
        (&Transform, &mut GlobalTransform),
        (Without<Parent>, Without<Children>, Changed<Transform>),
    >,
) {
    for (transform, mut global_transform) in root_query.iter_mut() {
        *global_transform = GlobalTransform::from(*transform);
    }
}

/// Update [`GlobalTransform`] component of entities based on entity hierarchy and
/// [`Transform`] component.
pub(crate) fn transform_propagate_system(
    mut root_query: Query<
        (
            &Transform,
            Changed<Transform>,
            &Children,
            &mut GlobalTransform,
        ),
        Without<Parent>,
    >,
    mut transform_query: Query<
        (
            &Transform,
            Changed<Transform>,
            Option<&Children>,
            &mut GlobalTransform,
        ),
        With<Parent>,
    >,
    // Stack space for the depth-first search of a given hierarchy. Used as a Local to
    // avoid reallocating the stack space used here.
    mut pending: Local<Vec<Pending>>,
) {
    for (transform, is_changed, children, mut global_transform) in root_query.iter_mut() {
        let mut changed = false;
        if is_changed {
            *global_transform = GlobalTransform::from(*transform);
            changed = true;
        }

        pending.extend(children.0.iter().map(|child| Pending {
            parent: &*global_transform as *const GlobalTransform,
            changed,
            child: *child,
        }));

        while let Some(mut current) = pending.pop() {
            if let Ok((transform, changed, children, mut global_transform)) =
                transform_query.get_mut(current.child)
            {
                current.changed |= changed;
                // SAFE: The pointers here are generated only during this one traversal
                // from one given run of the system.
                let global_matrix = unsafe {
                    if current.changed {
                        *global_transform = current.parent.read().mul_transform(*transform);
                    }
                    &*global_transform as *const GlobalTransform
                };
                if let Some(children) = children {
                    pending.extend(children.0.iter().map(|child| Pending {
                        parent: global_matrix,
                        changed: current.changed,
                        child: *child,
                    }));
                }
            }
        }
    }
    debug_assert!(pending.is_empty());
}

#[cfg(test)]
mod test {
    use bevy_ecs::{
        schedule::{Schedule, Stage, SystemStage},
        system::{CommandQueue, Commands},
        world::World,
    };

    use super::*;
    use crate::{
        hierarchy::{parent_update_system, BuildChildren, BuildWorldChildren},
        TransformBundle,
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
}
