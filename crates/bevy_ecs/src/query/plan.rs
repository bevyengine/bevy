use alloc::borrow::Cow;
use crate::{
    component::ComponentId,
    entity::Entity,
    query::FilteredAccess,
    world::unsafe_world_cell::UnsafeWorldCell,
};
use alloc::vec::Vec;
use alloc::vec;
use fixedbitset::FixedBitSet;
use bevy_ecs::archetype::Archetype;
use bevy_ecs::component::{Component};
use bevy_ecs::prelude::{QueryBuilder, World};
use bevy_ecs::query::{QueryData, QueryFilter};
use bevy_ecs::relationship::Relationship;
use bevy_ecs::storage::{SparseSetIndex, TableId};
use bevy_ecs::world::unsafe_world_cell::UnsafeEntityCell;
use crate::relationship::RelationshipAccessor;

/// Represents a single source in a multi-source query.
/// Each term has its own ComponentAccess requirements.
#[derive(Debug, Clone)]
pub enum QueryOperation {
    Relationship(QueryRelationship),
    Source(QuerySource),
}

#[derive(Debug, Clone)]
pub struct QuerySource {
    access: FilteredAccess,
    variable_idx: u8,
}


impl QuerySource {
    pub fn matches(&self, archetype: &Archetype) -> bool {
        self.matches_component_set(&|id| archetype.contains(id))
    }

    /// Returns `true` if this query matches a set of components. Otherwise, returns `false`.
    pub fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        self.access.filter_sets.iter().any(|set| {
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
    /// The source term index within the QueryPlan
    pub source_idx: u8,
    /// The target term index within the QueryPlan
    pub target_idx: u8,
    /// The relationship component that links source to target.
    pub relationship_component: ComponentId,
    /// Accessor to dynamically access the 'target' of the relationship
    pub relationship_accessor: RelationshipAccessor,
}

impl QueryRelationship {
    /// Get the 'target' of the source entity
    pub unsafe fn get(&self, source: Entity, world: UnsafeWorldCell<'_>) -> Option<Entity> {
        let entity_field_offset = match self.relationship_accessor {
            RelationshipAccessor::Relationship { entity_field_offset, .. } => {entity_field_offset}
            RelationshipAccessor::RelationshipTarget { .. } => {
                unreachable!()
            }
        };
        let relationship_ptr = world.get_entity(source).ok()?.get_by_id(self.relationship_component)?;
        Some(unsafe { *relationship_ptr.byte_add(entity_field_offset).deref() })
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

    pub fn add_source_from_builder(&mut self, f: impl Fn(QueryBuilder) -> QueryBuilder) -> &mut Self {
        let query_builder = QueryBuilder::new(&mut self.world);
        let builder = f(query_builder);
        self.plan.add_source(builder.access().clone());
        self
    }

    pub fn add_source<D: QueryData, F: QueryFilter>(&mut self) -> &mut Self {
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

        self.plan.add_source(access);
        self
    }


    /// Add a relationship between two terms using a typed Relationship component.
    pub fn add_relationship<R: Relationship>(
        &mut self,
        source_term: u8,
        target_term: u8,
    ) {
        let component_id = self.world.register_component::<R>();
        let accessor = self.world
            .components()
            .get_info(component_id)
            .unwrap()
            .relationship_accessor()
            .unwrap().clone();

        self.plan.add_relationship(
            source_term,
            target_term,
            component_id,
            accessor,
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
#[derive(Debug, Default, Clone)]
pub struct QueryPlan {
    /// All operations in this query.
    pub operations: Vec<QueryOperation>,
    pub num_variables: u8,
}


impl QueryPlan {
    pub fn query_iter<'w, 's>(&'s self, world: UnsafeWorldCell<'w>) -> Iter<'w, 's> {
        Iter {
            world,
            query_state: Cow::Borrowed(self),
            iter_state: IterState::new(self),
        }
    }
}


#[derive(Debug, Copy, Clone)]
pub enum VariableState {
    // we are searching through all the entities of a table
    Search {
        table: Option<TableId>,
        // offset in the current table
        offset: u32,
        // length of the current table
        current_len: u32,
        // index of the table inside the StorageState
        storage_idx: u32,
    },
    // An entity has been found by following a relationship
    FixedByRelationship(Entity)
}

impl Default for VariableState {
    fn default() -> Self {
        Self::Search {
            table: None,
            offset: 0,
            current_len: 0,
            storage_idx: 0,
        }
    }
}


#[derive(Default, Clone)]
pub struct StorageState<'w> {
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
    /// Whether we have already found an Entity for the source when we are about to process the next operation
    written: Vec<FixedBitSet>,
}

impl<'w> IterState<'w> {
    fn new(plan: &QueryPlan) -> Self {
        let num_operations = plan.operations.len();
        let num_variables = plan.num_variables as usize;
        let mut variable_state =  vec![VariableState::default(); num_variables];
        let mut storages_state = vec![StorageState::default(); num_variables];
        let mut written = vec![FixedBitSet::with_capacity(num_variables); num_operations];
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
    query_state: Cow<'s, QueryPlan>,
    iter_state: IterState<'s>,
}


impl<'w, 's> Iter<'w, 's> {

    /// Returns true if the `variable` was currently assigned an Entity
    fn written(&self, variable: u8) -> bool {
        self.iter_state.written[self.iter_state.curr_op as usize].contains(variable as usize)
    }

    /// Get the entity currently written to the variable indexed by `variable`, if it has been written
    fn written_entity(&self, variable: u8) -> Option<Entity> {
        let variable_idx = variable as usize;
        Some(match self.iter_state.variable_state[variable_idx] {
            VariableState::Search { table, offset, .. } => {
                let table_id = table?;
                // Safety:
                let table = unsafe { self.world.storages() }.tables.get(table_id)?;
                unsafe{ *table.entities().get_unchecked(offset as usize) }
            }
            VariableState::FixedByRelationship(entity) => {entity}
        })
    }

    // we found entity1, QueryOp:Rel -> set entity 2 to one of the entities via the RelationShip
    fn dispatch(&mut self) -> bool {
        let op = self.current_op();
        match op {
            QueryOperation::Relationship(_) => self.op_relationship(),
            QueryOperation::Source(_) => self.op_query(),
        }
    }

    /// Try to find an entity via the relationship
    fn op_relationship(&mut self) -> bool {
        if self.iter_state.backtrack {
            return false;
        }
        let QueryOperation::Relationship(rel) = self.current_op() else {
            unreachable!()
        };
        // TODO: do we always find the source first? what if the target was written but not the source?
        debug_assert!(self.written(rel.source_idx), "we only support queries where the source has been found before we are querying the relationship");
        // we already found the target term!
        if self.written(rel.target_idx) {
            let target_state = self.iter_state.variable_state[rel.target_idx as usize];
            match target_state {
                VariableState::Search { .. } => {
                    todo!("Check if target_entity is equal to the relationship.get() value")
                }
                VariableState::FixedByRelationship(_) => {
                    true
                }
            }
        } else {
            // need to find the target term! We do this by simply following the relationship.get()
            let source_entity = self.written_entity(rel.source_idx).unwrap();
            // it is guaranteed that the target exists, since the Relationship component is present
            // on the source entity
            // SAFETY: TODO
            let target_entity = unsafe { rel.get(source_entity, self.world).unwrap_unchecked() };
            self.set_variable_state(rel.target_idx, VariableState::FixedByRelationship(target_entity));
            true
        }
    }

    fn op_query(&mut self) -> bool {
        let QueryOperation::Source(source) = self.current_op() else {
            unreachable!()
        };
        let variable_idx = source.variable_idx as usize;

        // we already have a potential candidate: check if it matches the query
        if let VariableState::FixedByRelationship(entity) = self.iter_state.variable_state[variable_idx] {
            // in this case keep backtracking
            if self.iter_state.backtrack {
                return false
            }
            let archetype = unsafe { self.world.get_entity(entity).unwrap_unchecked().archetype() };
            return source.matches(archetype)
        }

        // we haven't found the entity yet.
        // - TODO: either we already had some constraints on the entity (i.e. a potential list of archetypes
        //    that this entity can be part of), in which case we can further filter down these archetypes
        //    -> this can only happen if we allow multiple queries for a single variable
        //
        // we need to use the component index to find a list of potential archetypes
        // that could match the query

        // TODO: what about caching?

        // the first time we evaluate the query, we set the list of potential tables
        if !self.iter_state.backtrack {
            let tables = &self.iter_state.storages_state[variable_idx].tables;
            // only set the list of potential tables if we didn't do so before
            if tables.len() == 0 {
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
                let mut matching_tables = Vec::new();
                if let Some(archetypes) = potential_archetypes {
                    for archetype_id in archetypes {
                        // SAFETY: get_potential_archetypes only returns archetype ids that are valid for the world
                        let archetype = &self.world.archetypes()[*archetype_id];
                        if source.matches(archetype) {
                            matching_tables.push(archetype.table_id())
                        }
                    }
                }
                let Some(&table) = matching_tables.first() else {
                    return false;
                };
                self.iter_state.storages_state[variable_idx] = StorageState {
                    tables: Cow::Owned(matching_tables)
                };
                self.set_variable_state(variable_idx as u8, VariableState::Search {
                    table: Some(table),
                    offset: 0,
                    storage_idx: 0,
                    current_len: 0,
                });
                return true;
            }
        }

        // else we backtracked! we need to advance in the current table, or go to the next table
        let storage_state = &self.iter_state.storages_state[variable_idx];
        let target_state = &mut self.iter_state.variable_state[variable_idx];
        let VariableState::Search { table: Some(table_id), offset, current_len, storage_idx, } = target_state else {
            unreachable!();
        };
        if *storage_idx >= storage_state.tables.len() as u32 {
            return false
        }
        let iteration_start = *current_len == 0;
        // loop to skip empty tables
        loop {
            // either beginning of the iteration, or finished processing a table, so skip to the next
            if offset == current_len {
                // go to next table
                if !iteration_start {
                    *storage_idx += 1;
                    if *storage_idx >= storage_state.tables.len() as u32 {
                        return false
                    }
                    *table_id = storage_state.tables[*storage_idx as usize];
                    *offset = 0;
                }
                let table = unsafe { self.world.world().storages().tables.get(*table_id).unwrap_unchecked() };
                *current_len = table.entity_count();
                let table = unsafe { self.world.world().storages().tables.get(*table_id).unwrap_unchecked() };
                if table.is_empty() {
                    // skip table
                    continue;
                }
            }

            // TODO: store `table.entities()` somewhere so we don't have to fetch it again every time
            // let table = unsafe { self.world.world().storages().tables.get(*table_id).unwrap_unchecked() };
            // let entity = unsafe{ table.entities().get_unchecked(*offset as usize) };
            *offset += 1;

            // this entity is our current candidate
            return true;
        }
    }

    /// Assign the variable state (and also set the entity as written for the next op)
    fn set_variable_state(&mut self, variable: u8, state: VariableState) {
        // the operation writes the entity as written for the **next** operation
        let curr_op = self.iter_state.curr_op as usize;
        let variable_idx = variable as usize;
        if curr_op + 1 < self.iter_state.written.len() {
            self.iter_state.written[curr_op + 1].grow_and_insert(variable_idx);
        }
        self.iter_state.variable_state[variable_idx] = state;
    }

    fn current_op(&self) -> &QueryOperation {
        &self.query_state.operations[self.iter_state.curr_op as usize]
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
            let op_index = self.iter_state.curr_op as usize;
            let matched = self.dispatch();

            if !matched {
                // Operation did not match, return to previous op
                self.iter_state.backtrack = true;
                // Returned from all operations, no more matches
                if self.iter_state.curr_op == 0 {
                    return false;
                }
                self.iter_state.curr_op -= 1;
            } else {
                // Operation did match, move to next op
                self.iter_state.backtrack = false;
                self.iter_state.curr_op += 1;

                if self.iter_state.curr_op < num_operations {
                    // Setup written state for next operation. The ops themselves have already updated the written state for the next
                    // op, but we also need to propagate the existing written from the current op

                    // (we have a written bitset for each operation so that on backtracking we can retrieve the previous `written` state)
                    let (written_next_op, written_op) = self.iter_state.written.get_mut(op_index..op_index + 2).unwrap().split_at_mut(1);
                    // written[op_index + 1].union_with(written[op_index])
                    written_next_op[0].union_with(&written_op[0]);
                }
            }
        }
        true
    }
}

pub struct DynamicItem<'w> {
    // TODO: return FilteredEntityRef/FilteredEntityMut by borrowing from the State, which contains the access.
    entities: Vec<UnsafeEntityCell<'w>>,
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
    pub(crate) fn add_source(&mut self, access: FilteredAccess) -> usize {
        let op_index = self.operations.len();
        self.operations.push(QueryOperation::Source(QuerySource {
            access,
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
        accessor: RelationshipAccessor,
    ) -> &mut Self {
        self.operations.push(QueryOperation::Relationship(QueryRelationship {
            source_idx: source.into().index,
            target_idx: target.into().index,
            relationship_component,
            relationship_accessor: accessor,
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
    use bevy_ecs::prelude::*;
    use super::*;
    use crate::{
        component::Component,
        hierarchy::ChildOf,
        prelude::World,
    };

    #[derive(Component)]
    struct Marker;

    #[test]
    fn test_query_plan_basic() {
        let mut world = World::new();

        // Correct pair
        let parent = world.spawn_empty().id();
        let child = world.spawn((Marker, ChildOf(parent))).id();
        world.flush();

        // Build a simple plan using the builder API
        let mut builder = QueryPlanBuilder::new(&mut world);
        builder.add_source::<&Marker, ()>();
        builder.add_relationship::<ChildOf>(0, 1);
        builder.add_source::<Entity, ()>();
        let plan = builder.compile();

        let iter = plan.query_iter(world.as_unsafe_world_cell());
        let results: Vec<DynamicItem> = iter.collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entities[0].entity(), child);
        assert_eq!(results[0].entities[1].entity(), parent);
    }

    /// Checks that filters in the source or target of the relationship are respected
    #[test]
    fn test_query_plan_single_relationship() {
        let mut world = World::new();

        // Parent does not have the marker
        let parent1 = world.spawn_empty().id();
        let child1 = world.spawn((Marker, ChildOf(parent1))).id();
        world.flush();

        // Child does not have the marker
        let parent2 = world.spawn(Marker).id();
        let child2 = world.spawn(ChildOf(parent2)).id();

        // Both have markers but there is no relationship
        let parent3 = world.spawn(Marker).id();
        let child3 = world.spawn(Marker).id();

        // Two correct pairs, (Child, Parent) and (Parent, Grandparent)
        let grandparent4 = world.spawn(Marker).id();
        let parent4 = world.spawn((Marker, ChildOf(grandparent4))).id();
        let child4 = world.spawn((Marker, ChildOf(parent4))).id();

        // Both sources must have the Marker
        let mut builder = QueryPlanBuilder::new(&mut world);
        builder.add_source::<&Marker, ()>();
        builder.add_relationship::<ChildOf>(0, 1);
        builder.add_source::<Entity, With<Marker>>();
        let plan = builder.compile();

        let iter = plan.query_iter(world.as_unsafe_world_cell());
        let results: Vec<DynamicItem> = iter.collect();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].entities[0].entity(), child4);
        assert_eq!(results[0].entities[1].entity(), parent4);
        assert_eq!(results[1].entities[0].entity(), parent4);
        assert_eq!(results[2].entities[1].entity(), grandparent4);
    }
}

