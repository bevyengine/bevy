#![expect(
    unsafe_code,
    reason = "Transform propagation is very performance sensitive and unsafe code is needed to skip bound checks and access components in parallel"
)]

use core::cell::RefMut;

use crate::{
    bitset_storage::BitsetStorage,
    components::{GlobalTransform, Transform},
    helper::TransformHelper,
};

use alloc::vec::Vec;
use bevy_ecs::{
    prelude::*,
    query::QueryFilter,
    relationship::Relationship,
    storage::{Table, TableId, TableRow},
    system::SystemParam,
};

use bevy_tasks::{task_scope, ComputeTaskPool, TaskPool, TaskRunner};

/// Generic system that propagates transforms,
/// using [`TransformHelper`] for any entity matching the filter `F`.
/// Useful for moving and rendering in the same frame.
pub fn propagate_transforms_for<F: QueryFilter + 'static>(
    tf_helper: TransformHelper,
    mut query: Query<(Entity, &mut GlobalTransform), F>,
) {
    for (entity, mut gtf) in query.iter_mut() {
        let result = tf_helper
            .compute_global_transform(entity)
            .inspect_err(|_err| {
                #[cfg(feature = "trace")]
                bevy_utils::once!(tracing::warn!(
                    "Failed to compute GlobalTransform for entity {:?}: {:?}",
                    entity,
                    _err
                ));
            });

        if let Ok(computed) = result {
            *gtf = computed;
        }
    }
}

/// Update [`GlobalTransform`] component of entities that aren't in the hierarchy
///
/// Third party plugins should ensure that this is used in concert with
/// [`propagate_transforms`]
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
    #[cfg(feature = "multi_threaded")]
    query
        .p0()
        .par_iter_mut()
        .for_each(|(transform, mut global_transform)| {
            *global_transform = GlobalTransform::from(*transform);
        });
    #[cfg(not(feature = "multi_threaded"))]
    query
        .p0()
        .iter_mut()
        .for_each(|(transform, mut global_transform)| {
            *global_transform = GlobalTransform::from(*transform);
        });
    // Update orphaned entities.
    let mut query = query.p1();
    let mut iter = query.iter_many_mut(orphaned.read()).matched();
    while let Some((transform, mut global_transform)) = iter.fetch_next() {
        if !transform.is_changed() && !global_transform.is_added() {
            *global_transform = GlobalTransform::from(*transform);
        }
    }
}

/// Configure the behavior of static scene optimizations for [`Transform`] propagation.
///
/// For scenes with many static entities, it is much faster to track trees of unchanged
/// [`Transform`]s and skip these during the expensive transform propagation step. If your scene is
/// very dynamic, the cost of tracking these trees can exceed the performance benefits. By default,
/// static scene optimization is enabled.
#[derive(Clone, Copy, Resource, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum StaticTransformOptimizations {
    /// Enable static scene optimizations.
    #[default]
    Enabled,
    /// Disable static scene optimizations.
    Disabled,
}

impl StaticTransformOptimizations {
    /// Returns `true` if static scene optimizations are enabled.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        *self == StaticTransformOptimizations::Enabled
    }
}

#[derive(Default)]
struct PropagationSources(
    #[cfg(feature = "multi_threaded")] bevy_utils::Parallel<Vec<Entity>>,
    #[cfg(not(feature = "multi_threaded"))] core::cell::RefCell<Vec<Entity>>,
);

impl PropagationSources {
    fn borrow_local_mut(&self) -> RefMut<'_, Vec<Entity>> {
        #[cfg(feature = "multi_threaded")]
        {
            self.0.borrow_local_mut()
        }

        #[cfg(not(feature = "multi_threaded"))]
        {
            self.0.borrow_mut()
        }
    }

    fn drain(&mut self) -> impl IntoIterator<Item = Entity> {
        #[cfg(feature = "multi_threaded")]
        {
            self.0.iter_mut().flat_map(|v| v.drain(..))
        }

        #[cfg(not(feature = "multi_threaded"))]
        {
            self.0.get_mut().drain(..)
        }
    }
}

type BaseQueryFilter = (With<Transform>, With<GlobalTransform>);

#[derive(SystemParam)]
struct NonStaticRootParams<'w, 's> {
    root_query: Query<
        'w,
        's,
        (
            Entity,
            Ref<'static, Transform>,
            &'static mut GlobalTransform,
        ),
        (BaseQueryFilter, Without<ChildOf>, With<Children>),
    >,
    orphaned: RemovedComponents<'w, 's, ChildOf>,
    orphaned_entities: Local<'s, Vec<Entity>>,
}

/// System parameters for performing transform propagation.
#[derive(SystemParam)]
pub struct PropagateTransformsParams<'w, 's> {
    static_optimizations: Res<'w, StaticTransformOptimizations>,

    shared_params: ParamSet<
        'w,
        's,
        (
            MarkDirtySubtreesParams<'static, 'static>,
            NonStaticRootParams<'static, 'static>,
            PropagateTransformParams<'static, 'static>,
        ),
    >,

    source_markers: SourceMarkers<'s>,
    propagation_candidates: Local<'s, Vec<Entity>>,
    propagation_sources: Local<'s, PropagationSources>,
}

/// Optimization for static scenes.
///
/// Propagates a "dirty bit" up the hierarchy towards ancestors. Transform propagation can ignore
/// entire subtrees of the hierarchy if it encounters an entity without the dirty bit.
///
/// Configure behavior with [`StaticTransformOptimizations`].
pub fn propagate_transforms(params: PropagateTransformsParams) {
    let PropagateTransformsParams {
        static_optimizations,
        mut shared_params,
        mut source_markers,
        mut propagation_candidates,
        mut propagation_sources,
    } = params;

    if static_optimizations.is_enabled() {
        mark_dirty_subtrees(
            shared_params.p0(),
            &mut source_markers,
            &mut propagation_candidates,
            &mut propagation_sources,
        );
    } else {
        let mut root_params = shared_params.p1();

        root_params.orphaned_entities.clear();
        root_params
            .orphaned_entities
            .extend(root_params.orphaned.read());
        root_params.orphaned_entities.sort_unstable();

        type Item<'w> = (Entity, Ref<'w, Transform>, Mut<'w, GlobalTransform>);
        let handle = |propagation_sources: &mut RefMut<Vec<Entity>>,
                      (entity, transform, mut global_transform): Item| {
            let changed = transform.is_changed()
                || global_transform.is_added()
                || root_params.orphaned_entities.binary_search(&entity).is_ok();

            if changed {
                global_transform.set_if_neq(GlobalTransform::from(*transform));
            }

            propagation_sources.push(entity);
        };

        #[cfg(feature = "multi_threaded")]
        root_params
            .root_query
            .par_iter_mut()
            .for_each_init(|| propagation_sources.borrow_local_mut(), handle);

        #[cfg(not(feature = "multi_threaded"))]
        let mut propagation_sources = propagation_sources.borrow_local_mut();
        #[cfg(not(feature = "multi_threaded"))]
        root_params
            .root_query
            .iter_mut()
            .for_each(|item| handle(&mut propagation_sources, item));
    }

    propagate_transform_dirty_subtrees(
        shared_params.p2(),
        &source_markers,
        *static_optimizations,
        &mut propagation_sources,
    );
}

type TablesQueryData = (TableId, &'static Table, TableRow);

type ChangedQueryData = (TableId, TableRow, Entity);
type ChangedQueryFilter = Or<(Changed<Transform>, Changed<ChildOf>, Added<GlobalTransform>)>;
type ChangedQuery<'w, 's> = Query<'w, 's, ChangedQueryData, (ChangedQueryFilter, BaseQueryFilter)>;

type TransformsQuery<'w, 's> =
    Query<'w, 's, (&'static Transform, &'static mut GlobalTransform), BaseQueryFilter>;
type TransformQueries<'w, 's> = (ChangedQuery<'w, 's>, TransformsQuery<'w, 's>);

#[derive(SystemParam)]
struct MarkDirtySubtreesParams<'w, 's> {
    tables: Query<'w, 's, TablesQueryData, BaseQueryFilter>,
    parents: Query<'w, 's, &'static ChildOf, BaseQueryFilter>,
    orphaned: RemovedComponents<'w, 's, ChildOf>,
    param_set: ParamSet<'w, 's, TransformQueries<'static, 'static>>,
}

struct MarkDirtySubtreesTaskRunner<'w, 's, 'p> {
    tables: Query<'w, 's, TablesQueryData, BaseQueryFilter>,
    parents: Query<'w, 's, &'static ChildOf, BaseQueryFilter>,
    transforms: TransformsQuery<'w, 's>,

    source_markers: &'p SourceMarkers<'s>,
    propagation_sources: &'p PropagationSources,
}

#[derive(SystemParam)]
struct SourceMarkers<'s> {
    // Note: it might be tempting to make one of these a "seen" bitset, however it would be incorrect in
    // multithreaded contexts due to using `Relaxed` ordering, because a reader could observe a bit set in `seen`
    // but not in `is_maybe_source` (or the other bitset) even if the writer first wrote to that bitset and then to `seen`.
    is_maybe_source: Local<'s, BitsetStorage>,
    is_not_source: Local<'s, BitsetStorage>,
}

impl SourceMarkers<'_> {
    fn reset_table(&mut self, table: TableId, len: usize) {
        self.is_maybe_source.reset_table(table, len);
        self.is_not_source.reset_table(table, len);
    }

    /// # Safety
    ///
    /// `table` and `row` must be in bounds for this bitset storage.
    unsafe fn get_unchecked(&self, table: TableId, row: TableRow) -> Option<bool> {
        // Check first `is_not_source` because that is more likely to be set.

        // SAFETY: The caller guarantees `table` and `row` are in bounds.
        let is_not_source = unsafe { self.is_not_source.get_unchecked(table, row) };
        if is_not_source {
            return Some(false);
        }

        // SAFETY: The caller guarantees `table` and `row` are in bounds.
        let is_maybe_source = unsafe { self.is_maybe_source.get_unchecked(table, row) };
        if is_maybe_source {
            return Some(true);
        }

        None
    }

    /// # Safety
    ///
    /// `table` and `row` must be in bounds for this bitset storage.
    unsafe fn set_unchecked(&self, table: TableId, row: TableRow, is_maybe_source: bool) -> bool {
        let bitset = if is_maybe_source {
            &self.is_maybe_source
        } else {
            &self.is_not_source
        };

        // SAFETY: The caller guarantees `table` and `row` are in bounds.
        unsafe { bitset.set_unchecked(table, row) }
    }
}

impl TaskRunner for MarkDirtySubtreesTaskRunner<'_, '_, '_> {
    type Task = Entity;

    type WorkerLocal<'w> = RefMut<'w, Vec<Entity>>;

    fn init_local(&self) -> Self::WorkerLocal<'_> {
        self.propagation_sources.borrow_local_mut()
    }

    fn run(&self, task: Self::Task, mut scope: bevy_tasks::TaskScope<Self>) {
        let entity = task;
        let propagation_sources = scope.worker_local();

        // First loop: determine whether any ancestor of the current entity is a source.
        let mut has_ancestor_source = false;
        let mut need_ancestor_marking = false;
        for ancestor in self.parents.iter_ancestors(entity) {
            let Ok((table_id, _, table_row)) = self.tables.get(ancestor) else {
                continue;
            };

            // SAFETY: MarkDirtyTreesTaskRunner is only constructed after resetting `source_markers`'s tables,
            // which make all accesses inbounds for entities in queries with `BaseQueryFilters` (including `parents`).
            let is_maybe_source = unsafe { self.source_markers.get_unchecked(table_id, table_row) };

            // Short circuit: we check the cached data to see whether this parent was already processed or not.
            if let Some(is_maybe_source) = is_maybe_source {
                has_ancestor_source = is_maybe_source;
                break;
            }

            // If we reached this point there's at least one ancestor that is not marked yet, we'll want to mark them later.
            need_ancestor_marking = true;
        }

        // Second loop: mark the ancestors so future loops short-circuit earlier.
        if need_ancestor_marking {
            for ancestor in self.parents.iter_ancestors(entity) {
                let Ok((table_id, _, table_row)) = self.tables.get(ancestor) else {
                    continue;
                };

                let was_already_marked = unsafe {
                    self.source_markers
                        .set_unchecked(table_id, table_row, has_ancestor_source)
                };

                // Short circuit: we don't need to update any more entities if we find an entity that was already updated.
                if was_already_marked {
                    break;
                }
            }
        }

        // No ancestor is a source, so the current entity is a propagation source.
        // Start propagation from it.
        if !has_ancestor_source {
            propagation_sources.push(entity);

            // SAFETY: This is the only task processing this entity, so it cannot conflict with the same call at this point.
            // It also cannot conflict with the `get` on the same query right after this, as that can only alias this one
            // if we reach this point of the code when processing a child of the current entity, but the child of a propagation
            // source is never a propagation source, since one of its ancestor is one.
            let (transform, mut global_transform) =
                unsafe { self.transforms.get_unchecked(entity) }
                    .expect("entity matches the base filters");

            if let Ok(parent) = self.parents.get(entity)
                && let Ok((_, parent_transform)) = self.transforms.get(parent.get())
            {
                global_transform.set_if_neq(parent_transform.mul_transform(*transform));
            } else {
                global_transform.set_if_neq(GlobalTransform::from(*transform));
            }
        }
    }
}

fn mark_dirty_subtrees(
    mut param: MarkDirtySubtreesParams<'_, '_>,
    source_markers: &mut SourceMarkers<'_>,
    propagation_candidates: &mut Vec<Entity>,
    propagation_sources: &mut PropagationSources,
) {
    // Step 1: reset the bitset storages so that we can skip bound checks later.
    let contiguous_iter = param
        .tables
        .contiguous_iter()
        .expect("TableId, Table and Transform are dense");
    for (table_id, table, _) in contiguous_iter {
        source_markers.reset_table(table_id, table.entity_count() as usize);
    }

    // If static optimizations are enabled we'll look for entities that directly need a transform update,
    // then find all the other entities that will in turn also need a transform update.
    // We need to identify the propagation sources such that no source is a descendent of another source (for soundness!)
    // and finally run transform propagation from each of them.

    let changed = param.param_set.p0();
    // First gather and mark the potential propagation sources.
    // TODO: Maybe make this parallel?
    for (table_id, table_row, entity) in changed {
        // SAFETY: we just reset `source_markers` and `changed` requires `BaseQueryFilter`.
        unsafe { source_markers.set_unchecked(table_id, table_row, true) };
        propagation_candidates.push(entity);
    }
    for entity in param.orphaned.read() {
        let Ok((table_id, _, table_row)) = param.tables.get(entity) else {
            continue;
        };

        // SAFETY: we just reset `source_markers` and `changed` requires `BaseQueryFilter`.
        let was_already_marked = unsafe { source_markers.set_unchecked(table_id, table_row, true) };

        // Only process this entity if we didn't process it already. We cannot have duplicates in `propagation_candidates`.
        if !was_already_marked {
            propagation_candidates.push(entity);
        }
    }

    task_scope(
        propagation_candidates.drain(..),
        256,
        || ComputeTaskPool::get_or_init(TaskPool::default),
        |worker| {
            #[cfg(feature = "trace")]
            let _span = tracing::info_span!("mark dirty subtrees worker").entered();
            worker.run();
        },
        MarkDirtySubtreesTaskRunner {
            tables: param.tables,
            parents: param.parents,
            transforms: param.param_set.p1(),
            source_markers,
            propagation_sources,
        },
    );
}

type ChildTransformsQueryData<'a> = (
    &'a Transform,
    &'a ChildOf,
    TableId,
    TableRow,
    Option<&'a Children>,
);

#[derive(SystemParam)]
struct PropagateTransformParams<'w, 's> {
    parent_query: Query<'w, 's, Option<&'static Children>, BaseQueryFilter>,
    child_query: Query<'w, 's, ChildTransformsQueryData<'static>, BaseQueryFilter>,
    transform_query: Query<'w, 's, Mut<'static, GlobalTransform>, BaseQueryFilter>,
}

struct PropagateTransformTaskRunner<'o, 'w, 's> {
    params: PropagateTransformParams<'w, 's>,
    source_markers: &'o SourceMarkers<'s>,
    static_optimizations: StaticTransformOptimizations,
}

impl TaskRunner for PropagateTransformTaskRunner<'_, '_, '_> {
    type Task = Entity;

    type WorkerLocal<'w> = ();

    fn init_local<'w>(&'w self) -> Self::WorkerLocal<'w> {}

    fn run(&self, task: Self::Task, mut scope: bevy_tasks::TaskScope<Self>) {
        let entity = task;

        // Fetch the data for the current entity.
        let Ok(children) = self.params.parent_query.get(entity) else {
            return;
        };
        let Some(children) = children else { return };

        // SAFETY: parent_query requires `BaseQueryFilter`, which requires `GlobalTransform`, meaning this query must match.
        let parent_transform =
            unsafe { self.params.transform_query.get(entity).unwrap_unchecked() };
        let changed = parent_transform.is_changed();
        let mut curr = Some((entity, &*parent_transform, changed, children));

        while let Some((entity, parent_transform, parent_changed, children)) = curr.take() {
            // Double check that there are no duplicates in `Children`.
            // This will be unnecessary once `Children` guarantees uniqueness.
            debug_assert_eq!(
                children
                    .iter()
                    .collect::<bevy_ecs::entity::EntityHashSet>()
                    .len(),
                children.len()
            );

            for &child in children {
                let Ok((transform, parent, table_id, table_row, child_children)) =
                    self.params.child_query.get(child)
                else {
                    continue;
                };

                // Soundness check: we are iterating over `Children`, but the source of truth of whether
                // an entity is actually a child if `ChildOf`. We double check this to avoid processing
                // the same entity twice if the
                assert_eq!(entity, parent.get());

                // If static optimizations are enabled we only process an entity as long as either the
                // transform of the parent changed or any entity in its subtree might have their transform changed.
                let stop = self.static_optimizations.is_enabled()
                    && !parent_changed
                    // SAFETY: we reset the bitset at the start of the system for all entities matching `BaseQueryFilter`.
                    && unsafe { self.source_markers.get_unchecked(table_id, table_row) }.is_none();

                if stop {
                    continue;
                }

                // SAFETY: we only process each parent once, and we just checked that we are actually processing this entity's
                // parent, meaning we cannot be processing this entity twice. We only use this query mutably here, meaning
                // this is the only place where we mutably borrow this entity and no conflict can arise.
                let global_transform = unsafe { self.params.transform_query.get_unchecked(child) };

                // SAFETY: `child` matches `child_query`, whose `BaseQueryFilter` requires `GlobalTransform`, meaning
                // it must also match `transform_query`.
                let mut global_transform = unsafe { global_transform.unwrap_unchecked() };

                let changed =
                    global_transform.set_if_neq(parent_transform.mul_transform(*transform));

                if let Some(child_children) = child_children
                    && !child_children.is_empty()
                {
                    if curr.is_none() {
                        let global_transform = Ref::from(global_transform).into_inner();
                        curr = Some((child, global_transform, changed, child_children));
                    } else {
                        scope.add(child);
                    }
                }
            }
        }
    }
}

fn propagate_transform_dirty_subtrees(
    params: PropagateTransformParams<'_, '_>,
    source_markers: &SourceMarkers<'_>,
    static_optimizations: StaticTransformOptimizations,
    propagation_sources: &mut PropagationSources,
) {
    task_scope(
        propagation_sources.drain(),
        256,
        || ComputeTaskPool::get_or_init(TaskPool::default),
        |worker| {
            #[cfg(feature = "trace")]
            let _span = tracing::info_span!("transform propagation worker").entered();
            worker.run();
        },
        PropagateTransformTaskRunner {
            params,
            source_markers,
            static_optimizations,
        },
    );
}

#[cfg(test)]
mod test {
    use alloc::{vec, vec::Vec};
    use bevy_app::prelude::*;
    use bevy_ecs::world::CommandQueue;
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
        schedule.add_systems((propagate_transforms, sync_simple_transforms).chain());
        world.insert_resource(StaticTransformOptimizations::default());

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
        schedule.add_systems((propagate_transforms, sync_simple_transforms).chain());
        world.insert_resource(StaticTransformOptimizations::default());

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
        schedule.add_systems((propagate_transforms, sync_simple_transforms).chain());
        world.insert_resource(StaticTransformOptimizations::default());

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
        schedule.add_systems((propagate_transforms, sync_simple_transforms).chain());
        world.insert_resource(StaticTransformOptimizations::default());

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
            (propagate_transforms, sync_simple_transforms).chain(),
        )
        .insert_resource(StaticTransformOptimizations::default());

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
            propagate_transforms,
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

        // SAFETY: ChildOf is not mutable but this is for a test to produce a scenario that
        // cannot happen
        let mut a = unsafe { child_entity.get_mut_assume_mutable::<ChildOf>().unwrap() };

        // SAFETY: ChildOf is not mutable but this is for a test to produce a scenario that
        // cannot happen
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
        schedule.add_systems((propagate_transforms, sync_simple_transforms).chain());
        world.insert_resource(StaticTransformOptimizations::default());

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

    // Regression test for https://github.com/bevyengine/bevy/issues/25416
    #[test]
    fn did_propagate_after_entity_index_reuse() {
        // Enough same-frame removals to flush full chunks of stale entity IDs through
        // `mark_dirty_trees` before the live replacements are processed.
        const ROOT_COUNT: usize = 256;
        const CHILDREN_PER_ROOT: usize = 8;

        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut world = World::default();

        let mut schedule = Schedule::default();
        schedule.add_systems((propagate_transforms, sync_simple_transforms).chain());
        world.insert_resource(StaticTransformOptimizations::default());

        let roots: Vec<Entity> = (0..ROOT_COUNT)
            .map(|i| {
                world
                    .spawn(Transform::from_xyz(10.0 + i as f32, 0.0, 0.0))
                    .id()
            })
            .collect();
        let spawn_children = |world: &mut World| {
            for &root in &roots {
                for slot in 0..CHILDREN_PER_ROOT {
                    world.spawn((
                        Transform::from_xyz(0.0, 1.0 + slot as f32, 0.0),
                        ChildOf(root),
                    ));
                }
            }
        };
        spawn_children(&mut world);
        schedule.run(&mut world);

        // Despawn every child and spawn replacements in the same frame, reusing the freed
        // entity indices.
        let stale: Vec<Entity> = roots
            .iter()
            .flat_map(|&root| world.get::<Children>(root).unwrap().iter())
            .collect();
        for child in stale {
            world.despawn(child);
        }
        spawn_children(&mut world);
        schedule.run(&mut world);

        let mut children = world.query::<(&Transform, &GlobalTransform, &ChildOf)>();
        for (transform, global_transform, child_of) in children.iter(&world) {
            let root_transform = world.get::<Transform>(child_of.parent()).unwrap();
            assert_eq!(
                global_transform,
                &(GlobalTransform::from(*root_transform) * *transform),
                "respawned child kept a stale GlobalTransform",
            );
        }
    }
}
