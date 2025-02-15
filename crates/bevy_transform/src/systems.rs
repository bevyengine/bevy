use core::sync::atomic::{AtomicI32, Ordering};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};

use crate::components::{GlobalTransform, Transform};
use alloc::vec::Vec;
use bevy_ecs::{entity::UniqueEntityIter, prelude::*, system::lifetimeless::Read};
use bevy_tasks::{ComputeTaskPool, TaskPool};
use bevy_utils::Parallel;

/// Update [`GlobalTransform`] component of entities that aren't in the hierarchy
///
/// Third party plugins should ensure that this is used in concert with
/// [`propagate_parent_transforms`] and [`compute_transform_leaves`].
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

/// A queue shared between threads for transform propagation.
pub struct WorkQueue {
    /// A semaphore that tracks how many threads are busy doing work. Used to determine when there
    /// is no more work to do.
    busy_threads: AtomicI32,
    sender: Sender<Vec<Entity>>,
    receiver: Arc<Mutex<Receiver<Vec<Entity>>>>,
    local_queue: Parallel<Vec<Entity>>,
}
impl Default for WorkQueue {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            busy_threads: AtomicI32::default(),
            sender: tx,
            receiver: Arc::new(Mutex::new(rx)),
            local_queue: Default::default(),
        }
    }
}
impl WorkQueue {
    const TASK_CHUNK_SIZE: usize = 1024;

    #[inline]
    fn send_batches_with(sender: &Sender<Vec<Entity>>, outbox: &mut Vec<Entity>) {
        for chunk in outbox
            .chunks(Self::TASK_CHUNK_SIZE)
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
        // Iterate over the locals to send batched tasks, avoiding the need to drain the locals into
        // a larger allocation.
        local_queue
            .iter_mut()
            .for_each(|outbox| Self::send_batches_with(sender, outbox));
    }
}

/// Alias for a large, repeatedly used query.
type NodeQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Ref<'static, Transform>,
        Mut<'static, GlobalTransform>,
        Read<Children>,
        Read<ChildOf>,
    ),
>;

/// Computes the [`GlobalTransform`]s of non-leaf nodes in the entity hierarchy, propagating
/// [`Transform`]s of parents to their children.
pub fn propagate_parent_transforms(
    mut queue: Local<WorkQueue>,
    mut orphaned: RemovedComponents<ChildOf>,
    mut orphans: Local<Vec<Entity>>,
    mut roots: Query<(Entity, Ref<Transform>, &mut GlobalTransform, &Children), Without<ChildOf>>,
    nodes: NodeQuery,
) {
    // Orphans
    orphans.clear();
    orphans.extend(orphaned.read());
    orphans.sort_unstable();

    // Process roots in parallel, seeding the work queue
    roots.par_iter_mut().for_each_init(
        || queue.local_queue.borrow_local_mut(),
        |outbox, (parent, transform, mut parent_transform, children)| {
            if transform.is_changed()
                || parent_transform.is_added()
                || orphans.binary_search(&parent).is_ok()
            {
                *parent_transform = GlobalTransform::from(*transform);
            }

            // SAFETY: the parent entities passed into this function are taken from iterating over
            // the root entity query. Queries always iterate over unique entities, preventing
            // mutable aliasing, and making this call safe.
            #[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
            unsafe {
                propagate_to_child_unchecked((parent, parent_transform, children), &nodes, outbox);
            }
        },
    );
    // Send all tasks in thread local outboxes *after* roots are processed to reduce the total
    // number of channel sends by avoiding sending partial batches.
    queue.send_batches();

    // Spawn workers on the task pool to recursively propagate the hierarchy in parallel.
    let task_pool = ComputeTaskPool::get_or_init(TaskPool::default);
    task_pool.scope(|s| {
        (0..task_pool.thread_num())
            .for_each(|_| s.spawn(async { propagation_worker(&queue, &nodes) }));
    });
}

/// A parallel worker that will consume processed parent entities from the queue, and push children
/// to the queue once it has propagated their [`GlobalTransform`].
#[inline]
fn propagation_worker(queue: &WorkQueue, nodes: &NodeQuery) {
    let _span = bevy_log::info_span!("transform propagation worker").entered();
    let mut outbox = queue.local_queue.borrow_local_mut();
    loop {
        // Try to acquire a lock on the work queue in a tight loop. Profiling shows this is much
        // more efficient than relying on `.lock()`, which causes gaps to form between tasks.
        let Ok(rx) = queue.receiver.try_lock() else {
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

        // At this point, we know there is work to do, so we increment the busy thread counter,
        // and drop the mutex guard *after* we have incremented the counter. This ensures that
        // if another thread is able to acquire a lock, the busy thread counter will already be
        // incremented.
        queue.busy_threads.fetch_add(1, Ordering::Relaxed);
        drop(rx); // Important: drop after atomic and before work starts.

        'task_loop: for mut parent in tasks.drain(..) {
            // SAFETY: Parent entities fed into this function are pulled from the work queue, which
            // is in turn fed from this function. The function will panic if cycles are found in the
            // hierarchy, which means we can be certain that the work queue itself contains unique
            // entities, making this safe to call.
            #[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
            unsafe {
                let (_, _, mut parent_g_transform, mut children, _) =
                    nodes.get_unchecked(parent).unwrap();

                // Optimization: when there is a single child, we want to recurse the hierarchy
                // sequentially until we find an entity with multiple children, at which point we
                // continue with the parallel task generation. If we don't do this, long chains of
                // single entities can become very slow, as they require starting a new task for
                // each level of the hierarchy. When there are many levels, this overhead hurts.
                while children.len() == 1 {
                    let child = *children.first().expect("Length should equal 1");
                    let Ok((_, child_transform, mut child_g_transform, grandchildren, child_of)) =
                        nodes.get_unchecked(child)
                    else {
                        continue 'task_loop;
                    };
                    assert!(child_of.get() == parent); // Safety: ensure no hierarchy cycles
                    if parent_g_transform.is_changed()
                        || child_transform.is_changed()
                        || child_g_transform.is_added()
                    {
                        *child_g_transform = parent_g_transform.mul_transform(*child_transform);
                    }
                    parent = child;
                    children = grandchildren;
                    parent_g_transform = child_g_transform;
                }

                propagate_to_child_unchecked(
                    (parent, parent_g_transform, children),
                    nodes,
                    &mut outbox,
                );
            }
            // Send chunks from inside the loop as well as at the end. This allows other workers to
            // pick up work while this task is still running. Only do this within the loop when the
            // outbox has grown large enough.
            if outbox.len() >= WorkQueue::TASK_CHUNK_SIZE {
                WorkQueue::send_batches_with(&queue.sender, &mut outbox);
            }
        }
        WorkQueue::send_batches_with(&queue.sender, &mut outbox);
        queue.busy_threads.fetch_add(-1, Ordering::Relaxed);
    }
}

/// Propagate transforms from `parent` to its non-leaf `children`, pushing updated child entities to
/// the `outbox`. Propagation does not visit leaf nodes; instead, they are computed in
/// [`compute_transform_leaves`], which can optimize much more efficiently.
///
/// # Safety
///
/// Callers must ensure that concurrent calls to this function are given unique `parent` entities.
/// Calling this function concurrently with the same `parent` is unsafe. This function will validate
/// that the entity hierarchy does not contain cycles to prevent mutable aliasing during
/// propagation, but it is unable to verify that it isn't being used to mutably alias the same
/// entity.
///
/// # Panics
///
/// Panics if the parent of a node is not the same as the supplied `parent`. This check can be used
/// to call this function safely.
///
/// If this function is only called when traversing from ancestors to descendant, using the entities
/// returned from te `outbox`, it can be used safely in parallel. This function will internally
/// panic if a cycle is found in the hierarchy to prevent soundness issues.
#[inline]
#[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
unsafe fn propagate_to_child_unchecked(
    (parent, parent_transform, children): (Entity, Mut<GlobalTransform>, &Children),
    nodes: &NodeQuery,
    outbox: &mut Vec<Entity>,
) {
    // Safety: `Children` is guaranteed to hold unique entities.
    #[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
    let children_iter = unsafe { UniqueEntityIter::from_iterator_unchecked(children.iter()) };
    // Performance note: iter_many tests every child to see if it meets the query. For leaf nodes,
    // this unfortunately means we have the pay the price of checking every child, even if it is a
    // leaf node and is skipped.
    //
    // To ensure this is still the fastest design, I tried removing the second pass
    // (`compute_transform_leaves`) and instead simply doing that here. However, that proved to be
    // much slower than two pass for a few reasons:
    // - it's less cache friendly and is outright slower than the tight loop in the second pass
    // - it prevents parallelism, as all children must be iterated in series
    //
    // The only way I can see to make this faster when there are many leaf nodes is to speed up
    // archetype checking to make the iterator skip leaf entities more quickly.
    for (child, transform, mut global_transform, _, child_of) in
        // Safety: traversing the entity tree from the roots, we assert that the childof and
        // children pointers match in both directions (see assert below) to ensure the hierarchy
        // does not have any cycles. Because the hierarchy does not have cycles, we know we are
        // visiting disjoint entities in parallel, which is safe.
        unsafe { nodes.iter_many_unique_unsafe(children_iter) }
    {
        assert!(child_of.get() == parent);
        if parent_transform.is_changed() || transform.is_changed() || global_transform.is_added() {
            *global_transform = parent_transform.mul_transform(*transform);
        }
        outbox.push(child);
    }
}

/// Compute leaf [`GlobalTransform`]s in parallel.
///
/// This is run after [`propagate_parent_transforms`], to ensure the parents' [`GlobalTransform`]s
/// have been computed. This makes computing leaves embarrassingly parallel.
pub fn compute_transform_leaves(
    parents: Query<Ref<GlobalTransform>, With<Children>>,
    mut leaves: Query<(Ref<Transform>, &mut GlobalTransform, &ChildOf), Without<Children>>,
) {
    leaves
        .par_iter_mut()
        .for_each(|(transform, mut global_transform, parent)| {
            let Ok(parent_transform) = parents.get(parent.get()) else {
                return;
            };
            if parent_transform.is_changed()
                || transform.is_changed()
                || global_transform.is_added()
            {
                *global_transform = parent_transform.mul_transform(*transform);
            }
        });
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
        schedule.add_systems(
            (
                sync_simple_transforms,
                propagate_parent_transforms,
                compute_transform_leaves,
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
                sync_simple_transforms,
                propagate_parent_transforms,
                compute_transform_leaves,
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
                sync_simple_transforms,
                propagate_parent_transforms,
                compute_transform_leaves,
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
                sync_simple_transforms,
                propagate_parent_transforms,
                compute_transform_leaves,
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
                sync_simple_transforms,
                propagate_parent_transforms,
                compute_transform_leaves,
            )
                .chain(),
        );

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

        app.add_systems(
            Update,
            (
                propagate_parent_transforms,
                sync_simple_transforms,
                compute_transform_leaves,
            )
                .chain(),
        );

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
        schedule.add_systems((
            sync_simple_transforms,
            propagate_parent_transforms,
            compute_transform_leaves,
        ));

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
