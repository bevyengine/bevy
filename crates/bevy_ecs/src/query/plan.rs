#![allow(dead_code)]
use alloc::borrow::Cow;
use crate::{
    component::ComponentId,
    entity::Entity,
    query::FilteredAccess,
    world::unsafe_world_cell::UnsafeWorldCell,
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::vec;
use std::any::Any;
use std::ptr::NonNull;
use fixedbitset::FixedBitSet;
use bevy_ecs::archetype::Archetype;
use bevy_ecs::prelude::{ContainsEntity, QueryBuilder, World};
use bevy_ecs::query::{QueryData, QueryFilter, WorldQuery};
use bevy_ecs::relationship::{Relationship, RelationshipTarget};
use bevy_ecs::storage::{SparseSetIndex, TableId, TableRow};
use bevy_ecs::world::unsafe_world_cell::UnsafeEntityCell;
use bevy_ptr::{Ptr, PtrMut};
use crate::relationship::RelationshipAccessor;

/// Represents a single source in a multi-source query.
/// Each term has its own ComponentAccess requirements.
#[derive(Debug)]
pub enum QueryOperation {
    Relationship(QueryRelationship),
    Source(QuerySource),
}

type MatchesArchetypeFn = unsafe fn(archetype: &Archetype, component_access: &FilteredAccess, fetch_state: Ptr, filter_state: Ptr) -> bool;

/// Check if the archetype should be considered
pub unsafe fn matches_archetype<D: QueryData, F: QueryFilter>(archetype: &Archetype, component_access: &FilteredAccess, fetch_state: Ptr, filter_state: Ptr) -> bool {
    let fetch_state = unsafe { fetch_state.deref::<D::State>() };
    let filter_state = unsafe { filter_state.deref::<F::State>() };
    D::matches_component_set(fetch_state, &|id| archetype.contains(id))
        && F::matches_component_set(filter_state, &|id| archetype.contains(id))
        && QuerySource::matches_component_set(component_access, &|id| archetype.contains(id))
}

type FilterFetchFn = unsafe fn(state: Ptr, fetch: PtrMut, entity: Entity, table_row: TableRow) -> bool;

/// Returns true if the provided [`Entity`] and [`TableRow`] should be included in the query results.
/// If false, the entity will be skipped.
pub unsafe fn filter_fetch<F: QueryFilter>(state: Ptr, fetch: PtrMut, entity: Entity, table_row: TableRow) -> bool {
    let state = unsafe { state.deref::<F::State>() };
    let fetch = unsafe { fetch.deref_mut::<F::Fetch<'_>>() };
    unsafe { F::filter_fetch(state, fetch, entity, table_row) }
}

#[derive(Debug)]
pub struct QuerySource {
    access: FilteredAccess,
    fetch_state: Box<dyn Any>,
    filter_state: Box<dyn Any>,
    matches_archetype_fn: MatchesArchetypeFn,
    filter_fetch_fn: FilterFetchFn,
    variable_idx: u8,
}


impl QuerySource {
    pub fn matches(&self, archetype: &Archetype) -> bool {
        let fetch_state = unsafe { NonNull::new_unchecked(self.fetch_state.as_ref() as *const dyn Any as *mut dyn Any) };
        let filter_state = unsafe { NonNull::new_unchecked(self.filter_state.as_ref() as *const dyn Any as *mut dyn Any) };
        unsafe { (self.matches_archetype_fn)(
            archetype,
            &self.access,
            Ptr::new(fetch_state.cast::<u8>()),
            Ptr::new(filter_state.cast::<u8>()),
        ) }
    }

    /// Returns `true` if this access matches a set of components. Otherwise, returns `false`.
    pub fn matches_component_set(access: &FilteredAccess, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        access.filter_sets.iter().any(|set| {
            set.with
                .ones()
                .all(|index| set_contains_id(ComponentId::get_sparse_set_index(index)))
                && set
                    .without
                    .ones()
                    .all(|index| !set_contains_id(ComponentId::get_sparse_set_index(index)))
        })
    }
}

/// Describes how two query terms are connected via a relationship.
#[derive(Debug, Clone)]
pub struct QueryRelationship {
    /// The source term index within the QueryPlan. The source is always from the point of view of the Relationship component, not the RElationshipTarget.
    /// ChildOf(0, 1) adds 0 as source, 1 as target.
    /// Children(0, 1) adds 1 as source, 0 as target.
    pub source_idx: u8,
    /// The target term index within the QueryPlan
    pub target_idx: u8,
    /// The relationship component that links source to target. (e.g. ChildOf)
    pub relationship_component: ComponentId,
    /// The relationship target component that links target to source. (e.g. Children)
    pub relationship_target_component: ComponentId,
    /// Accessor to dynamically access the 'target' of the relationship
    pub relationship_accessor: RelationshipAccessor,
    pub relationship_target_accessor: RelationshipAccessor,
}

impl QueryRelationship {
    /// Get the 'target' of the source entity
    pub unsafe fn get_target(&self, source: Entity, world: UnsafeWorldCell<'_>) -> Option<Entity> {
        let entity_field_offset = match self.relationship_accessor {
            RelationshipAccessor::Relationship { entity_field_offset, .. } => {entity_field_offset}
            RelationshipAccessor::RelationshipTarget { .. } => {
                unreachable!()
            }
        };
        let relationship_ptr = world.get_entity(source).ok()?.get_by_id(self.relationship_component)?;
        Some(unsafe { *relationship_ptr.byte_add(entity_field_offset).deref() })
    }

    pub unsafe fn get_sources(&self, target: Entity, world: UnsafeWorldCell<'_>) -> Option<Vec<Entity>> {
        let iter = match self.relationship_target_accessor {
            RelationshipAccessor::Relationship { .. } => {
                unreachable!()
            }
            RelationshipAccessor::RelationshipTarget { iter, .. } => {iter}
        };
        let relationship_ptr = world.get_entity(target).ok()?.get_by_id(self.relationship_target_component)?;
        let sources: Vec<_> = unsafe { iter(relationship_ptr).collect() };
        Some(sources)
    }
}

pub struct QueryVariable {
    index: u8,
}

impl<T: Into<u8>> From<T> for QueryVariable  {
    fn from(value: T) -> Self {
        Self {
            index: value.into()
        }
    }
}

pub struct QueryPlanBuilder<'w> {
    world: &'w mut World,
    plan: QueryPlan,
}

impl<'w> QueryPlanBuilder<'w> {
    pub fn new(world: &'w mut World) -> Self {
        Self {
            world,
            plan: QueryPlan::default(),
        }
    }

    pub fn add_source_from_builder<D: QueryData, F: QueryFilter>(&mut self, f: impl Fn(QueryBuilder) -> QueryBuilder) -> &mut Self
        where <D as WorldQuery>::State: 'static,
              <F as WorldQuery>::State: 'static, {
        let query_builder = QueryBuilder::new(&mut self.world);
        let mut builder = f(query_builder);
        let builder = builder.transmute_filtered::<D, F>();

        let mut fetch_state = D::init_state(builder.world_mut());
        let filter_state = F::init_state(builder.world_mut());

        let mut component_access = FilteredAccess::default();
        D::update_component_access(&fetch_state, &mut component_access);
        D::provide_extra_access(
            &mut fetch_state,
            component_access.access_mut(),
            builder.access().access(),
        );

        let access = builder.access().clone();
        let matches_archetype_fn: MatchesArchetypeFn  = matches_archetype::<D, F>;
        let filter_fetch_fn: FilterFetchFn  = filter_fetch::<F>;
        self.plan.add_source(access, Box::new(fetch_state), Box::new(filter_state), matches_archetype_fn, filter_fetch_fn);
        self
    }

    pub fn add_source<D: QueryData, F: QueryFilter>(&mut self) -> &mut Self
    where <D as WorldQuery>::State: 'static,
          <F as WorldQuery>::State: 'static, {
        let fetch_state = D::init_state(&mut self.world);
        let filter_state = F::init_state(&mut self.world);

        let mut access = FilteredAccess::default();
        D::update_component_access(&fetch_state, &mut access);

        // Use a temporary empty FilteredAccess for filters. This prevents them from conflicting with the
        // main Query's `fetch_state` access. Filters are allowed to conflict with the main query fetch
        // because they are evaluated *before* a specific reference is constructed.
        let mut filter_access = FilteredAccess::default();
        F::update_component_access(&filter_state, &mut filter_access);

        // Merge the temporary filter access with the main access. This ensures that filter access is
        // properly considered in a global "cross-query" context (both within systems and across systems).
        access.extend(&filter_access);

        let matches_archetype_fn: MatchesArchetypeFn = matches_archetype::<D, F>;
        let filter_fetch_fn: FilterFetchFn  = filter_fetch::<F>;
        self.plan.add_source(access, Box::new(fetch_state), Box::new(filter_state), matches_archetype_fn, filter_fetch_fn);
        self
    }


    /// Add a relationship between two terms using a typed Relationship component.
    pub fn add_relationship<R: Relationship>(
        &mut self,
        source_term: u8,
        target_term: u8,
    ) {
        let component_id = self.world.register_component::<R>();
        let target_component_id = self.world.register_component::<<R as Relationship>::RelationshipTarget>();
        let accessor = self.world
            .components()
            .get_info(component_id)
            .unwrap()
            .relationship_accessor()
            .unwrap().clone();
        let target_accessor = self.world
            .components()
            .get_info(target_component_id)
            .unwrap()
            .relationship_accessor()
            .unwrap().clone();

        self.plan.add_relationship(
            source_term,
            target_term,
            component_id,
            target_component_id,
            accessor,
            target_accessor,
        );
    }

    /// Add a relationship between two terms using a typed Relationship component.
    pub fn add_relationship_target<R: RelationshipTarget>(
        &mut self,
        source_term: u8,
        target_term: u8,
    ) {
        let component_id = self.world.register_component::<<R as RelationshipTarget>::Relationship>();
        let target_component_id = self.world.register_component::<R>();
        let accessor = self.world
            .components()
            .get_info(component_id)
            .unwrap()
            .relationship_accessor()
            .unwrap().clone();
        let target_accessor = self.world
            .components()
            .get_info(target_component_id)
            .unwrap()
            .relationship_accessor()
            .unwrap().clone();

        self.plan.add_relationship(
            target_term,
            source_term,
            component_id,
            target_component_id,
            accessor,
            target_accessor,
        );
    }

    pub fn compile(self) -> QueryPlan {
        self.plan.compile()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum QueryPlanError {
    /// The source does not exist
    #[error("The source with index {0} does not exist")]
    QuerySourceNotFound(u8),
}

/// A dynamic query plan that describes how to match multiple entities
/// connected through relationships.
#[derive(Debug, Default)]
pub struct QueryPlan {
    /// All operations in this query.
    pub operations: Vec<QueryOperation>,
    pub num_variables: u8,
}


impl QueryPlan {
    pub fn query_iter<'w, 's>(&'s self, world: UnsafeWorldCell<'w>) -> Iter<'w, 's> {
        Iter {
            world,
            query_state: &self,
            iter_state: IterState::new(self),
        }
    }
}


#[derive(Debug, Clone)]
pub enum VariableState {
    // Default state: we haven't applied any constraints yet
    Unknown,
    // we are searching through all the entities of a table
    Search {
        // offset in the current table
        offset: u32,
        // length of the current table
        current_len: u32,
        // index of the table inside the StorageState
        storage_idx: u32,
    },
    // An entity has been found by following a relationship
    FixedByRelationship(Entity),
    // The entity is among the RelationshipTargets
    FixedByRelationshipTarget {
        sources: Vec<Entity>,
        index: usize,
    }
}

impl Default for VariableState {
    fn default() -> Self {
        Self::Unknown
    }
}


#[derive(Default, Clone)]
pub struct StorageState<'w> {
    /// List of tables that are being iterated for this variable. (maybe merge with VariableState::Search?)
    tables: Cow<'w, [TableId]>,
}

pub struct IterState<'w> {
    // are we backtracking through the plan?
    pub backtrack: bool,
    // index of the operation in the plan we are currently executing
    pub curr_op: u8,
    /// Index of the current entity for each variable
    pub variable_state: Vec<VariableState>,
    /// List of matching tables/archetypes to iterate through for each variable
    pub storages_state: Vec<StorageState<'w>>,
    /// For each operation, which variables were written by the operation
    /// This is useful to know if we need to backtrack a variable when backtracking the operation.
    written: Vec<FixedBitSet>,
}

impl<'w> IterState<'w> {
    fn new(plan: &QueryPlan) -> Self {
        let num_operations = plan.operations.len();
        let num_variables = plan.num_variables as usize;
        let variable_state =  vec![VariableState::default(); num_variables];
        let storages_state = vec![StorageState::default(); num_variables];
        let written = vec![FixedBitSet::with_capacity(num_variables); num_operations];
        Self {
            backtrack: false,
            curr_op: 0,
            variable_state,
            storages_state,
            written,
        }
    }

}


/// Iterator that iterates through a dynamic query plan
pub struct Iter<'w, 's> {
    world: UnsafeWorldCell<'w>,
    query_state: &'s QueryPlan,
    iter_state: IterState<'s>,
}


impl<'w, 's> Iter<'w, 's> {

    /// Returns true if the `variable` was assigned an Entity in the current operation
    fn written(&self, variable: u8) -> bool {
        self.iter_state.written[self.iter_state.curr_op as usize].contains(variable as usize)
    }

    /// Get the entity currently written to the variable indexed by `variable`, if it has been written
    fn written_entity(&self, variable: u8) -> Option<Entity> {
        let variable_idx = variable as usize;
        match &self.iter_state.variable_state[variable_idx] {
            VariableState::Unknown => {
                None
            }
            VariableState::Search { storage_idx, offset, .. } => {
                // Safety:
                let table_id = &self.iter_state.storages_state[variable_idx].tables[*storage_idx as usize];
                let table = unsafe { self.world.storages() }.tables.get(*table_id)?;
                Some(unsafe{ *table.entities().get_unchecked(*offset as usize) })
            }
            VariableState::FixedByRelationship(entity) => {Some(*entity)}
            VariableState::FixedByRelationshipTarget { sources, index } => {
                Some(sources[*index])
            }
        }
    }

    fn dispatch(&mut self) -> bool {
        let op = self.current_op();
        match op {
            QueryOperation::Relationship(_) => self.op_relationship(),
            QueryOperation::Source(_) => self.op_query(),
        }
    }

    /// Try to find an entity via the relationship
    fn op_relationship(&mut self) -> bool {
        let QueryOperation::Relationship(rel) = self.current_op() else {
            unreachable!()
        };
        if self.iter_state.backtrack {
            if self.written(rel.source_idx) {
                return self.backtrack_variable(rel.source_idx);
            }
            if self.written(rel.target_idx) {
                return self.backtrack_variable(rel.target_idx);
            }
            return false;
        }
        match (self.written_entity(rel.source_idx), self.written_entity(rel.target_idx)) {
            (None, None) => {
                unreachable!("we only support queries where the source has been found before we are querying the relationship");
            }
            (Some(source_entity), None) => {
                // we found the source, need to find the target
                match unsafe { rel.get_target(source_entity, self.world) } {
                    None => false,
                    Some(target_entity) => {
                        self.set_variable_state(rel.target_idx, VariableState::FixedByRelationship(target_entity));
                        true
                    }
                }
            }
            (None, Some(target_entity)) => {
                // we found the target, need to find the source
                match unsafe { rel.get_sources(target_entity, self.world) } {
                    None => false,
                    Some(sources) => {
                        self.set_variable_state(rel.source_idx, VariableState::FixedByRelationshipTarget {
                            sources,
                            index: 0,
                        });
                        true
                    }
                }
            }
            (Some(source_entity), Some(target_entity)) => {
                // we found both, need to check if they are linked by the relationship
                unsafe { rel.get_target(source_entity, self.world) }.is_some_and(|expected_target_entity| target_entity == expected_target_entity)
            }
        }

    }

    fn op_query(&mut self) -> bool {
        let QueryOperation::Source(source) = &self.query_state.operations[self.iter_state.curr_op as usize] else {
            unreachable!()
        };
        let variable_idx = source.variable_idx as usize;
        let storage_state = &self.iter_state.storages_state[variable_idx];
        match &mut self.iter_state.variable_state[variable_idx] {
            VariableState::Unknown => {
                if self.iter_state.backtrack {
                    return false
                }
                // the first time we evaluate the query, we set the list of potential tables
                let tables = &storage_state.tables;
                assert_eq!(tables.len(), 0);

                let mut matching_tables = Vec::new();
                let mut current_len = 0;
                if source.access.required.is_empty() {
                    for archetype in self.world.archetypes().iter() {
                        // NOTE: you could have empty archetypes even if the table is not empty
                        //  and you could have non-empty archetypes with empty tables (e.g. when the archetype only has sparse sets)
                        let table_id = archetype.table_id();
                        let table = unsafe { self.world.world().storages().tables.get(table_id).unwrap_unchecked() };
                        // skip empty tables
                        if !table.is_empty() && source.matches(archetype) {
                            if current_len == 0 {
                                current_len = table.entity_count();
                            }
                            matching_tables.push(archetype.table_id())
                        }
                    }
                } else {
                    // if there are required components, we can optimize by only iterating through archetypes
                    // that contain at least one of the required components
                    let potential_archetypes = source
                        .access
                        .required
                        .ones()
                        .filter_map(|idx| {
                            let component_id = ComponentId::get_sparse_set_index(idx);
                            self.world
                                .archetypes()
                                .component_index()
                                .get(&component_id)
                                .map(|index| index.keys())
                        })
                        // select the component with the fewest archetypes
                        .min_by_key(ExactSizeIterator::len);
                    if let Some(archetypes) = potential_archetypes {
                        for archetype_id in archetypes {
                            // SAFETY: get_potential_archetypes only returns archetype ids that are valid for the world
                            let archetype = &self.world.archetypes()[*archetype_id];
                            // NOTE: you could have empty archetypes even if the table is not empty
                            //  and you could have non-empty archetypes with empty tables (e.g. when the archetype only has sparse sets)
                            let table_id = archetype.table_id();
                            let table = unsafe { self.world.world().storages().tables.get(table_id).unwrap_unchecked() };
                            // skip empty tables
                            if !table.is_empty() && source.matches(archetype) {
                                if current_len == 0 {
                                    current_len = table.entity_count();
                                }
                                matching_tables.push(archetype.table_id())
                            }
                        }
                    }
                }
                if matching_tables.first().is_none() {
                    return false;
                }
                self.iter_state.storages_state[variable_idx] = StorageState {
                    tables: Cow::Owned(matching_tables)
                };
                // TODO: need to iterate here until we find a good candidate! check with filter_fetch
                self.set_variable_state(variable_idx as u8, VariableState::Search {
                    offset: 0,
                    storage_idx: 0,
                    current_len,
                });
                true
            }
            VariableState::Search { .. } => {
                // we are already searching through a list of tables, we need to increment the index
                assert!(self.iter_state.backtrack);
                assert!(self.written(source.variable_idx));
                self.backtrack_variable(source.variable_idx)
            }
            _ => {
                // the entity has been fixed by a relationship, we need to check if it matches the query
                // (unless we are backtracking)
                if self.iter_state.backtrack {
                    return false
                }
                let candidate = unsafe { self.written_entity(source.variable_idx).unwrap_unchecked() };
                let archetype = unsafe { self.world.get_entity(candidate).unwrap_unchecked().archetype() };
                source.matches(archetype)
            }
        }
    }

    /// Assign the variable state and keep track that the variable was written in the current operation
    fn set_variable_state(&mut self, variable: u8, state: VariableState) {
        self.iter_state.written[self.iter_state.curr_op as usize].insert(variable as usize);
        self.iter_state.variable_state[variable as usize] = state;
    }

    fn current_op(&self) -> &QueryOperation {
        &self.query_state.operations[self.iter_state.curr_op as usize]
    }

    /// Backtrack the variable if it had been written in the current operation
    fn backtrack_variable(&mut self, variable: u8) -> bool {
        let curr_op = self.iter_state.curr_op as usize;
        let variable_idx = variable as usize;
        // wrap in closure to allow early return
        let res = (|| {
            match &mut self.iter_state.variable_state[variable_idx] {
                VariableState::Unknown => {
                    unreachable!()
                }
                VariableState::Search { offset, current_len, storage_idx } => {
                    let storage_state = &self.iter_state.storages_state[variable_idx];
                    if *storage_idx >= storage_state.tables.len() as u32 {
                        return false
                    }
                    // TODO: apply filter_fetch
                    // either beginning of the iteration, or finished processing a table, so skip to the next
                    if (*offset + 1) == *current_len {
                        // go to next table
                        *storage_idx += 1;
                        if *storage_idx >= storage_state.tables.len() as u32 {
                            return false
                        }
                        *offset = 0;
                        let table_id = storage_state.tables[*storage_idx as usize];
                        let table = unsafe { self.world.world().storages().tables.get(table_id).unwrap_unchecked() };
                        *current_len = table.entity_count();
                    } else {
                        *offset += 1;
                    }
                    return true
                }
                VariableState::FixedByRelationship(_) => {
                    false
                }
                VariableState::FixedByRelationshipTarget { sources, index } => {
                    *index += 1;
                    if *index >= sources.len() {
                        return false
                    }
                    true
                }
            }
        })();
        if !res {
            // if we don't find a next candidate, reset the variable state to Unknown and remove from written
            self.iter_state.written[curr_op].remove(variable_idx);
            self.iter_state.variable_state[variable_idx] = VariableState::Unknown;
        }
        res
    }

    /// Check if we can still find a matching item
    fn next_match(&mut self) -> bool {
        let num_operations = self.query_state.operations.len() as u8;
        // Past the end of operations so must have matched, return to previous op
        if self.iter_state.curr_op == num_operations {
            self.iter_state.backtrack = true;
            self.iter_state.curr_op -= 1;
        }

        while self.iter_state.curr_op < num_operations {
            let matched = self.dispatch();
            if !matched {
                // Returned from all operations, no more matches
                if self.iter_state.curr_op == 0 {
                    return false;
                }
                self.iter_state.backtrack = true;
                self.iter_state.curr_op -= 1;
            } else {
                // Operation did match, move to next op
                self.iter_state.backtrack = false;
                self.iter_state.curr_op += 1;
            }
        }
        true
    }
}

pub struct DynamicItem<'w> {
    // TODO: return FilteredEntityRef/FilteredEntityMut by borrowing from the State, which contains the access.
    entities: Vec<UnsafeEntityCell<'w>>,
}

impl core::fmt::Debug for DynamicItem<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list()
            .entries(self.entities.iter().map(|e| e.entity()))
            .finish()
    }
}

impl<'w, 's> Iterator for Iter<'w, 's> {
    type Item = DynamicItem<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.next_match() {
            return None;
        }
        // retrieve the Item
        let num_variables = self.query_state.num_variables as usize;
        let mut matching_entities = Vec::with_capacity(num_variables);
        for idx in 0..num_variables {
            let entity = self.written_entity(idx as u8).unwrap();
            let entity_cell = self.world.get_entity(entity).unwrap();
            matching_entities.push(entity_cell);
        }
        Some(DynamicItem {
            entities: matching_entities
        })
    }
}


impl QueryPlan {
    /// Create a new empty query plan.
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            num_variables: 0,
        }
    }

    /// Add a term to the query plan.
    pub(crate) fn add_source(&mut self, access: FilteredAccess, fetch_state: Box<dyn Any>, filter_state: Box<dyn Any>, matches_archetype_fn: MatchesArchetypeFn, filter_fetch_fn: FilterFetchFn) ->
                                                                                                                                                                                               usize {
        let op_index = self.operations.len();
        self.operations.push(QueryOperation::Source(QuerySource {
            access,
            fetch_state,
            filter_state,
            matches_archetype_fn,
            filter_fetch_fn,
            variable_idx: self.num_variables,
        }));
        self.num_variables += 1;
        op_index
    }

    /// Add a relationship between two terms.
    pub fn add_relationship(
        &mut self,
        source: impl Into<QueryVariable>,
        target: impl Into<QueryVariable>,
        relationship_component: ComponentId,
        relationship_target_component: ComponentId,
        accessor: RelationshipAccessor,
        target_accessor: RelationshipAccessor,
    ) -> &mut Self {
        self.operations.push(QueryOperation::Relationship(QueryRelationship {
            source_idx: source.into().index,
            target_idx: target.into().index,
            relationship_component,
            relationship_target_component,
            relationship_accessor: accessor,
            relationship_target_accessor: target_accessor,
        }));
        self
    }

    /// Do some optimizations and return the compiled plan
    pub fn compile(self) -> QueryPlan {
        // TODO: add the relationship/relationshipTarget accesses to each source term
        self
    }
}


#[cfg(test)]
mod tests {
    #![allow(unused_variables)]
    use bevy_ecs::prelude::*;
    use super::*;
    use crate::{
        component::Component,
        hierarchy::ChildOf,
        prelude::World,
    };

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    struct C;

    #[derive(Component)]
    #[relationship(relationship_target = R1Target)]
    struct R1(#[entities] Entity);

    #[derive(Component)]
    #[relationship_target(relationship = R1)]
    struct R1Target(#[entities] Vec<Entity>);

    #[derive(Component)]
    #[relationship(relationship_target = R2Target)]
    struct R2(#[entities] Entity);

    #[derive(Component)]
    #[relationship_target(relationship = R2)]
    struct R2Target(#[entities] Vec<Entity>);

    /// Q1 -> R12 -> Q2
    #[test]
    fn test_query_plan_basic() {
        let mut world = World::new();

        // Correct pair
        let parent = world.spawn_empty().id();
        let child = world.spawn((A, ChildOf(parent))).id();
        world.flush();

        let ptr_a = world.register_component::<A>();

        // Build a simple plan using the builder API
        let mut builder = QueryPlanBuilder::new(&mut world);
        builder.add_source::<(), With<A>>();
        builder.add_relationship::<ChildOf>(0, 1);
        builder.add_source::<(), ()>();
        let plan = builder.compile();

        let iter = plan.query_iter(world.as_unsafe_world_cell());
        let results: Vec<DynamicItem> = iter.collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entities[0].entity(), child);
        assert_eq!(results[0].entities[1].entity(), parent);
    }

    /// Checks that filters in the source or target of the relationship are respected
    /// Q1 -> R12 -> Q2
    #[test]
    fn test_query_plan_single_relationship() {
        let mut world = World::new();

        // Parent does not have the marker
        let parent1 = world.spawn_empty().id();
        let _ = world.spawn((A, ChildOf(parent1))).id();
        world.flush();

        // Child does not have the marker
        let parent2 = world.spawn(A).id();
        let _ = world.spawn(ChildOf(parent2)).id();

        // Both have markers but there is no relationship
        let _ = world.spawn(A).id();
        let _ = world.spawn(A).id();

        // Two correct pairs, (Child, Parent) and (Parent, Grandparent)
        let grandparent4 = world.spawn(A).id();
        let parent4 = world.spawn((A, ChildOf(grandparent4))).id();
        let child4 = world.spawn((A, ChildOf(parent4))).id();

        // Both sources must have the Marker
        let mut builder = QueryPlanBuilder::new(&mut world);
        builder.add_source::<(), With<A>>();
        builder.add_relationship::<ChildOf>(0, 1);
        builder.add_source::<(), With<A>>();
        let plan = builder.compile();

        let iter = plan.query_iter(world.as_unsafe_world_cell());
        let results: Vec<DynamicItem> = iter.collect();

        assert_eq!(results.len(), 2);
        assert_eq!(results[1].entities[0].entity(), parent4);
        assert_eq!(results[1].entities[1].entity(), grandparent4);
        assert_eq!(results[0].entities[0].entity(), child4);
        assert_eq!(results[0].entities[1].entity(), parent4);
    }

    /// Checks that filters in the source or target of the relationship are respected
    /// Q1 -> R12 -> Q2 (but expressed using relationship target)
    #[test]
    fn test_query_plan_single_relationship_target() {
        let mut world = World::new();

        // Parent does not have the marker
        let parent1 = world.spawn_empty().id();
        let _ = world.spawn((A, ChildOf(parent1))).id();
        world.flush();

        // Child does not have the marker
        let parent2 = world.spawn(A).id();
        let _ = world.spawn(ChildOf(parent2)).id();

        // Both have markers but there is no relationship
        let _ = world.spawn(A).id();
        let _ = world.spawn(A).id();

        // Two correct pairs, (Child, Parent) and (Parent, Grandparent)
        let grandparent4 = world.spawn(A).id();
        let parent4 = world.spawn((A, ChildOf(grandparent4))).id();
        let child4 = world.spawn((A, ChildOf(parent4))).id();

        // Both sources must have the Marker
        let mut builder = QueryPlanBuilder::new(&mut world);
        builder.add_source::<Entity, With<A>>();
        builder.add_relationship_target::<Children>(1, 0);
        builder.add_source::<Entity, With<A>>();
        let plan = builder.compile();

        let iter = plan.query_iter(world.as_unsafe_world_cell());
        let results: Vec<DynamicItem> = iter.collect();

        assert_eq!(results.len(), 2);
        assert_eq!(results[1].entities[0].entity(), parent4);
        assert_eq!(results[1].entities[1].entity(), grandparent4);
        assert_eq!(results[0].entities[0].entity(), child4);
        assert_eq!(results[0].entities[1].entity(), parent4);
    }

    /// The first variable $1 is the target of the relationship
    /// Q1 -> R21 -> Q2
    #[test]
    fn test_query_plan_single_relationship_reverse() {
        let mut world = World::new();

        // Parent does not have the marker
        let parent1 = world.spawn_empty().id();
        let _ = world.spawn((A, ChildOf(parent1))).id();
        world.flush();

        // Child does not have the marker
        let parent2 = world.spawn(A).id();
        let _ = world.spawn(ChildOf(parent2)).id();

        // Both have markers but there is no relationship
        let _ = world.spawn(A).id();
        let _ = world.spawn(A).id();

        // Two correct pairs, (Child, Parent) and (Parent, Grandparent)
        let grandparent4 = world.spawn(A).id();
        let parent4 = world.spawn((A, ChildOf(grandparent4))).id();
        let child4 = world.spawn((A, ChildOf(parent4))).id();

        // Both sources must have the Marker
        let mut builder = QueryPlanBuilder::new(&mut world);
        builder.add_source::<Entity, With<A>>();
        builder.add_relationship::<ChildOf>(1, 0);
        builder.add_source::<Entity, With<A>>();
        let plan = builder.compile();

        let iter = plan.query_iter(world.as_unsafe_world_cell());
        let results: Vec<DynamicItem> = iter.collect();

        assert_eq!(results.len(), 2);
        assert_eq!(results[1].entities[0].entity(), parent4);
        assert_eq!(results[1].entities[1].entity(), child4);
        assert_eq!(results[0].entities[0].entity(), grandparent4);
        assert_eq!(results[0].entities[1].entity(), parent4);
    }

    /// Q1 -> R12 -> Q2 -> R23 -> Q3
    #[test]
    fn test_query_plan_multi_relationship() {
        let mut world = World::new();

        // Valid triplet
        let valid_3 = world.spawn(C).id();
        let valid_2 = world.spawn((B, R2(valid_3))).id();
        let valid_1 = world.spawn((A, R1(valid_2))).id();

        // Invalid triplet: the constraint Q3 is not satisfied
        let invalid_3_a = world.spawn_empty().id();
        let invalid_2_a = world.spawn((B, R2(invalid_3_a))).id();
        let _ = world.spawn((A, R1(invalid_2_a))).id();

        // Invalid triplet: the constraint Q3 is not satisfied
        let invalid_3_b = world.spawn(C).id();
        let invalid_2_b = world.spawn(R2(invalid_3_b)).id();
        let _ = world.spawn((A, R1(invalid_2_b))).id();

        // Invalid triplet: the constraint Q1 is not satisfied
        let invalid_3_c = world.spawn(C).id();
        let invalid_2_c = world.spawn((B, R2(invalid_3_c))).id();
        let _ = world.spawn(R1(invalid_2_c)).id();

        // Invalid triplet: the constraint R23 is not satisfied
        let invalid_3_d = world.spawn(C).id();
        let invalid_2_d = world.spawn(B).id();
        let _ = world.spawn((A, R1(invalid_2_d))).id();

        // Invalid triplet: the constraint R12 is not satisfied
        let invalid_3_e = world.spawn(C).id();
        let invalid_2_e = world.spawn((B, R2(invalid_3_e))).id();
        let _ = world.spawn(A).id();

        let mut builder = QueryPlanBuilder::new(&mut world);
        builder.add_source::<(), With<A>>();
        builder.add_relationship::<R1>(0, 1);
        builder.add_source::<(), With<B>>();
        builder.add_relationship::<R2>(1, 2);
        builder.add_source::<(), With<C>>();
        let plan = builder.compile();

        let iter = plan.query_iter(world.as_unsafe_world_cell());
        let results: Vec<DynamicItem> = iter.collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entities[0].entity(), valid_1);
        assert_eq!(results[0].entities[1].entity(), valid_2);
        assert_eq!(results[0].entities[2].entity(), valid_3);
    }


    /// Q1 -> R12 -> Q2 -> R23 -> Q3 -> R13
    /// At the end, we backtrack through R13 but we don't modify the fixed state of $1 and $3 since
    /// they were written in previous ops
    #[test]
    fn test_query_plan_multi_relationship_fixed_backtrack() {
        let mut world = World::new();

        // Valid triplet
        let valid_3 = world.spawn(C).id();
        let valid_2 = world.spawn((B, R1(valid_3))).id();
        let valid_1 = world.spawn((A, ChildOf(valid_2), R2(valid_3))).id();

        // Invalid triplet: the last constraint (R2) is not satisfied
        let invalid_3 = world.spawn(C).id();
        let invalid_2 = world.spawn((B, R1(valid_3))).id();
        let invalid_1 = world.spawn((A, ChildOf(valid_2))).id();

        world.flush();

        // Both sources must have the Marker
        let mut builder = QueryPlanBuilder::new(&mut world);
        builder.add_source::<(), With<A>>();
        builder.add_relationship::<ChildOf>(0, 1);
        builder.add_source::<(), With<B>>();
        builder.add_relationship::<R1>(1, 2);
        builder.add_source::<(), With<C>>();
        builder.add_relationship::<R2>(0, 2);
        let plan = builder.compile();

        let iter = plan.query_iter(world.as_unsafe_world_cell());
        let results: Vec<DynamicItem> = iter.collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entities[0].entity(), valid_1);
        assert_eq!(results[0].entities[1].entity(), valid_2);
        assert_eq!(results[0].entities[2].entity(), valid_3);
    }
}