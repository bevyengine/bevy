use crate::components::{GlobalTransform, Transform};
use bevy_ecs::{
    change_detection::Ref,
    prelude::{Changed, DetectChanges, Entity, Query, With, Without},
    query::{Added, Or},
    removal_detection::RemovedComponents,
    system::{Local, ParamSet},
};
use bevy_hierarchy::{Children, Parent};

/// Update [`GlobalTransform`] component of entities that aren't in the hierarchy
///
/// Third party plugins should ensure that this is used in concert with [`propagate_transforms`].
pub fn sync_simple_transforms(
    mut query: ParamSet<(
        Query<
            (&Transform, &mut GlobalTransform),
            (
                Or<(Changed<Transform>, Added<GlobalTransform>)>,
                Without<Parent>,
                Without<Children>,
            ),
        >,
        Query<(Ref<Transform>, &mut GlobalTransform), (Without<Parent>, Without<Children>)>,
    )>,
    mut orphaned: RemovedComponents<Parent>,
) {
    // Update changed entities.
    query
        .p0()
        .par_iter_mut()
        .for_each(|(transform, mut global_transform)| {
            *global_transform = GlobalTransform::from(*transform);
        });
    // Update orphaned entities.
    let mut query = query.p1();
    let mut iter = query.iter_many_mut(orphaned.read());
    while let Some((transform, mut global_transform)) = iter.fetch_next() {
        if !transform.is_changed() && !global_transform.is_added() {
            *global_transform = GlobalTransform::from(*transform);
        }
    }
}

/// Update [`GlobalTransform`] component of entities based on entity hierarchy and
/// [`Transform`] component.
///
/// Third party plugins should ensure that this is used in concert with [`sync_simple_transforms`].
pub fn propagate_transforms(
    mut root_query: Query<
        (Entity, &Children, Ref<Transform>, &mut GlobalTransform),
        Without<Parent>,
    >,
    mut orphaned: RemovedComponents<Parent>,
    transform_query: Query<(Ref<Transform>, &mut GlobalTransform, Option<&Children>), With<Parent>>,
    parent_query: Query<(Entity, Ref<Parent>)>,
    mut orphaned_entities: Local<Vec<Entity>>,
) {
    orphaned_entities.clear();
    orphaned_entities.extend(orphaned.read());
    orphaned_entities.sort_unstable();
    root_query.par_iter_mut().for_each(
        |(entity, children, transform, mut global_transform)| {
            let changed = transform.is_changed() || global_transform.is_added() || orphaned_entities.binary_search(&entity).is_ok();
            if changed {
                *global_transform = GlobalTransform::from(*transform);
            }

            for (child, actual_parent) in parent_query.iter_many(children) {
                assert_eq!(
                    actual_parent.get(), entity,
                    "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
                );
                // SAFETY:
                // - `child` must have consistent parentage, or the above assertion would panic.
                // Since `child` is parented to a root entity, the entire hierarchy leading to it is consistent.
                // - We may operate as if all descendants are consistent, since `propagate_recursive` will panic before 
                //   continuing to propagate if it encounters an entity with inconsistent parentage.
                // - Since each root entity is unique and the hierarchy is consistent and forest-like,
                //   other root entities' `propagate_recursive` calls will not conflict with this one.
                // - Since this is the only place where `transform_query` gets used, there will be no conflicting fetches elsewhere.
                #[allow(unsafe_code)]
                unsafe {
                    propagate_recursive(
                        &global_transform,
                        &transform_query,
                        &parent_query,
                        child,
                        changed || actual_parent.is_changed(),
                    );
                }
            }
        },
    );
}

/// Recursively propagates the transforms for `entity` and all of its descendants.
///
/// # Panics
///
/// If `entity`'s descendants have a malformed hierarchy, this function will panic occur before propagating
/// the transforms of any malformed entities and their descendants.
///
/// # Safety
///
/// - While this function is running, `transform_query` must not have any fetches for `entity`,
///     nor any of its descendants.
/// - The caller must ensure that the hierarchy leading to `entity`
///     is well-formed and must remain as a tree or a forest. Each entity must have at most one parent.
#[allow(unsafe_code)]
unsafe fn propagate_recursive(
    parent: &GlobalTransform,
    transform_query: &Query<
        (Ref<Transform>, &mut GlobalTransform, Option<&Children>),
        With<Parent>,
    >,
    parent_query: &Query<(Entity, Ref<Parent>)>,
    entity: Entity,
    mut changed: bool,
) {
    let (global_matrix, children) = {
        let Ok((transform, mut global_transform, children)) =
            // SAFETY: This call cannot create aliased mutable references.
            //   - The top level iteration parallelizes on the roots of the hierarchy.
            //   - The caller ensures that each child has one and only one unique parent throughout the entire
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
            (unsafe { transform_query.get_unchecked(entity) }) else {
                return;
            };

        changed |= transform.is_changed() || global_transform.is_added();
        if changed {
            *global_transform = parent.mul_transform(*transform);
        }
        (*global_transform, children)
    };

    let Some(children) = children else { return };
    for (child, actual_parent) in parent_query.iter_many(children) {
        assert_eq!(
            actual_parent.get(), entity,
            "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );
        // SAFETY: The caller guarantees that `transform_query` will not be fetched
        // for any descendants of `entity`, so it is safe to call `propagate_recursive` for each child.
        //
        // The above assertion ensures that each child has one and only one unique parent throughout the
        // entire hierarchy.
        unsafe {
            propagate_recursive(
                &global_matrix,
                transform_query,
                parent_query,
                child,
                changed || actual_parent.is_changed(),
            );
        }
    }
}

#[cfg(test)]
mod test {
    use bevy_app::prelude::*;
    use bevy_ecs::prelude::*;
    use bevy_ecs::world::CommandQueue;
    use bevy_math::{vec3, Vec3};
    use bevy_tasks::{ComputeTaskPool, TaskPool};

    use crate::bundles::TransformBundle;
    use crate::systems::*;
    use bevy_hierarchy::{BuildChildren, BuildWorldChildren};

    #[test]
    fn correct_parent_removed() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut world = World::default();
        let offset_global_transform =
            |offset| GlobalTransform::from(Transform::from_xyz(offset, offset, offset));
        let offset_transform =
            |offset| TransformBundle::from_transform(Transform::from_xyz(offset, offset, offset));

        let mut schedule = Schedule::default();
        schedule.add_systems((sync_simple_transforms, propagate_transforms));

        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, &world);
        let root = commands.spawn(offset_transform(3.3)).id();
        let parent = commands.spawn(offset_transform(4.4)).id();
        let child = commands.spawn(offset_transform(5.5)).id();
        commands.entity(parent).set_parent(root);
        commands.entity(child).set_parent(parent);
        command_queue.apply(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            world.get::<GlobalTransform>(parent).unwrap(),
            &offset_global_transform(4.4 + 3.3),
            "The transform systems didn't run, ie: `GlobalTransform` wasn't updated",
        );

        // Remove parent of `parent`
        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, &world);
        commands.entity(parent).remove_parent();
        command_queue.apply(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            world.get::<GlobalTransform>(parent).unwrap(),
            &offset_global_transform(4.4),
            "The global transform of an orphaned entity wasn't updated properly",
        );

        // Remove parent of `child`
        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, &world);
        commands.entity(child).remove_parent();
        command_queue.apply(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            world.get::<GlobalTransform>(child).unwrap(),
            &offset_global_transform(5.5),
            "The global transform of an orphaned entity wasn't updated properly",
        );
    }

    #[test]
    fn did_propagate() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut world = World::default();

        let mut schedule = Schedule::default();
        schedule.add_systems((sync_simple_transforms, propagate_transforms));

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

        let mut schedule = Schedule::default();
        schedule.add_systems((sync_simple_transforms, propagate_transforms));

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
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut world = World::default();

        let mut schedule = Schedule::default();
        schedule.add_systems((sync_simple_transforms, propagate_transforms));

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
        ComputeTaskPool::get_or_init(TaskPool::default);

        app.add_systems(Update, (sync_simple_transforms, propagate_transforms));

        let translation = vec3(1.0, 0.0, 0.0);

        // These will be overwritten.
        let mut child = Entity::from_raw(0);
        let mut grandchild = Entity::from_raw(1);
        let parent = app
            .world_mut()
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
        assert_eq!(&**app.world().get::<Children>(parent).unwrap(), &[child]);
        assert_eq!(
            &**app.world().get::<Children>(child).unwrap(),
            &[grandchild]
        );
        // Note that at this point, the `GlobalTransform`s will not have updated yet, due to `Commands` delay
        app.update();

        let mut state = app.world_mut().query::<&GlobalTransform>();
        for global in state.iter(app.world()) {
            assert_eq!(global, &GlobalTransform::from_translation(translation));
        }
    }

    #[test]
    #[should_panic]
    fn panic_when_hierarchy_cycle() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        // We cannot directly edit Parent and Children, so we use a temp world to break
        // the hierarchy's invariants.
        let mut temp = World::new();
        let mut app = App::new();

        app.add_systems(Update, (propagate_transforms, sync_simple_transforms));

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
        let (child, grandchild) = setup_world(app.world_mut());

        assert_eq!(temp_child, child);
        assert_eq!(temp_grandchild, grandchild);

        app.world_mut()
            .spawn(TransformBundle::IDENTITY)
            .push_children(&[child]);
        std::mem::swap(
            &mut *app.world_mut().get_mut::<Parent>(child).unwrap(),
            &mut *temp.get_mut::<Parent>(grandchild).unwrap(),
        );

        app.update();
    }

    #[test]
    fn global_transform_should_not_be_overwritten_after_reparenting() {
        let translation = Vec3::ONE;
        let mut world = World::new();

        // Create transform propagation schedule
        let mut schedule = Schedule::default();
        schedule.add_systems((sync_simple_transforms, propagate_transforms));

        // Spawn a `TransformBundle` entity with a local translation of `Vec3::ONE`
        let mut spawn_transform_bundle = || {
            world
                .spawn(TransformBundle::from_transform(
                    Transform::from_translation(translation),
                ))
                .id()
        };

        // Spawn parent and child with identical transform bundles
        let parent = spawn_transform_bundle();
        let child = spawn_transform_bundle();
        world.entity_mut(parent).add_child(child);

        // Run schedule to propagate transforms
        schedule.run(&mut world);

        // Child should be positioned relative to its parent
        let parent_global_transform = *world.entity(parent).get::<GlobalTransform>().unwrap();
        let child_global_transform = *world.entity(child).get::<GlobalTransform>().unwrap();
        assert!(parent_global_transform
            .translation()
            .abs_diff_eq(translation, 0.1));
        assert!(child_global_transform
            .translation()
            .abs_diff_eq(2. * translation, 0.1));

        // Reparent child
        world.entity_mut(child).remove_parent();
        world.entity_mut(parent).add_child(child);

        // Run schedule to propagate transforms
        schedule.run(&mut world);

        // Translations should be unchanged after update
        assert_eq!(
            parent_global_transform,
            *world.entity(parent).get::<GlobalTransform>().unwrap()
        );
        assert_eq!(
            child_global_transform,
            *world.entity(child).get::<GlobalTransform>().unwrap()
        );
    }
}
