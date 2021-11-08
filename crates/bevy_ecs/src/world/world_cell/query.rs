mod fetch;

use crate::{
    component::ComponentId,
    prelude::Entity,
    query::{Fetch, FilterFetch, FilteredAccess, QueryIter, QueryState, WorldQuery},
    world::{
        world_cell::query::fetch::OptQuery, CellCommandQueue, World, WorldCell, WorldCellState,
        WorldOverlay,
    },
};
use std::{
    any::{Any, TypeId},
    cell::Cell,
    collections::hash_set::IntoIter,
    marker::PhantomData,
    rc::Rc,
};

use bevy_utils::HashSet;
use fetch::CellFetch;
pub(crate) use fetch::FetchRefs;
pub use fetch::WorldCellQuery;

pub(super) struct QueryCacheEntry<Q: ?Sized + DynQueryState = dyn DynQueryState> {
    pub(super) alive_count: Cell<u32>,
    pub(super) in_working_set: Cell<bool>,
    pub(super) opt_query: Box<dyn DynQueryState>,
    pub(super) query: Q,
}

impl QueryCacheEntry {
    pub(super) fn alive_filtered_access(&self) -> Option<&FilteredAccess<ComponentId>> {
        if self.alive_count.get() > 0 {
            Some(self.query.component_access())
        } else {
            None
        }
    }
}

pub(super) trait DynQueryState: Any {
    fn component_access(&self) -> &FilteredAccess<ComponentId>;
    fn as_any(&self) -> &dyn Any;
}

impl<Q: WorldQuery + 'static, F: WorldQuery + 'static> DynQueryState for QueryState<Q, F>
where
    F::Fetch: FilterFetch,
{
    fn component_access(&self) -> &FilteredAccess<ComponentId> {
        &self.component_access
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone, Copy)]
pub struct QueryToken<Q, F = ()>(pub(super) PhantomData<(Q, F)>)
where
    Q: WorldCellQuery + 'static,
    F: WorldCellQuery + 'static,
    F::Fetch: FilterFetch;

pub struct CellQuery<'w, Q, F> {
    query_entry: Rc<QueryCacheEntry>,
    state: &'w WorldCellState,
    world: &'w World,
    marker: PhantomData<(Q, F)>,
}

impl<'w, Q, F> CellQuery<'w, Q, F>
where
    Q: WorldCellQuery + OptQuery + 'static,
    F: WorldCellQuery + 'static,
    F::Fetch: FilterFetch,
{
    #[allow(dead_code)]
    pub fn iter(&self) -> CellQueryIter<'w, '_, Q, F> {
        CellQueryIter::new(self)
    }
}

fn assert_component_access_compatibility(
    query_type: &'static str,
    filter_type: &'static str,
    current: &FilteredAccess<ComponentId>,
    world: &World,
    state: &WorldCellState,
) {
    let mut conflicts = state.get_live_query_conflicts_filtered(current);
    if conflicts.is_empty() {
        return;
    }
    let conflicting_components = conflicts
        .drain(..)
        .map(|component_id| world.components.get_info(component_id).unwrap().name())
        .collect::<Vec<&str>>();
    let accesses = conflicting_components.join(", ");
    panic!("CellQuery<{}, {}> in WorldCell accesses component(s) {} in a way that conflicts with other active access. Allowing this would break Rust's mutability rules. Consider using `Without<T>` to create disjoint Queries.",
                query_type, filter_type, accesses);
}

enum CellQueryIterState<'w, 's, Q, F>
where
    Q: WorldCellQuery + OptQuery,
    F: WorldCellQuery,
    F::Fetch: FilterFetch,
{
    Query {
        iter: QueryIter<'w, 's, (Entity, Q), F>,
        potential_new_entities: HashSet<Entity>,
    },
    Potential {
        iter: IntoIter<Entity>,
    },
}

pub struct CellQueryIter<'w, 's, Q, F>
where
    Q: WorldCellQuery + OptQuery,
    F: WorldCellQuery,
    F::Fetch: FilterFetch,
{
    iter: CellQueryIterState<'w, 's, Q, F>,
    opt_query: &'s QueryState<Q::OptQuery>,
    // Rc holds data referenced in `inner`. Must be dropped last.
    // That Rc is normally held inside `WorldCellState` anyway, but holding it directly allows to guarantee
    // safety easier, as `WorldCellState` is now free to evict cache at any time without consequences
    // extra_
    query_entry: Rc<QueryCacheEntry>,
    refs: FetchRefs,
    command_queue: &'w CellCommandQueue,
    world: &'w World,
    overlay: WorldOverlay,
}

impl<'w, 's, Q, F> Drop for CellQueryIter<'w, 's, Q, F>
where
    Q: WorldCellQuery + OptQuery,
    F: WorldCellQuery,
    F::Fetch: FilterFetch,
{
    fn drop(&mut self) {
        self.query_entry
            .alive_count
            .set(self.query_entry.alive_count.get() - 1);
    }
}

impl<'w, 's, Q, F> CellQueryIter<'w, 's, Q, F>
where
    Q: WorldCellQuery + OptQuery + 'static,
    F: WorldCellQuery + 'static,
    F::Fetch: FilterFetch,
{
    fn new(cell_query: &'s CellQuery<'w, Q, F>) -> Self {
        let query = cell_query
            .query_entry
            .query
            .as_any()
            .downcast_ref::<QueryState<(Entity, Q), F>>()
            .unwrap();
        let opt_query = cell_query
            .query_entry
            .opt_query
            .as_any()
            .downcast_ref::<QueryState<Q::OptQuery>>()
            .unwrap();

        // cast away the query_entry lifetime, so we can return an iterator that's self-referential
        // SAFETY:
        // - we hold onto the entry Rc for the entire lifetime of this reference, as it's cloned into returned WorldCellIter
        let query = unsafe {
            (query as *const QueryState<(Entity, Q), F>)
                .as_ref()
                .unwrap()
        };

        assert_component_access_compatibility(
            std::any::type_name::<Q>(),
            std::any::type_name::<F>(),
            &query.component_access,
            cell_query.world,
            cell_query.state,
        );

        let inner = unsafe {
            query.iter_unchecked_manual(
                cell_query.world,
                cell_query.world.last_change_tick(),
                cell_query.world.read_change_tick(),
            )
        };

        let query_entry = cell_query.query_entry.clone();
        query_entry
            .alive_count
            .set(query_entry.alive_count.get() + 1);

        let mut overlay = WorldOverlay::default();

        // prepare filters and modifiers based on current commands
        cell_query.state.command_queue.apply_overlay(
            &mut overlay,
            cell_query.world,
            &query.component_access,
        );

        Self {
            query_entry,
            iter: CellQueryIterState::Query {
                iter: inner,
                potential_new_entities: overlay.potential_new_entities(&query.component_access),
            },
            opt_query,
            refs: cell_query.state.current_query_refs.clone(),
            command_queue: &cell_query.state.command_queue,
            world: cell_query.world,
            overlay,
        }
    }
}

impl<'w, 's, Q, F> Iterator for CellQueryIter<'w, 's, Q, F>
where
    Q: WorldCellQuery + OptQuery,
    F: WorldCellQuery,
    F::Fetch: FilterFetch,
    <Q as WorldCellQuery>::CellFetch: CellFetch<
        'w,
        's,
        OptItem = <<<Q as OptQuery>::OptQuery as WorldQuery>::Fetch as Fetch<'w, 's>>::Item,
    >,
{
    type Item = <Q::CellFetch as CellFetch<'w, 's>>::CellItem;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.iter {
                CellQueryIterState::Query {
                    iter,
                    potential_new_entities,
                } => {
                    for (entity, data) in iter {
                        // no processing necessary
                        if !self.overlay.touched_entities.contains(&entity) {
                            return Some(Q::CellFetch::wrap(data, entity, &self.refs));
                        }

                        if self.overlay.despawned_entities.contains(&entity) {
                            continue;
                        }

                        if let Some(data) = Q::CellFetch::overlay(
                            data,
                            entity,
                            &self.refs,
                            &self.overlay,
                            self.world.components(),
                            self.command_queue,
                        ) {
                            potential_new_entities.remove(&entity);
                            return Some(data);
                        }
                    }
                    if let CellQueryIterState::Query {
                        potential_new_entities,
                        ..
                    } = std::mem::replace(
                        &mut self.iter,
                        CellQueryIterState::Potential {
                            iter: HashSet::default().into_iter(),
                        },
                    ) {
                        self.iter = CellQueryIterState::Potential {
                            iter: potential_new_entities.into_iter(),
                        };
                    }
                }
                // handle extra matches
                CellQueryIterState::Potential { iter } => {
                    for potential_match in iter {
                        if let Ok(raw) = unsafe {
                            self.opt_query.get_unchecked_manual(
                                self.world,
                                potential_match,
                                self.world.last_change_tick(),
                                self.world.read_change_tick(),
                            )
                        } {
                            if let Some(data) = Q::CellFetch::fetch_overlay(
                                raw,
                                potential_match,
                                &self.refs,
                                &self.overlay,
                                self.world.components(),
                                self.command_queue,
                            ) {
                                return Some(data);
                            }
                        }
                    }
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.iter {
            CellQueryIterState::Query {
                iter,
                potential_new_entities,
            } => {
                let (min, max) = iter.size_hint();
                (min, max.map(|x| x + potential_new_entities.len()))
            }
            CellQueryIterState::Potential { iter } => iter.size_hint(),
        }
    }
}

impl<'w> WorldCell<'w> {
    pub fn init_query<Q: WorldCellQuery + OptQuery + 'static>(&mut self) -> QueryToken<Q, ()> {
        self.init_filtered_query()
    }

    pub fn init_filtered_query<Q, F>(&mut self) -> QueryToken<Q, F>
    where
        Q: WorldCellQuery + OptQuery + 'static,
        F: WorldCellQuery + 'static,
        F::Fetch: FilterFetch,
    {
        let key = TypeId::of::<QueryState<(Entity, Q), F>>();
        let world = &mut self.world;
        self.state.query_cache.entry(key).or_insert_with(|| {
            Rc::new(QueryCacheEntry {
                alive_count: Cell::new(0),
                in_working_set: Cell::new(false),
                opt_query: Box::new(world.query::<Q::OptQuery>()),
                query: world.query_filtered::<(Entity, Q), F>(),
            })
        });

        QueryToken(PhantomData)
    }

    /// Requires `init_query` with the right type to be called beforehand
    pub fn query<Q, F>(&self, token: QueryToken<Q, F>) -> CellQuery<Q, F>
    where
        Q: WorldCellQuery + 'static,
        F: WorldCellQuery + 'static,
        F::Fetch: FilterFetch,
    {
        // token is only used to statically pass the query initialization state
        let _ = token;

        let key = TypeId::of::<QueryState<(Entity, Q), F>>();
        let query_entry = self
            .state
            .query_cache
            .get(&key)
            .expect("token cannot exist without initialization");

        // the token existence guarantees that the query was initialized, but not necessarily in the same WorldCell session.
        // So instead of during initialization, we add queries to working set at the first use in each session.
        if !query_entry.in_working_set.get() {
            query_entry.in_working_set.set(true);
            self.state
                .query_cache_working_set
                .borrow_mut()
                .push(query_entry.clone());
        }

        CellQuery {
            query_entry: query_entry.clone(),
            state: &self.state,
            world: self.world,
            marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        prelude::{Component, Entity, With, Without},
        world::{QueryToken, World, WorldCell},
    };

    #[derive(Component, Debug, Clone, PartialEq)]
    struct A;
    #[derive(Component, Debug, Clone, PartialEq)]
    struct B;

    #[derive(Component, Debug, Clone, PartialEq)]
    struct C(usize);

    #[test]
    fn world_cell_query() {
        let mut world = World::default();

        world.spawn().insert_bundle((A, B));
        world.spawn().insert(A);
        world.spawn().insert(B);
        let mut cell = world.cell();

        let t1 = cell.init_query::<&mut A>();
        let t2 = cell.init_query::<&mut B>();
        let t3 = cell.init_filtered_query::<&mut B, Without<A>>();
        let t4 = cell.init_query::<(&mut A, &mut B)>();

        let q1 = cell.query(t1);
        let q2 = cell.query(t2);
        let q3 = cell.query(t3);
        let q4 = cell.query(t4);

        let mut vals = Vec::new();
        for x in q1.iter() {
            for y in q2.iter() {
                vals.push((x.clone(), y.clone()));
            }
        }
        assert_eq!(vals, vec![(A, B), (A, B), (A, B), (A, B)]);

        let mut vals = Vec::new();
        for x in q2.iter() {
            for y in q1.iter() {
                vals.push((x.clone(), y.clone()));
            }
        }
        assert_eq!(vals, vec![(B, A), (B, A), (B, A), (B, A)]);

        let mut vals = Vec::new();
        for x in q3.iter() {
            for (y1, y2) in q4.iter() {
                vals.push((x.clone(), y1.clone(), y2.clone()));
            }
        }
        assert_eq!(vals, vec![(B, A, B)]);
    }

    #[test]
    #[should_panic]
    fn world_cell_query_access_panic() {
        let mut world = World::default();

        world.spawn().insert_bundle((A, B));
        world.spawn().insert(A);
        world.spawn().insert(B);
        let mut cell = world.cell();

        let t1 = cell.init_query::<&mut A>();
        let t2 = cell.init_query::<(&A, &mut B)>();

        let q1 = cell.query(t1);
        let q2 = cell.query(t2);

        for _x in q1.iter() {
            for _y in q2.iter() {
                // should panic
            }
        }
    }

    #[test]
    fn world_cell_query_twice() {
        let mut world = World::default();

        world.spawn().insert_bundle((A, B));
        world.spawn().insert(A);
        world.spawn().insert(B);
        let mut cell = world.cell();

        let t1 = cell.init_query::<&A>();

        let q1 = cell.query(t1);

        let mut vals = Vec::new();
        for x in q1.iter() {
            for y in q1.iter() {
                vals.push((x.clone(), y.clone()));
            }
        }
        assert_eq!(vals, vec![(A, A), (A, A), (A, A), (A, A)]);
    }

    #[test]
    #[should_panic]
    fn world_cell_query_twice_mut() {
        let mut world = World::default();

        world.spawn().insert_bundle((A, B));
        world.spawn().insert(A);
        world.spawn().insert(B);
        let mut cell = world.cell();

        let t1 = cell.init_query::<&mut A>();

        let q1 = cell.query(t1);

        for _x in q1.iter() {
            for _y in q1.iter() {
                // should panic
            }
        }
    }

    #[test]
    fn world_cell_query_in_fn() {
        let mut world = World::default();

        world.spawn().insert_bundle((A, B));
        world.spawn().insert(A);
        world.spawn().insert(B);
        let mut cell = world.cell();

        let t1 = cell.init_filtered_query();
        let t2 = cell.init_filtered_query();
        let t3 = cell.init_filtered_query();

        perform_query_a(&cell, t1);
        perform_query_b(&cell, t2, t3);

        fn perform_query_a(world: &WorldCell, t: QueryToken<&A>) {
            let mut vals = Vec::new();
            let q = world.query(t);
            for x in q.iter() {
                for y in q.iter() {
                    vals.push((x.clone(), y.clone()));
                }
            }
            assert_eq!(vals, vec![(A, A), (A, A), (A, A), (A, A)])
        }

        fn perform_query_b(
            world: &WorldCell,
            t1: QueryToken<(&mut A, &mut B)>,
            t2: QueryToken<&mut B, Without<A>>,
        ) {
            let mut vals = Vec::new();
            let q1 = world.query(t1);
            let q2 = world.query(t2);
            for (x1, x2) in q1.iter() {
                for y in q2.iter() {
                    vals.push((x1.clone(), x2.clone(), y.clone()));
                }
            }
            assert_eq!(vals, vec![(A, B, B)])
        }
    }

    #[test]
    fn world_cell_query_overlay() {
        let mut world = World::default();

        world.spawn().insert(A).insert(C(1));
        world.spawn().insert(A);
        world.spawn().insert(A).insert(C(2));
        // world.spawn()
        let mut cell = world.cell();

        let t1 = cell.init_query::<(Entity, &A, Option<&C>)>();
        let t2 = cell.init_query::<(&A, &C)>();

        let q1 = cell.query(t1);
        let q2 = cell.query(t2);

        let mut vals = Vec::new();
        for (entity, _, c) in q1.iter() {
            cell.entity(entity).insert(C(c.map_or(0, |c| c.0) + 10));
            for (_, c) in q2.iter() {
                vals.push(c.clone());
            }
        }
        assert_eq!(
            vals,
            vec![C(1), C(2), C(10), C(11), C(2), C(10), C(11), C(12), C(10)]
        );
    }

    #[test]
    fn world_cell_query_without_insert() {
        let mut world = World::default();

        let _e0 = world.spawn().insert(C(1)).id();
        let e1 = world.spawn().insert(A).insert(C(2)).id();
        let e2 = world.spawn().insert(A).id();
        let mut cell = world.cell();

        let token1 = cell.init_filtered_query();
        // let token2 = cell.init_filtered_query();

        let query1 = cell.query::<(Entity, With<A>, Without<C>), ()>(token1);
        // let query2 = cell.query::<Entity, (With<A>, Without<C>)>(token2);

        assert_eq!(query1.iter().collect::<Vec<_>>(), vec![(e2, true, true)]);
        // assert_eq!(query2.iter().collect::<Vec<_>>(), vec![e2]);

        cell.entity(e1).remove::<C>();
        cell.entity(e2).insert(C(3));

        assert_eq!(query1.iter().collect::<Vec<_>>(), vec![(e1, true, true)]);
        // assert_eq!(query2.iter().collect::<Vec<_>>(), vec![e1]);
    }
}
