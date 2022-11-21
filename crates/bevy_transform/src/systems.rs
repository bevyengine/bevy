use crate::components::{GlobalTransform, Transform};
use bevy_ecs::{
    entity::Entity,
    query::{Changed, Or, With, Without},
    system::{Local, Query},
};
use bevy_hierarchy::{Children, Parent};
use core::cell::Cell;
use thread_local::ThreadLocal;

pub(crate) struct Pending {
    parent: *const GlobalTransform,
    changed: bool,
    parent_entity: Entity,
    child: Entity,
}

// SAFE: Access to the parent pointer is only usable in this module, the values
// are cleared after every system execution, and the system only uses one
// thread. There is no way to move this type across multiple threads.
unsafe impl Send for Pending {}
// SAFE: Access to the parent pointer is only usable in this module, the values
// are cleared after every system execution, and the system only uses one
// thread. There is no way to access this type across multiple threads.
unsafe impl Sync for Pending {}

/// Update [`GlobalTransform`] component of entities that aren't in the hierarchy
pub fn sync_simple_transforms(
    mut query: Query<
        (&Transform, &mut GlobalTransform),
        (Changed<Transform>, Without<Parent>, Without<Children>),
    >,
) {
    query.par_for_each_mut(1024, |(transform, mut global_transform)| {
        *global_transform = GlobalTransform::from(*transform);
    });
}

/// Update [`GlobalTransform`] component of entities based on entity hierarchy and
/// [`Transform`] component.
pub(crate) fn propagate_transforms(
    mut root_query: Query<
        (
            Entity,
            &Transform,
            &Children,
            Or<(Changed<Transform>, Changed<Children>)>,
            &mut GlobalTransform,
        ),
        Without<Parent>,
    >,
    parent_query: Query<&Parent>,
    transform_query: Query<
        (
            &Transform,
            Or<(Changed<Transform>, Changed<Children>)>,
            Option<&Children>,
            &mut GlobalTransform,
        ),
        With<Parent>,
    >,
    // Stack space for the depth-first search of a given hierarchy. Used as a Local to
    // avoid reallocating the stack space used here.
    pending_queues: Local<ThreadLocal<Cell<Vec<Pending>>>>,
) {
    root_query.par_for_each_mut(
        // The differing depths and sizes of hierarchy trees causes the work for each root to be
        // different. A batch size of 1 ensures that each tree gets it's own task and multiple
        // large trees are not clumped together.
        1,
        |(root, transform, children, changed,  mut global_transform)| {
            let pending_cell = pending_queues.get_or_default();
            let mut pending = pending_cell.take();

            if changed {
                *global_transform = GlobalTransform::from(*transform);
            }

            pending.extend(children.iter().map(|child| Pending {
                parent: &*global_transform as *const GlobalTransform,
                changed,
                parent_entity: root,
                child: *child,
            }));

            while let Some(current) = pending.pop() {
                let Ok(actual_parent) = parent_query.get(current.child) else {
                    panic!("Propagated child for {:?} has no Parent component!", current.child);
                };
                assert_eq!(
                    actual_parent.get(), current.parent_entity,
                    "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
                );

                // SAFETY: This call cannot create aliased mutable references.
                //   - The top level iteration parallelizes on the roots of the hierarchy.
                //   - The above assertion ensures that each child has one and only one unique parent throughout the entire
                //     hierarchy.
                //
                // For example, consider the following malformed hierarchy:
                //
                //     A
                //   /   \
                //  B     C
                //   \   /
                //     D
                //
                // D has two parents, B and C. If the propagation passes through C, but the Parent component on D points to B,
                // the above check will panic as the origin parent does match the recorded parent.
                //
                // Also consider the following case, where A and B are roots:
                //
                //  A       B
                //   \     /
                //    C   D
                //     \ /
                //      E
                //
                // Even if these A and B start two separate tasks running in parallel, one of them will panic before attempting
                // to mutably access E.
                let fetch = unsafe { transform_query.get_unchecked(current.child) };

                // If our `Children` has changed, we need to recalculate everything below us
                let Ok((transform, mut changed, children, mut global_transform)) = fetch else {
                    continue;
                };

                changed |= current.changed;
                if changed {
                    // SAFE: The pointers here are generated only during this one traversal
                    // from one given run of the system.
                    unsafe {
                        *global_transform = current.parent.read().mul_transform(*transform);
                    };
                }
                if let Some(children) = children {
                    pending.extend(children.iter().map(|child| Pending {
                        parent: &*global_transform as *const GlobalTransform,
                        changed: current.changed,
                        parent_entity: current.child,
                        child: *child,
                    }));
                }
            }

            debug_assert!(pending.is_empty());

        });
}

#[cfg(test)]
mod test {
    use bevy_app::prelude::*;
    use bevy_ecs::prelude::*;
    use bevy_ecs::system::CommandQueue;
    use bevy_math::vec3;
    use bevy_tasks::{ComputeTaskPool, TaskPool};

    use crate::components::{GlobalTransform, Transform};
    use crate::systems::*;
    use crate::TransformBundle;
    use bevy_hierarchy::{BuildChildren, BuildWorldChildren, Children, Parent};

    #[derive(StageLabel)]
    struct Update;

    #[test]
    fn did_propagate() {
        ComputeTaskPool::init(TaskPool::default);
        let mut world = World::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(sync_simple_transforms);
        update_stage.add_system(propagate_transforms);

        let mut schedule = Schedule::default();
        schedule.add_stage(Update, update_stage);

        // Root entity
        world.spawn(TransformBundle::from(Transform::from_xyz(1.0, 0.0, 0.0)));

        let mut children = Vec::new();
        world
            .spawn(TransformBundle::from(Transform::from_xyz(1.0, 0.0, 0.0)))
            .with_children(|parent| {
                children.push(
                    parent
                        .spawn(TransformBundle::from(Transform::from_xyz(0.0, 2.0, 0.)))
                        .id(),
                );
                children.push(
                    parent
                        .spawn(TransformBundle::from(Transform::from_xyz(0.0, 0.0, 3.)))
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
        update_stage.add_system(sync_simple_transforms);
        update_stage.add_system(propagate_transforms);

        let mut schedule = Schedule::default();
        schedule.add_stage(Update, update_stage);

        // Root entity
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut children = Vec::new();
        commands
            .spawn(TransformBundle::from(Transform::from_xyz(1.0, 0.0, 0.0)))
            .with_children(|parent| {
                children.push(
                    parent
                        .spawn(TransformBundle::from(Transform::from_xyz(0.0, 2.0, 0.0)))
                        .id(),
                );
                children.push(
                    parent
                        .spawn(TransformBundle::from(Transform::from_xyz(0.0, 0.0, 3.0)))
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
        ComputeTaskPool::init(TaskPool::default);
        let mut world = World::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(sync_simple_transforms);
        update_stage.add_system(propagate_transforms);

        let mut schedule = Schedule::default();
        schedule.add_stage(Update, update_stage);

        // Add parent entities
        let mut children = Vec::new();
        let parent = {
            let mut command_queue = CommandQueue::default();
            let mut commands = Commands::new(&mut command_queue, &world);
            let parent = commands.spawn(Transform::from_xyz(1.0, 0.0, 0.0)).id();
            commands.entity(parent).with_children(|parent| {
                children.push(parent.spawn(Transform::from_xyz(0.0, 2.0, 0.0)).id());
                children.push(parent.spawn(Transform::from_xyz(0.0, 3.0, 0.0)).id());
            });
            command_queue.apply(&mut world);
            schedule.run(&mut world);
            parent
        };

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
        {
            let mut command_queue = CommandQueue::default();
            let mut commands = Commands::new(&mut command_queue, &world);
            commands.entity(children[1]).add_child(children[0]);
            command_queue.apply(&mut world);
            schedule.run(&mut world);
        }

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

    #[test]
    fn correct_transforms_when_no_children() {
        let mut app = App::new();
        ComputeTaskPool::init(TaskPool::default);

        app.add_system(sync_simple_transforms);
        app.add_system(propagate_transforms);

        let translation = vec3(1.0, 0.0, 0.0);

        // These will be overwritten.
        let mut child = Entity::from_raw(0);
        let mut grandchild = Entity::from_raw(1);
        let parent = app
            .world
            .spawn((
                Transform::from_translation(translation),
                GlobalTransform::IDENTITY,
            ))
            .with_children(|builder| {
                child = builder
                    .spawn(TransformBundle::IDENTITY)
                    .with_children(|builder| {
                        grandchild = builder.spawn(TransformBundle::IDENTITY).id();
                    })
                    .id();
            })
            .id();

        app.update();

        // check the `Children` structure is spawned
        assert_eq!(&**app.world.get::<Children>(parent).unwrap(), &[child]);
        assert_eq!(&**app.world.get::<Children>(child).unwrap(), &[grandchild]);
        // Note that at this point, the `GlobalTransform`s will not have updated yet, due to `Commands` delay
        app.update();

        let mut state = app.world.query::<&GlobalTransform>();
        for global in state.iter(&app.world) {
            assert_eq!(global, &GlobalTransform::from_translation(translation));
        }
    }

    #[test]
    #[should_panic]
    fn panic_when_hierarchy_cycle() {
        ComputeTaskPool::init(TaskPool::default);
        // We cannot directly edit Parent and Children, so we use a temp world to break
        // the hierarchy's invariants.
        let mut temp = World::new();
        let mut app = App::new();

        app.add_system(propagate_transforms)
            .add_system(sync_simple_transforms);

        fn setup_world(world: &mut World) -> (Entity, Entity) {
            let mut grandchild = Entity::from_raw(0);
            let child = world
                .spawn(TransformBundle::IDENTITY)
                .with_children(|builder| {
                    grandchild = builder.spawn(TransformBundle::IDENTITY).id();
                })
                .id();
            (child, grandchild)
        }

        let (temp_child, temp_grandchild) = setup_world(&mut temp);
        let (child, grandchild) = setup_world(&mut app.world);

        assert_eq!(temp_child, child);
        assert_eq!(temp_grandchild, grandchild);

        app.world
            .spawn(TransformBundle::IDENTITY)
            .push_children(&[child]);
        std::mem::swap(
            &mut *app.world.get_mut::<Parent>(child).unwrap(),
            &mut *temp.get_mut::<Parent>(grandchild).unwrap(),
        );

        app.update();
    }
}
