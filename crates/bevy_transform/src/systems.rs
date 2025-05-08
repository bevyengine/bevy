use crate::components::{GlobalTransform, Transform, TransformTreeChanged};
use bevy_ecs::prelude::*;
#[cfg(feature = "std")]
pub use parallel::propagate_parent_transforms;
#[cfg(not(feature = "std"))]
pub use serial::propagate_parent_transforms;

/// Update [`GlobalTransform`] component of entities that aren't in the hierarchy
///
/// Third party plugins should ensure that this is used in concert with
/// [`propagate_parent_transforms`] and [`mark_dirty_trees`].
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

/// Optimization for static scenes. Propagates a "dirty bit" up the hierarchy towards ancestors.
/// Transform propagation can ignore entire subtrees of the hierarchy if it encounters an entity
/// without the dirty bit.
pub fn mark_dirty_trees(
    changed_transforms: Query<
        Entity,
        Or<(Changed<Transform>, Changed<ChildOf>, Added<GlobalTransform>)>,
    >,
    mut orphaned: RemovedComponents<ChildOf>,
    mut transforms: Query<(Option<&ChildOf>, &mut TransformTreeChanged)>,
) {
    for entity in changed_transforms.iter().chain(orphaned.read()) {
        let mut next = entity;
        while let Ok((child_of, mut tree)) = transforms.get_mut(next) {
            if tree.is_changed() && !tree.is_added() {
                // If the component was changed, this part of the tree has already been processed.
                // Ignore this if the change was caused by the component being added.
                break;
            }
            tree.set_changed();
            if let Some(parent) = child_of.map(ChildOf::parent) {
                next = parent;
            } else {
                break;
            };
        }
    }
}

// TODO: This serial implementation isn't actually serial, it parallelizes across the roots.
// Additionally, this couples "no_std" with "single_threaded" when these two features should be
// independent.
//
// What we want to do in a future refactor is take the current "single threaded" implementation, and
// actually make it single threaded. This will remove any overhead associated with working on a task
// pool when you only have a single thread, and will have the benefit of removing the need for any
// unsafe. We would then make the multithreaded implementation work across std and no_std, but this
// is blocked a no_std compatible Channel, which is why this TODO is not yet implemented.
//
// This complexity might also not be needed. If the multithreaded implementation on a single thread
// is as fast as the single threaded implementation, we could simply remove the entire serial
// module, and make the multithreaded module no_std compatible.
//
/// Serial hierarchy traversal. Useful in `no_std` or single threaded contexts.
#[cfg(not(feature = "std"))]
mod serial {
    use crate::prelude::*;
    use alloc::vec::Vec;
    use bevy_ecs::prelude::*;

    /// Update [`GlobalTransform`] component of entities based on entity hierarchy and [`Transform`]
    /// component.
    ///
    /// Third party plugins should ensure that this is used in concert with
    /// [`sync_simple_transforms`](super::sync_simple_transforms) and
    /// [`mark_dirty_trees`](super::mark_dirty_trees).
    pub fn propagate_parent_transforms(
        mut root_query: Query<
            (Entity, &Children, Ref<Transform>, &mut GlobalTransform),
            Without<ChildOf>,
        >,
        mut orphaned: RemovedComponents<ChildOf>,
        transform_query: Query<
            (Ref<Transform>, &mut GlobalTransform, Option<&Children>),
            With<ChildOf>,
        >,
        child_query: Query<(Entity, Ref<ChildOf>), With<GlobalTransform>>,
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

            for (child, child_of) in child_query.iter_many(children) {
                assert_eq!(
                    child_of.parent(), entity,
                    "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
                );
                // SAFETY:
                // - `child` must have consistent parentage, or the above assertion would panic.
                //   Since `child` is parented to a root entity, the entire hierarchy leading to it
                //   is consistent.
                // - We may operate as if all descendants are consistent, since
                //   `propagate_recursive` will panic before continuing to propagate if it
                //   encounters an entity with inconsistent parentage.
                // - Since each root entity is unique and the hierarchy is consistent and
                //   forest-like, other root entities' `propagate_recursive` calls will not conflict
                //   with this one.
                // - Since this is the only place where `transform_query` gets used, there will be
                //   no conflicting fetches elsewhere.
                #[expect(unsafe_code, reason = "`propagate_recursive()` is unsafe due to its use of `Query::get_unchecked()`.")]
                unsafe {
                    propagate_recursive(
                        &global_transform,
                        &transform_query,
                        &child_query,
                        child,
                        changed || child_of.is_changed(),
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
    /// If `entity`'s descendants have a malformed hierarchy, this function will panic occur before
    /// propagating the transforms of any malformed entities and their descendants.
    ///
    /// # Safety
    ///
    /// - While this function is running, `transform_query` must not have any fetches for `entity`,
    ///   nor any of its descendants.
    /// - The caller must ensure that the hierarchy leading to `entity` is well-formed and must
    ///   remain as a tree or a forest. Each entity must have at most one parent.
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
        child_query: &Query<(Entity, Ref<ChildOf>), With<GlobalTransform>>,
        entity: Entity,
        mut changed: bool,
    ) {
        let (global_matrix, children) = {
            let Ok((transform, mut global_transform, children)) =
            // SAFETY: This call cannot create aliased mutable references.
            //   - The top level iteration parallelizes on the roots of the hierarchy.
            //   - The caller ensures that each child has one and only one unique parent throughout
            //     the entire hierarchy.
            //
            // For example, consider the following malformed hierarchy:
            //
            //     A
            //   /   \
            //  B     C
            //   \   /
            //     D
            //
            // D has two parents, B and C. If the propagation passes through C, but the ChildOf
            // component on D points to B, the above check will panic as the origin parent does
            // match the recorded parent.
            //
            // Also consider the following case, where A and B are roots:
            //
            //  A       B
            //   \     /
            //    C   D
            //     \ /
            //      E
            //
            // Even if these A and B start two separate tasks running in parallel, one of them will
            // panic before attempting to mutably access E.
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
        for (child, child_of) in child_query.iter_many(children) {
            assert_eq!(
            child_of.parent(), entity,
            "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );
            // SAFETY: The caller guarantees that `transform_query` will not be fetched for any
            // descendants of `entity`, so it is safe to call `propagate_recursive` for each child.
            //
            // The above assertion ensures that each child has one and only one unique parent
            // throughout the entire hierarchy.
            unsafe {
                propagate_recursive(
                    global_matrix.as_ref(),
                    transform_query,
                    child_query,
                    child,
                    changed || child_of.is_changed(),
                );
            }
        }
    }
}

// TODO: Relies on `std` until a `no_std` `mpsc` channel is available.
//
/// Parallel hierarchy traversal with a batched work sharing scheduler. Often 2-5 times faster than
/// the serial version.
#[cfg(feature = "std")]
mod parallel {
    use crate::prelude::*;
    // TODO: this implementation could be used in no_std if there are equivalents of these.
    use alloc::{sync::Arc, vec::Vec};
    use bevy_ecs::{entity::UniqueEntityIter, prelude::*, system::lifetimeless::Read};
    use bevy_tasks::{ComputeTaskPool, TaskPool};
    use bevy_utils::Parallel;
    use core::sync::atomic::{AtomicI32, Ordering};
    use std::sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    };

    /// Update [`GlobalTransform`] component of entities based on entity hierarchy and [`Transform`]
    /// component.
    ///
    /// Third party plugins should ensure that this is used in concert with
    /// [`sync_simple_transforms`](super::sync_simple_transforms) and
    /// [`mark_dirty_trees`](super::mark_dirty_trees).
    pub fn propagate_parent_transforms(
        mut queue: Local<WorkQueue>,
        mut roots: Query<
            (Entity, Ref<Transform>, &mut GlobalTransform, &Children),
            (Without<ChildOf>, Changed<TransformTreeChanged>),
        >,
        nodes: NodeQuery,
    ) {
        // Process roots in parallel, seeding the work queue
        roots.par_iter_mut().for_each_init(
            || queue.local_queue.borrow_local_mut(),
            |outbox, (parent, transform, mut parent_transform, children)| {
                *parent_transform = GlobalTransform::from(*transform);

                // SAFETY: the parent entities passed into this function are taken from iterating
                // over the root entity query. Queries iterate over disjoint entities, preventing
                // mutable aliasing, and making this call safe.
                #[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
                unsafe {
                    propagate_descendants_unchecked(
                        parent,
                        parent_transform,
                        children,
                        &nodes,
                        outbox,
                        &queue,
                        // Need to revisit this single-max-depth by profiling more representative
                        // scenes. It's possible that it is actually beneficial to go deep into the
                        // hierarchy to build up a good task queue before starting the workers.
                        // However, we avoid this for now to prevent cases where only a single
                        // thread is going deep into the hierarchy while the others sit idle, which
                        // is the problem that the tasks sharing workers already solve.
                        1,
                    );
                }
            },
        );
        // Send all tasks in thread local outboxes *after* roots are processed to reduce the total
        // number of channel sends by avoiding sending partial batches.
        queue.send_batches();

        if let Ok(rx) = queue.receiver.try_lock() {
            if let Some(task) = rx.try_iter().next() {
                // This is a bit silly, but the only way to see if there is any work is to grab a
                // task. Peeking will remove the task even if you don't call `next`, resulting in
                // dropping a task. What we do here is grab the first task if there is one, then
                // immediately send it to the back of the queue.
                queue.sender.send(task).ok();
            } else {
                return; // No work, don't bother spawning any tasks
            }
        }

        // Spawn workers on the task pool to recursively propagate the hierarchy in parallel.
        let task_pool = ComputeTaskPool::get_or_init(TaskPool::default);
        task_pool.scope(|s| {
            (1..task_pool.thread_num()) // First worker is run locally instead of the task pool.
                .for_each(|_| s.spawn(async { propagation_worker(&queue, &nodes) }));
            propagation_worker(&queue, &nodes);
        });
    }

    /// A parallel worker that will consume processed parent entities from the queue, and push
    /// children to the queue once it has propagated their [`GlobalTransform`].
    #[inline]
    fn propagation_worker(queue: &WorkQueue, nodes: &NodeQuery) {
        #[cfg(feature = "std")]
        let _span = bevy_log::info_span!("transform propagation worker").entered();

        let mut outbox = queue.local_queue.borrow_local_mut();
        loop {
            // Try to acquire a lock on the work queue in a tight loop. Profiling shows this is much
            // more efficient than relying on `.lock()`, which causes gaps to form between tasks.
            let Ok(rx) = queue.receiver.try_lock() else {
                core::hint::spin_loop(); // No apparent impact on profiles, but best practice.
                continue;
            };
            // If the queue is empty and no other threads are busy processing work, we can conclude
            // there is no more work to do, and end the task by exiting the loop.
            let Some(mut tasks) = rx.try_iter().next() else {
                if queue.busy_threads.load(Ordering::Relaxed) == 0 {
                    break; // All work is complete, kill the worker
                }
                continue; // No work to do now, but another thread is busy creating more work.
            };
            if tasks.is_empty() {
                continue; // This shouldn't happen, but if it does, we might as well stop early.
            }

            // If the task queue is extremely short, it's worthwhile to gather a few more tasks to
            // reduce the amount of thread synchronization needed once this very short task is
            // complete.
            while tasks.len() < WorkQueue::CHUNK_SIZE / 2 {
                let Some(mut extra_task) = rx.try_iter().next() else {
                    break;
                };
                tasks.append(&mut extra_task);
            }

            // At this point, we know there is work to do, so we increment the busy thread counter,
            // and drop the mutex guard *after* we have incremented the counter. This ensures that
            // if another thread is able to acquire a lock, the busy thread counter will already be
            // incremented.
            queue.busy_threads.fetch_add(1, Ordering::Relaxed);
            drop(rx); // Important: drop after atomic and before work starts.

            for parent in tasks.drain(..) {
                // SAFETY: each task pushed to the worker queue represents an unprocessed subtree of
                // the hierarchy, guaranteeing unique access.
                #[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
                unsafe {
                    let (_, (_, p_global_transform, _), (p_children, _)) =
                        nodes.get_unchecked(parent).unwrap();
                    propagate_descendants_unchecked(
                        parent,
                        p_global_transform,
                        p_children.unwrap(), // All entities in the queue should have children
                        nodes,
                        &mut outbox,
                        queue,
                        // Only affects performance. Trees deeper than this will still be fully
                        // propagated, but the work will be broken into multiple tasks. This number
                        // was chosen to be larger than any reasonable tree depth, while not being
                        // so large the function could hang on a deep hierarchy.
                        10_000,
                    );
                }
            }
            WorkQueue::send_batches_with(&queue.sender, &mut outbox);
            queue.busy_threads.fetch_add(-1, Ordering::Relaxed);
        }
    }

    /// Propagate transforms from `parent` to its `children`, pushing updated child entities to the
    /// `outbox`. This function will continue propagating transforms to descendants in a depth-first
    /// traversal, while simultaneously pushing unvisited branches to the outbox, for other threads
    /// to take when idle.
    ///
    /// # Safety
    ///
    /// Callers must ensure that concurrent calls to this function are given unique `parent`
    /// entities. Calling this function concurrently with the same `parent` is unsound. This
    /// function will validate that the entity hierarchy does not contain cycles to prevent mutable
    /// aliasing during propagation, but it is unable to verify that it isn't being used to mutably
    /// alias the same entity.
    ///
    /// ## Panics
    ///
    /// Panics if the parent of a child node is not the same as the supplied `parent`. This
    /// assertion ensures that the hierarchy is acyclic, which in turn ensures that if the caller is
    /// following the supplied safety rules, multi-threaded propagation is sound.
    #[inline]
    #[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
    unsafe fn propagate_descendants_unchecked(
        parent: Entity,
        p_global_transform: Mut<GlobalTransform>,
        p_children: &Children,
        nodes: &NodeQuery,
        outbox: &mut Vec<Entity>,
        queue: &WorkQueue,
        max_depth: usize,
    ) {
        // Create mutable copies of the input variables, used for iterative depth-first traversal.
        let (mut parent, mut p_global_transform, mut p_children) =
            (parent, p_global_transform, p_children);

        // See the optimization note at the end to understand why this loop is here.
        for depth in 1..=max_depth {
            // Safety: traversing the entity tree from the roots, we assert that the childof and
            // children pointers match in both directions (see assert below) to ensure the hierarchy
            // does not have any cycles. Because the hierarchy does not have cycles, we know we are
            // visiting disjoint entities in parallel, which is safe.
            #[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
            let children_iter = unsafe {
                nodes.iter_many_unique_unsafe(UniqueEntityIter::from_iterator_unchecked(
                    p_children.iter(),
                ))
            };

            let mut last_child = None;
            let new_children = children_iter.filter_map(
                |(child, (transform, mut global_transform, tree), (children, child_of))| {
                    if !tree.is_changed() && !p_global_transform.is_changed() {
                        // Static scene optimization
                        return None;
                    }
                    assert_eq!(child_of.parent(), parent);

                    // Transform prop is expensive - this helps avoid updating entire subtrees if
                    // the GlobalTransform is unchanged, at the cost of an added equality check.
                    global_transform.set_if_neq(p_global_transform.mul_transform(*transform));

                    children.map(|children| {
                        // Only continue propagation if the entity has children.
                        last_child = Some((child, global_transform, children));
                        child
                    })
                },
            );
            outbox.extend(new_children);

            if depth >= max_depth || last_child.is_none() {
                break; // Don't remove anything from the outbox or send any chunks, just exit.
            }

            // Optimization: tasks should consume work locally as long as they can to avoid
            // thread synchronization for as long as possible.
            if let Some(last_child) = last_child {
                // Overwrite parent data with children, and loop to iterate through descendants.
                (parent, p_global_transform, p_children) = last_child;
                outbox.pop();

                // Send chunks during traversal. This allows sharing tasks with other threads before
                // fully completing the traversal.
                if outbox.len() >= WorkQueue::CHUNK_SIZE {
                    WorkQueue::send_batches_with(&queue.sender, outbox);
                }
            }
        }
    }

    /// Alias for a large, repeatedly used query. Queries for transform entities that have both a
    /// parent and possibly children, thus they are not roots.
    type NodeQuery<'w, 's> = Query<
        'w,
        's,
        (
            Entity,
            (
                Ref<'static, Transform>,
                Mut<'static, GlobalTransform>,
                Ref<'static, TransformTreeChanged>,
            ),
            (Option<Read<Children>>, Read<ChildOf>),
        ),
    >;

    /// A queue shared between threads for transform propagation.
    pub struct WorkQueue {
        /// A semaphore that tracks how many threads are busy doing work. Used to determine when
        /// there is no more work to do.
        busy_threads: AtomicI32,
        sender: Sender<Vec<Entity>>,
        receiver: Arc<Mutex<Receiver<Vec<Entity>>>>,
        local_queue: Parallel<Vec<Entity>>,
    }
    impl Default for WorkQueue {
        fn default() -> Self {
            let (tx, rx) = std::sync::mpsc::channel();
            Self {
                busy_threads: AtomicI32::default(),
                sender: tx,
                receiver: Arc::new(Mutex::new(rx)),
                local_queue: Default::default(),
            }
        }
    }
    impl WorkQueue {
        const CHUNK_SIZE: usize = 512;

        #[inline]
        fn send_batches_with(sender: &Sender<Vec<Entity>>, outbox: &mut Vec<Entity>) {
            for chunk in outbox
                .chunks(WorkQueue::CHUNK_SIZE)
                .filter(|c| !c.is_empty())
            {
                sender.send(chunk.to_vec()).ok();
            }
            outbox.clear();
        }

        #[inline]
        fn send_batches(&mut self) {
            let Self {
                sender,
                local_queue,
                ..
            } = self;
            // Iterate over the locals to send batched tasks, avoiding the need to drain the locals
            // into a larger allocation.
            local_queue
                .iter_mut()
                .for_each(|outbox| Self::send_batches_with(sender, outbox));
        }
    }
}

#[cfg(test)]
mod test {
    use alloc::{vec, vec::Vec};
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
        schedule.add_systems(
            (
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

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
        schedule.add_systems(
            (
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

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
        schedule.add_systems(
            (
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

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
        schedule.add_systems(
            (
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

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

        app.add_systems(
            Update,
            (
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

        let translation = vec3(1.0, 0.0, 0.0);

        // These will be overwritten.
        let mut child = Entity::from_raw_u32(0).unwrap();
        let mut grandchild = Entity::from_raw_u32(1).unwrap();
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
        // Note that at this point, the `GlobalTransform`s will not have updated yet, due to
        // `Commands` delay
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
        // We cannot directly edit ChildOf and Children, so we use a temp world to break the
        // hierarchy's invariants.
        let mut temp = World::new();
        let mut app = App::new();

        app.add_systems(
            Update,
            // It is unsound for this unsafe system to encounter a cycle without panicking. This
            // requirement only applies to systems with unsafe parallel traversal that result in
            // aliased mutability during a cycle.
            propagate_parent_transforms,
        );

        fn setup_world(world: &mut World) -> (Entity, Entity) {
            let mut grandchild = Entity::from_raw_u32(0).unwrap();
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

        let mut child_entity = app.world_mut().entity_mut(child);

        let mut grandchild_entity = temp.entity_mut(grandchild);

        #[expect(
            unsafe_code,
            reason = "ChildOf is not mutable but this is for a test to produce a scenario that cannot happen"
        )]
        // SAFETY: ChildOf is not mutable but this is for a test to produce a scenario that
        // cannot happen
        let mut a = unsafe { child_entity.get_mut_assume_mutable::<ChildOf>().unwrap() };

        // SAFETY: ChildOf is not mutable but this is for a test to produce a scenario that
        // cannot happen
        #[expect(
            unsafe_code,
            reason = "ChildOf is not mutable but this is for a test to produce a scenario that cannot happen"
        )]
        let mut b = unsafe {
            grandchild_entity
                .get_mut_assume_mutable::<ChildOf>()
                .unwrap()
        };

        core::mem::swap(a.as_mut(), b.as_mut());

        app.update();
    }

    #[test]
    fn global_transform_should_not_be_overwritten_after_reparenting() {
        let translation = Vec3::ONE;
        let mut world = World::new();

        // Create transform propagation schedule
        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                mark_dirty_trees,
                propagate_parent_transforms,
                sync_simple_transforms,
            )
                .chain(),
        );

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
