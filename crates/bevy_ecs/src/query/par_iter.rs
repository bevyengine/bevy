use std::marker::PhantomData;

use bevy_tasks::ParallelIterator;

use crate::{
    archetype::{ArchetypeId, Archetypes},
    query::{Fetch, FilterFetch, QueryState, WorldQuery},
    storage::{TableId, Tables},
    world::World,
};

pub enum ParQueryIter<'w, 's, Q: WorldQuery, F: WorldQuery>
where
    F::Fetch: FilterFetch,
{
    Dense {
        batch_size: usize,
        offset: usize,
        world: &'w World,
        query_state: &'s QueryState<Q, F>,
        tables: &'w Tables,
        table_id_iter: std::slice::Iter<'s, TableId>,
        table: Option<TableId>,
        last_change_tick: u32,
        change_tick: u32,
    },
    Sparse {
        batch_size: usize,
        offset: usize,
        world: &'w World,
        query_state: &'s QueryState<Q, F>,
        archetypes: &'w Archetypes,
        archetype_id_iter: std::slice::Iter<'s, ArchetypeId>,
        archetype: Option<ArchetypeId>,
        last_change_tick: u32,
        change_tick: u32,
    },
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> ParQueryIter<'w, 's, Q, F>
where
    F::Fetch: FilterFetch,
{
    pub(crate) unsafe fn new(
        world: &'w World,
        query_state: &'s QueryState<Q, F>,
        batch_size: usize,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        let fetch = <Q::Fetch as Fetch>::init(
            world,
            &query_state.fetch_state,
            last_change_tick,
            change_tick,
        );
        let filter = <F::Fetch as Fetch>::init(
            world,
            &query_state.filter_state,
            last_change_tick,
            change_tick,
        );
        if fetch.is_dense() && filter.is_dense() {
            let tables = &world.storages().tables;
            let mut table_id_iter = query_state.matched_table_ids.iter();
            ParQueryIter::Dense {
                batch_size,
                offset: 0,
                world,
                query_state,
                tables,
                last_change_tick,
                change_tick,
                table: table_id_iter.next().copied(),
                table_id_iter,
            }
        } else {
            let archetypes = world.archetypes();
            let mut archetype_id_iter = query_state.matched_archetype_ids.iter();
            ParQueryIter::Sparse {
                batch_size,
                offset: 0,
                world,
                query_state,
                archetypes,
                last_change_tick,
                change_tick,
                archetype: archetype_id_iter.next().copied(),
                archetype_id_iter,
            }
        }
    }
}

type QItem<'w, Q> = <<Q as WorldQuery>::Fetch as Fetch<'w>>::Item;

pub struct IntoBatchIterator<'w, 's, Q: WorldQuery, F: WorldQuery>
where
    F::Fetch: FilterFetch,
{
    world: &'w World,
    state: &'s QueryState<Q, F>,
    index_range: <std::ops::Range<usize> as IntoIterator>::IntoIter,
    last_change_tick: u32,
    change_tick: u32,
    tor: TableOrArchetype,
}
enum TableOrArchetype {
    Table(TableId),
    Archetype(ArchetypeId),
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> IntoIterator for IntoBatchIterator<'w, 's, Q, F>
where
    F::Fetch: FilterFetch,
{
    type Item = QItem<'w, Q>;

    type IntoIter = BatchIterator<'w, Q, F>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe {
            let mut fetch = <Q::Fetch as Fetch>::init(
                self.world,
                &self.state.fetch_state,
                self.last_change_tick,
                self.change_tick,
            );
            let mut filter = <F::Fetch as Fetch>::init(
                self.world,
                &self.state.filter_state,
                self.last_change_tick,
                self.change_tick,
            );

            let tables = &self.world.storages().tables;
            let dense = match self.tor {
                TableOrArchetype::Table(table) => {
                    let table = &tables[table];
                    fetch.set_table(&self.state.fetch_state, table);
                    filter.set_table(&self.state.filter_state, table);
                    true
                }
                TableOrArchetype::Archetype(archetype_id) => {
                    let archetype = &self.world.archetypes[archetype_id];
                    fetch.set_archetype(&self.state.fetch_state, archetype, tables);
                    filter.set_archetype(&self.state.filter_state, archetype, tables);
                    false
                }
            };

            BatchIterator {
                marker: PhantomData,
                dense,
                fetch,
                filter,
                ir: self.index_range,
            }
        }
    }
}

pub struct BatchIterator<'w, Q: WorldQuery, F: WorldQuery>
where
    F::Fetch: FilterFetch,
{
    marker: PhantomData<&'w ()>,
    dense: bool,
    fetch: Q::Fetch,
    filter: F::Fetch,
    ir: <std::ops::Range<usize> as IntoIterator>::IntoIter,
}

impl<'w, Q: WorldQuery, F: WorldQuery> Iterator for BatchIterator<'w, Q, F>
where
    F::Fetch: FilterFetch,
{
    type Item = QItem<'w, Q>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.ir.next() {
            unsafe {
                Some(if self.dense {
                    if !self.filter.table_filter_fetch(index) {
                        return self.next();
                    }
                    self.fetch.table_fetch(index)
                } else {
                    if !self.filter.archetype_filter_fetch(index) {
                        return self.next();
                    }
                    self.fetch.archetype_fetch(index)
                })
            }
        } else {
            None
        }
    }
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> ParallelIterator<IntoBatchIterator<'w, 's, Q, F>>
    for ParQueryIter<'w, 's, Q, F>
where
    F::Fetch: FilterFetch,
{
    type Item = QItem<'w, Q>;

    fn next_batch(&mut self) -> Option<IntoBatchIterator<'w, 's, Q, F>> {
        match self {
            ParQueryIter::Dense {
                batch_size,
                offset,
                world,
                query_state,
                tables,
                table_id_iter,
                table,
                last_change_tick,
                change_tick,
            } => {
                if let Some(table) = table {
                    if *offset >= tables[*table].len() {
                        if let Some(&id) = table_id_iter.next() {
                            dbg!(id);
                            *table = id;
                            *offset = 0;
                        } else {
                            return None;
                        }
                    }
                    let table_id = *table;
                    let table = &tables[table_id];
                    let len = (*batch_size).min(table.len() - *offset);
                    let range = (*offset)..((*offset) + len);
                    *offset += *batch_size;
                    Some(IntoBatchIterator {
                        world: *world,
                        state: *query_state,
                        index_range: range,
                        last_change_tick: *last_change_tick,
                        change_tick: *change_tick,
                        tor: TableOrArchetype::Table(table_id),
                    })
                } else if let Some(&id) = table_id_iter.next() {
                    dbg!(id);
                    *table = Some(id);
                    *offset = 0;
                    self.next_batch()
                } else {
                    None
                }
            }
            ParQueryIter::Sparse {
                batch_size,
                offset,
                world,
                query_state,
                archetypes,
                archetype_id_iter,
                archetype,
                last_change_tick,
                change_tick,
            } => {
                if let Some(archetype) = archetype {
                    if *offset >= archetypes[*archetype].len() {
                        if let Some(&id) = archetype_id_iter.next() {
                            *archetype = id;
                            *offset = 0;
                        } else {
                            return None;
                        }
                    }
                    let archetype_id = *archetype;
                    let archetype = &archetypes[archetype_id];
                    let len = (*batch_size).min(archetype.len() - *offset);
                    let range = (*offset)..((*offset) + len);
                    *offset += *batch_size;
                    Some(IntoBatchIterator {
                        world: *world,
                        state: *query_state,
                        index_range: range,
                        last_change_tick: *last_change_tick,
                        change_tick: *change_tick,
                        tor: TableOrArchetype::Archetype(archetype_id),
                    })
                } else if let Some(&id) = archetype_id_iter.next() {
                    *archetype = Some(id);
                    *offset = 0;
                    self.next_batch()
                } else {
                    None
                }
            }
        }
    }
}
