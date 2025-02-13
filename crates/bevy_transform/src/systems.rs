use core::sync::atomic::{AtomicI32, Ordering};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};

use crate::components::{GlobalTransform, Transform};
use alloc::vec::Vec;
use bevy_ecs::prelude::*;
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

/// Computes the [`GlobalTransform`]s of non-leaf nodes in the entity hierarchy, propagating
/// [`Transform`]s of parents to their children.
pub fn propagate_parent_transforms(
    queue: Local<WorkQueue>,
    mut orphaned: RemovedComponents<ChildOf>,
    mut orphans: Local<Vec<Entity>>,
    mut roots: Query<(Entity, Ref<Transform>, &mut GlobalTransform, &Children), Without<ChildOf>>,
    nodes: Query<(
        Entity,
        Ref<Transform>,
        &mut GlobalTransform,
        &Children,
        &ChildOf,
    )>,
) {
    // Orphans
    orphans.clear();
    orphans.extend(orphaned.read());
    orphans.sort_unstable();

    // Process roots in parallel, seeding the work queue
    roots
        .par_iter_mut()
        .for_each(|(parent, transform, mut parent_transform, children)| {
            if transform.is_changed()
                || parent_transform.is_added()
                || orphans.binary_search(&parent).is_ok()
            {
                *parent_transform = GlobalTransform::from(*transform);
            }

            // SAFETY: Visiting the hierarchy as a tree without cycles
            #[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
            unsafe {
                let mut outbox = Vec::new();
                propagate_to_child_unchecked(
                    (parent, parent_transform, children),
                    &nodes,
                    &mut outbox,
                );
                queue.sender.send(outbox).ok();
            }
        });

    // Spawn workers on the task pool to recursively propagate the hierarchy in parallel.
    let task_pool = ComputeTaskPool::get_or_init(TaskPool::default);
    task_pool.scope(|s| {
        (0..task_pool.thread_num())
            .for_each(|_| s.spawn(async { propagation_worker(&queue, &nodes) }));
    });
}

/// A parallel worker that will consume processed parent entities from the queue, and push children
/// to the queue once it has propagated their [`GlobalTransform`].
fn propagation_worker(
    queue: &WorkQueue,
    nodes: &Query<(
        Entity,
        Ref<'_, Transform>,
        &mut GlobalTransform,
        &Children,
        &ChildOf,
    )>,
) {
    let mut outbox = queue.local_queue.borrow_local_mut();
    loop {
        // Try to acquire a lock on the work queue in a tight loop.
        let Ok(rx) = queue.receiver.try_lock() else {
            continue;
        };
        // If the queue is empty and no other threads are busy processing work, we can conclude
        // there is no more work to do, and end the task by exiting the loop.
        let Some(mut tasks) = rx.try_iter().next() else {
            if queue.busy_threads.load(Ordering::Relaxed) == 0 {
                break; // All work is complete, kill the worker
            }
            continue; // No work to do now, but another thread is busy creating more work
        };
        if tasks.is_empty() {
            continue;
        }

        // At this point, we know there is work to do, so we increment the busy thread counter,
        // and drop the mutex guard *after* we have incremented the counter. This ensures that
        // if another thread is able to acquire a lock, the busy thread counter will already be
        // incremented.
        queue.busy_threads.fetch_add(1, Ordering::Relaxed);
        drop(rx);
        for parent in tasks.drain(..) {
            // SAFETY: Visiting the hierarchy as a tree without cycles. The assertion inside the
            // function will trigger if the hierarchy has a cycle
            #[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
            unsafe {
                let (_, _, transform, children, _) = nodes.get_unchecked(parent).unwrap();
                propagate_to_child_unchecked((parent, transform, children), nodes, &mut outbox);
            }
        }
        for chunk in outbox.chunks(1024) {
            queue.sender.send(chunk.to_vec()).ok();
        }
        outbox.clear();
        queue.busy_threads.fetch_add(-1, Ordering::Relaxed);
    }
}

/// Propagate transforms from `parent` to its non-leaf `children`, pushing updated child entities to
/// the `outbox`. Propagation does not visit leaf nodes as they are computed in a second parallel
/// pass.
///
/// # Panics
///
/// Panics if the parent of a node is not the same as the supplied `parent`. This check can be used
/// to call this function safely.
///
/// If this function is only called when traversing from ancestors to descendant, using the entities
/// returned from te `outbox`, it can be used safely in parallel. This function will internally
/// panic if a cycle is found in the hierarchy to prevent soundness issues.
///
/// # Safety
///
/// Callers of this function must ensure that if `nodes` is being used elsewhere concurrently, the
/// entities passed in to `children` are disjoint.
#[inline]
#[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
unsafe fn propagate_to_child_unchecked(
    (parent, parent_transform, children): (Entity, Mut<GlobalTransform>, &Children),
    nodes: &Query<(
        Entity,
        Ref<Transform>,
        &mut GlobalTransform,
        &Children,
        &ChildOf,
    )>,
    outbox: &mut Vec<Entity>,
) {
    // SAFETY: This function must only be called with disjoint entity access.
    #[expect(unsafe_code, reason = "Mutating disjoint entities in parallel")]
    let mut children = unsafe { nodes.iter_many_unsafe(children) };
    while let Some((child, transform, mut global_transform, _, child_of)) = children.fetch_next() {
        assert!(child_of.get() == parent);
        if parent_transform.is_changed() || transform.is_changed() || global_transform.is_added() {
            *global_transform = parent_transform.mul_transform(*transform);
        }
        outbox.push(child);
    }
}

/// Compute leaf [`GlobalTransform`]s in parallel.
///
/// This is run after [`propagate_transform_nodes`], to ensure the parents' [`GlobalTransform`]s
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
