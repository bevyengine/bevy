use core::ops::DerefMut;

use crate::components::{GlobalTransform, Transform};
use alloc::vec::Vec;
use bevy_ecs::prelude::*;
use bevy_tasks::TaskPool;
use bevy_utils::Parallel;

/// Update [`GlobalTransform`] component of entities that aren't in the hierarchy
///
/// Third party plugins should ensure that this is used in concert with [`propagate_transforms`].
pub fn sync_simple_transforms(
    mut query: ParamSet<(
        Query<
            (&Transform, &mut GlobalTransform),
            (
                Or<(Changed<Transform>, Added<GlobalTransform>)>,
                Without<ChildOf>,
                Without<Children>,
            ),
        >,
        Query<(Ref<Transform>, &mut GlobalTransform), (Without<ChildOf>, Without<Children>)>,
    )>,
    mut orphaned: RemovedComponents<ChildOf>,
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

/// Represent a subtree in the hierarchy when propagating transforms.
#[derive(Clone, Debug)]
pub struct PropagationNode {
    is_changed: bool,
    entity: Entity,
    global_transform: GlobalTransform,
}

/// Propagate transforms
pub fn propagate_transforms_par(
    task_pool: Local<TaskPool>,
    // Orphans
    mut orphaned: RemovedComponents<ChildOf>,
    mut orphans: Local<Vec<Entity>>,
    // Cached thread locals
    mut stack: Local<Parallel<Vec<PropagationNode>>>,
    mut next_stack: Local<Parallel<Vec<PropagationNode>>>,
    // Queries
    mut transform_queries: ParamSet<(
        Query<
            (Entity, Ref<Transform>, &mut GlobalTransform, Has<Children>),
            (Without<ChildOf>, With<Children>),
        >,
        Query<(Entity, Ref<Transform>, &mut GlobalTransform, Has<Children>), With<ChildOf>>,
        Query<&mut GlobalTransform>,
    )>,
    children: Query<&Children>,
) {
    // Orphans
    orphans.clear();
    orphans.extend(orphaned.read());
    orphans.sort_unstable();

    // Roots
    transform_queries.p0().par_iter_mut().for_each_init(
        || stack.borrow_local_mut(),
        |locals, components| {
            compute_transform(locals, None, components, &orphans);
        },
    );

    // Propagation stack
    loop {
        let stack_size = stack.iter_mut().fold(0, |acc, e| acc + e.len());
        if stack_size == 0 {
            break;
        }
        let local_stack_size = stack_size / task_pool.thread_num();

        // Important: this is very different from calling `Parallel::clear()`. Doing that will
        // reallocate the thread local, losing any allocated queues. Instead, we want to `clear`
        // each vector, which will retain allocations between system runs. This has a big impact on
        // performance.
        next_stack.iter_mut().for_each(|stack| {
            stack.clear();
            stack.reserve(local_stack_size);
        });

        // In both single threaded and multi threaded, we avoid allocations by double buffering
        // between two `Parallel` thread locals. One acts as the current stack that can be consumed
        // from. and the other is the next iteration's stack that can be pushed to.
        if stack_size < 1024 {
            // Single threaded when the stack is small
            let mut next_stack = next_stack.borrow_local_mut();
            next_stack.reserve(stack_size);
            let mut nodes = transform_queries.p1();
            for parent in stack.iter_mut().flatten() {
                let children = children
                    .get(parent.entity)
                    .expect("Only entities with children are pushed onto the stack.");
                let mut nodes = nodes.iter_many_mut(children);
                while let Some(components) = nodes.fetch_next() {
                    compute_transform(&mut next_stack, Some(parent), components, &orphans);
                }
            }
        } else {
            // Multithreaded only when the stack is large
            //
            // The main idea of the parallel traversal algorithm is to go wide over all entities of
            // a given hierarchy level. Starting from the root, calculate the global transform, then
            // work towards descendents, processing all entities at a given level per loop
            // iteration. Because no entities at the same hierarchy level can depend on each other,
            // we can parallelize across all these entities without any bookkeeping.
            let nodes = transform_queries.p1();

            let mut chunks = stack
                .iter_mut()
                // Break up the chunks to be smaller than the desired size, so that we are more
                // likely to be able to distribute work evenly across all threads
                .flat_map(|nodes| nodes.chunks(local_stack_size / 4));
            let orphans = &orphans;

            let compute = |chunks: &[&[PropagationNode]]| {
                // let _span = info_span!("Par").entered();
                let mut next_stack = next_stack.borrow_local_mut();
                for parent in chunks.iter().flat_map(|s| s.iter()) {
                    let children = children
                        .get(parent.entity)
                        .expect("Only entities with children are pushed onto the stack.");
                    // SAFETY:
                    //
                    // Each iteration of the outer loop visits entities on the same number
                    // of steps from the root of the hierarchy in a breadth-first search.
                    // The entity hierarchy forms a tree. Consequently, mutating all
                    // children at the same level of the tree cannot alias.
                    #[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
                    unsafe {
                        let mut nodes = nodes.iter_many_unsafe(children);
                        while let Some(components) = nodes.fetch_next() {
                            compute_transform(&mut next_stack, Some(parent), components, orphans);
                        }
                    }
                }
            };

            task_pool.scope(|scope| {
                while let Some(chunk) = chunks.next() {
                    let mut composite_chunks = smallvec::SmallVec::<[_; 8]>::new();
                    composite_chunks.push(chunk);
                    if chunk.len() < local_stack_size {
                        for chunk in chunks.by_ref() {
                            composite_chunks.push(chunk);
                            let total_len: usize = composite_chunks.iter().map(|c| c.len()).sum();
                            if total_len >= local_stack_size {
                                break;
                            }
                        }
                    }
                    scope.spawn(async move { compute(composite_chunks.as_slice()) });
                }
            });
        }

        // Double buffering: swap the next stack and the current stack to reuse allocated memory.
        core::mem::swap(&mut *stack, &mut *next_stack);
    }
}

#[inline]
fn compute_transform(
    thread_local_queue: &mut impl DerefMut<Target = Vec<PropagationNode>>,
    parent: Option<&PropagationNode>,
    (entity, transform, mut global_transform, has_children): (
        Entity,
        Ref<Transform>,
        Mut<GlobalTransform>,
        bool,
    ),
    orphans: &[Entity],
) {
    let is_changed = parent.map(|s| s.is_changed).unwrap_or_default()
        || transform.is_changed()
        || global_transform.is_added()
        // Only check if orphaned if it has no parent; avoids the search when not needed
        || parent.is_none() && orphans.binary_search(&entity).is_ok();

    if is_changed {
        *global_transform = if let Some(parent) = parent {
            parent.global_transform.mul_transform(*transform)
        } else {
            GlobalTransform::from(*transform)
        }
    }

    if has_children {
        thread_local_queue.push(PropagationNode {
            is_changed,
            entity,
            global_transform: *(global_transform.as_ref()),
        });
    }
}

/// Update [`GlobalTransform`] component of entities based on entity hierarchy and
/// [`Transform`] component.
///
/// Third party plugins should ensure that this is used in concert with [`sync_simple_transforms`].
pub fn propagate_transforms(
    mut root_query: Query<
        (Entity, &Children, Ref<Transform>, &mut GlobalTransform),
        Without<ChildOf>,
    >,
    mut orphaned: RemovedComponents<ChildOf>,
    transform_query: Query<
        (Ref<Transform>, &mut GlobalTransform, Option<&Children>),
        With<ChildOf>,
    >,
    parent_query: Query<(Entity, Ref<ChildOf>), With<GlobalTransform>>,
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
                #[expect(unsafe_code, reason = "`propagate_recursive()` is unsafe due to its use of `Query::get_unchecked()`.")]
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
#[expect(
    unsafe_code,
    reason = "This function uses `Query::get_unchecked()`, which can result in multiple mutable references if the preconditions are not met."
)]
unsafe fn propagate_recursive(
    parent: &GlobalTransform,
    transform_query: &Query<
        (Ref<Transform>, &mut GlobalTransform, Option<&Children>),
        With<ChildOf>,
    >,
    parent_query: &Query<(Entity, Ref<ChildOf>), With<GlobalTransform>>,
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
            // D has two parents, B and C. If the propagation passes through C, but the ChildOf component on D points to B,
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
        (global_transform, children)
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
                global_matrix.as_ref(),
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
    use alloc::vec;
    use bevy_app::prelude::*;
    use bevy_ecs::{prelude::*, world::CommandQueue};
    use bevy_math::{vec3, Vec3};
    use bevy_tasks::{ComputeTaskPool, TaskPool};

    use crate::systems::*;

    #[test]
    fn correct_parent_removed() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut world = World::default();
        let offset_global_transform =
            |offset| GlobalTransform::from(Transform::from_xyz(offset, offset, offset));
        let offset_transform = |offset| Transform::from_xyz(offset, offset, offset);

        let mut schedule = Schedule::default();
        schedule.add_systems((sync_simple_transforms, propagate_transforms));

        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, &world);
        let root = commands.spawn(offset_transform(3.3)).id();
        let parent = commands.spawn(offset_transform(4.4)).id();
        let child = commands.spawn(offset_transform(5.5)).id();
        commands.entity(parent).insert(ChildOf(root));
        commands.entity(child).insert(ChildOf(parent));
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
        commands.entity(parent).remove::<ChildOf>();
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
        commands.entity(child).remove::<ChildOf>();
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
        world.spawn(Transform::from_xyz(1.0, 0.0, 0.0));

        let mut children = Vec::new();
        world
            .spawn(Transform::from_xyz(1.0, 0.0, 0.0))
            .with_children(|parent| {
                children.push(parent.spawn(Transform::from_xyz(0.0, 2.0, 0.)).id());
                children.push(parent.spawn(Transform::from_xyz(0.0, 0.0, 3.)).id());
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
            .spawn(Transform::from_xyz(1.0, 0.0, 0.0))
            .with_children(|parent| {
                children.push(parent.spawn(Transform::from_xyz(0.0, 2.0, 0.0)).id());
                children.push(parent.spawn(Transform::from_xyz(0.0, 0.0, 3.0)).id());
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
                .collect::<Vec<_>>(),
            vec![children[1]]
        );

        assert_eq!(
            world
                .get::<Children>(children[1])
                .unwrap()
                .iter()
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
            .spawn(Transform::from_translation(translation))
            .with_children(|builder| {
                child = builder
                    .spawn(Transform::IDENTITY)
                    .with_children(|builder| {
                        grandchild = builder.spawn(Transform::IDENTITY).id();
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
        // We cannot directly edit ChildOf and Children, so we use a temp world to break
        // the hierarchy's invariants.
        let mut temp = World::new();
        let mut app = App::new();

        app.add_systems(Update, (propagate_transforms, sync_simple_transforms));

        fn setup_world(world: &mut World) -> (Entity, Entity) {
            let mut grandchild = Entity::from_raw(0);
            let child = world
                .spawn(Transform::IDENTITY)
                .with_children(|builder| {
                    grandchild = builder.spawn(Transform::IDENTITY).id();
                })
                .id();
            (child, grandchild)
        }

        let (temp_child, temp_grandchild) = setup_world(&mut temp);
        let (child, grandchild) = setup_world(app.world_mut());

        assert_eq!(temp_child, child);
        assert_eq!(temp_grandchild, grandchild);

        app.world_mut()
            .spawn(Transform::IDENTITY)
            .add_children(&[child]);
        core::mem::swap(
            #[expect(
                unsafe_code,
                reason = "ChildOf is not mutable but this is for a test to produce a scenario that cannot happen"
            )]
            // SAFETY: ChildOf is not mutable but this is for a test to produce a scenario that cannot happen
            unsafe {
                &mut *app
                    .world_mut()
                    .entity_mut(child)
                    .get_mut_assume_mutable::<ChildOf>()
                    .unwrap()
            },
            // SAFETY: ChildOf is not mutable but this is for a test to produce a scenario that cannot happen
            #[expect(
                unsafe_code,
                reason = "ChildOf is not mutable but this is for a test to produce a scenario that cannot happen"
            )]
            unsafe {
                &mut *temp
                    .entity_mut(grandchild)
                    .get_mut_assume_mutable::<ChildOf>()
                    .unwrap()
            },
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

        // Spawn a `Transform` entity with a local translation of `Vec3::ONE`
        let mut spawn_transform_bundle =
            || world.spawn(Transform::from_translation(translation)).id();

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
        world.entity_mut(child).remove::<ChildOf>();
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
