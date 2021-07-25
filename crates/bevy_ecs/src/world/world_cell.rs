use crate::{
    archetype::ArchetypeComponentId,
    component::{Component, ComponentId},
    prelude::{Bundle, Entity},
    query::{FilterFetch, FilteredAccess, QueryIter, QueryState, WorldQuery},
    storage::SparseSet,
    system::{CommandQueue, Despawn, Insert, InsertBundle, Remove, RemoveBundle, Resource},
    world::{Mut, World},
};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};

/// Exposes safe mutable access to multiple resources at a time in a World. Attempting to access
/// World in a way that violates Rust's mutability rules will panic thanks to runtime checks.
pub struct WorldCell<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) state: WorldCellState,
}

struct QueryCacheEntry<Q: ?Sized + DynQueryState = dyn DynQueryState> {
    alive_count: Cell<u32>,
    in_working_set: Cell<bool>,
    query: Q,
}

impl QueryCacheEntry {
    fn alive_filtered_access(&self) -> Option<&FilteredAccess<ComponentId>> {
        if self.alive_count.get() > 0 {
            Some(self.query.component_access())
        } else {
            None
        }
    }
}

trait DynQueryState: Any {
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

pub(crate) struct WorldCellState {
    resource_access: RefCell<ArchetypeComponentAccess>,
    query_cache: HashMap<TypeId, Rc<QueryCacheEntry>, fxhash::FxBuildHasher>,
    /// Queries that were activated at least once in the current WorldCell session.
    query_cache_working_set: RefCell<Vec<Rc<QueryCacheEntry>>>,
    command_queue: RefCell<CommandQueue>,
}

impl WorldCellState {
    // cannot be const because of hashmap, but should still be optimized out
    #[inline]
    pub fn new() -> Self {
        Self {
            resource_access: RefCell::new(ArchetypeComponentAccess::new()),
            // component_access: RefCell::new(ComponentAccess::new()),
            query_cache: HashMap::default(),
            query_cache_working_set: Default::default(),
            command_queue: Default::default(),
        }
    }

    fn get_live_query_conflicts_filtered(
        &self,
        filtered_access: &FilteredAccess<ComponentId>,
    ) -> Vec<ComponentId> {
        for query in self.query_cache_working_set.borrow().iter() {
            if let Some(current_filtered_access) = query.alive_filtered_access() {
                if !current_filtered_access.is_compatible(filtered_access) {
                    return current_filtered_access
                        .access()
                        .get_conflicts(filtered_access.access());
                }
            }
        }
        Vec::new()
    }
}

pub(crate) struct ArchetypeComponentAccess {
    access: SparseSet<ArchetypeComponentId, u32>,
}

const UNIQUE_ACCESS: u32 = 0;
const BASE_ACCESS: u32 = 1;
impl ArchetypeComponentAccess {
    const fn new() -> Self {
        Self {
            access: SparseSet::new(),
        }
    }

    fn read(&mut self, id: ArchetypeComponentId) -> bool {
        let id_access = self.access.get_or_insert_with(id, || BASE_ACCESS);
        if *id_access == UNIQUE_ACCESS {
            false
        } else {
            *id_access += 1;
            true
        }
    }

    fn drop_read(&mut self, id: ArchetypeComponentId) {
        let id_access = self.access.get_or_insert_with(id, || BASE_ACCESS);
        *id_access -= 1;
    }

    fn write(&mut self, id: ArchetypeComponentId) -> bool {
        let id_access = self.access.get_or_insert_with(id, || BASE_ACCESS);
        if *id_access == BASE_ACCESS {
            *id_access = UNIQUE_ACCESS;
            true
        } else {
            false
        }
    }

    fn drop_write(&mut self, id: ArchetypeComponentId) {
        let id_access = self.access.get_or_insert_with(id, || BASE_ACCESS);
        *id_access = BASE_ACCESS;
    }
}

impl<'w> Drop for WorldCell<'w> {
    fn drop(&mut self) {
        self.maintain();

        // give world ArchetypeComponentAccess back to reuse allocations
        let _ = std::mem::swap(&mut self.world.world_cell_state, &mut self.state);
    }
}

pub struct WorldCellRes<'w, T> {
    value: &'w T,
    archetype_component_id: ArchetypeComponentId,
    state: &'w WorldCellState,
}

impl<'w, T> WorldCellRes<'w, T> {
    fn new(
        value: &'w T,
        archetype_component_id: ArchetypeComponentId,
        state: &'w WorldCellState,
    ) -> Self {
        if !state
            .resource_access
            .borrow_mut()
            .read(archetype_component_id)
        {
            panic!(
                "Attempted to immutably access {}, but it is already mutably borrowed",
                std::any::type_name::<T>()
            )
        }
        Self {
            value,
            archetype_component_id,
            state,
        }
    }
}

impl<'w, T> Deref for WorldCellRes<'w, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T> Drop for WorldCellRes<'w, T> {
    fn drop(&mut self) {
        let mut access = self.state.resource_access.borrow_mut();
        access.drop_read(self.archetype_component_id);
    }
}

pub struct WorldCellResMut<'w, T> {
    value: Mut<'w, T>,
    archetype_component_id: ArchetypeComponentId,
    state: &'w WorldCellState,
}

impl<'w, T> WorldCellResMut<'w, T> {
    fn new(
        value: Mut<'w, T>,
        archetype_component_id: ArchetypeComponentId,
        state: &'w WorldCellState,
    ) -> Self {
        if !state
            .resource_access
            .borrow_mut()
            .write(archetype_component_id)
        {
            panic!(
                "Attempted to mutably access {}, but it is already mutably borrowed",
                std::any::type_name::<T>()
            )
        }
        Self {
            value,
            archetype_component_id,
            state,
        }
    }
}

impl<'w, T> Deref for WorldCellResMut<'w, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value.deref()
    }
}

impl<'w, T> DerefMut for WorldCellResMut<'w, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.value
    }
}

impl<'w, T> Drop for WorldCellResMut<'w, T> {
    fn drop(&mut self) {
        let mut access = self.state.resource_access.borrow_mut();
        access.drop_write(self.archetype_component_id);
    }
}

impl<'w> WorldCell<'w> {
    pub(crate) fn new(world: &'w mut World) -> Self {
        // this is cheap because WorldCellState::new() is const / allocation free
        let state = std::mem::replace(&mut world.world_cell_state, WorldCellState::new());
        // world's WorldCellState is recycled to cut down on allocations
        Self { world, state }
    }

    pub fn spawn(&self) -> CellEntityCommands<'_> {
        self.entity(self.world.entities.reserve_entity())
    }

    pub fn entity(&self, entity: Entity) -> CellEntityCommands<'_> {
        CellEntityCommands {
            entity,
            state: &self.state,
        }
    }

    /// A WorldCell session "barrier". Applies world commands issued thus far, optimizing future query accesses.
    pub fn maintain(&mut self) {
        // Clear working set when the WorldCell session ends.
        for entry in self.state.query_cache_working_set.get_mut().drain(..) {
            entry.in_working_set.set(false);
        }
        self.state.command_queue.borrow_mut().apply(self.world);
    }

    pub fn get_resource<T: Resource>(&self) -> Option<WorldCellRes<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldCellRes::new(
            // SAFE: ComponentId matches TypeId
            unsafe { self.world.get_resource_with_id(component_id)? },
            archetype_component_id,
            &self.state,
        ))
    }

    pub fn get_resource_mut<T: Resource>(&self) -> Option<WorldCellResMut<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldCellResMut::new(
            // SAFE: ComponentId matches TypeId and access is checked by WorldCellResMut
            unsafe {
                self.world
                    .get_resource_unchecked_mut_with_id(component_id)?
            },
            archetype_component_id,
            &self.state,
        ))
    }

    pub fn get_non_send<T: 'static>(&self) -> Option<WorldCellRes<'_, T>> {
        let world = &self.world;
        let component_id = world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldCellRes::new(
            // SAFE: ComponentId matches TypeId
            unsafe { world.get_non_send_with_id(component_id)? },
            archetype_component_id,
            &self.state,
        ))
    }

    pub fn get_non_send_mut<T: 'static>(&self) -> Option<WorldCellResMut<'_, T>> {
        let world = &self.world;
        let component_id = world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldCellResMut::new(
            // SAFE: ComponentId matches TypeId and access is checked by WorldCellResMut
            unsafe { world.get_non_send_unchecked_mut_with_id(component_id)? },
            archetype_component_id,
            &self.state,
        ))
    }

    pub fn init_query<Q: WorldQuery + 'static>(&mut self) -> QueryToken<Q, ()> {
        self.init_filtered_query()
    }

    pub fn init_filtered_query<Q, F>(&mut self) -> QueryToken<Q, F>
    where
        Q: WorldQuery + 'static,
        F: WorldQuery + 'static,
        F::Fetch: FilterFetch,
    {
        let key = TypeId::of::<QueryState<Q, F>>();
        let world = &mut self.world;
        self.state.query_cache.entry(key).or_insert_with(|| {
            Rc::new(QueryCacheEntry {
                alive_count: Cell::new(0),
                in_working_set: Cell::new(false),
                query: world.query_filtered::<Q, F>(),
            })
        });

        QueryToken(PhantomData)
    }

    /// Requires `init_query` with the right type to be called beforehand
    pub fn query<Q, F>(&self, token: QueryToken<Q, F>) -> CellQuery<Q, F>
    where
        Q: WorldQuery + 'static,
        F: WorldQuery + 'static,
        F::Fetch: FilterFetch,
    {
        // token is only used to statically pass the query initialization state
        let _ = token;

        let key = TypeId::of::<QueryState<Q, F>>();
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

/// A list of commands that will be run to modify an [`Entity`] inside `WorldCell`.
pub struct CellEntityCommands<'a> {
    entity: Entity,
    state: &'a WorldCellState,
}

impl<'a> CellEntityCommands<'a> {
    /// Retrieves the current entity's unique [`Entity`] id.
    #[inline]
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Adds a [`Bundle`] of components to the current entity.
    pub fn insert_bundle(&mut self, bundle: impl Bundle) -> &mut Self {
        self.state.command_queue.borrow_mut().push(InsertBundle {
            entity: self.entity,
            bundle,
        });
        self
    }

    /// Adds a single [`Component`] to the current entity.
    ///
    /// `Self::insert` can be chained with [`WorldCell::spawn`].
    ///
    /// See [`Commands::insert`] for analogous method in [`Commands`].
    pub fn insert(&mut self, component: impl Component) -> &mut Self {
        self.state.command_queue.borrow_mut().push(Insert {
            entity: self.entity,
            component,
        });
        self
    }

    /// See [`EntityMut::remove_bundle`](crate::world::EntityMut::remove_bundle).
    pub fn remove_bundle<T>(&mut self) -> &mut Self
    where
        T: Bundle,
    {
        self.state
            .command_queue
            .borrow_mut()
            .push(RemoveBundle::<T> {
                entity: self.entity,
                phantom: PhantomData,
            });
        self
    }

    /// See [`EntityMut::remove`](crate::world::EntityMut::remove).
    pub fn remove<T>(&mut self) -> &mut Self
    where
        T: Component,
    {
        self.state.command_queue.borrow_mut().push(Remove::<T> {
            entity: self.entity,
            phantom: PhantomData,
        });
        self
    }

    /// Despawns only the specified entity, not including its children.
    pub fn despawn(&mut self) {
        self.state.command_queue.borrow_mut().push(Despawn {
            entity: self.entity,
        })
    }
}

#[derive(Clone, Copy)]
pub struct QueryToken<Q, F = ()>(PhantomData<(Q, F)>)
where
    Q: WorldQuery + 'static,
    F: WorldQuery + 'static,
    F::Fetch: FilterFetch;

pub struct CellQuery<'w, Q, F> {
    query_entry: Rc<QueryCacheEntry>,
    state: &'w WorldCellState,
    world: &'w World,
    marker: PhantomData<(Q, F)>,
}

impl<'w, Q, F> CellQuery<'w, Q, F>
where
    Q: WorldQuery + 'static,
    F: WorldQuery + 'static,
    F::Fetch: FilterFetch,
{
    #[allow(dead_code)]
    fn iter(&self) -> CellQueryIter<'w, '_, Q, F> {
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

pub struct CellQueryIter<'w, 's, Q, F>
where
    Q: WorldQuery,
    F: WorldQuery,
    F::Fetch: FilterFetch,
{
    inner: QueryIter<'w, 's, Q, F>,
    // Rc holds data referenced in `inner`. Must be dropped last.
    // That Rc is normally held inside `WorldCellState` anyway, but holding it directly allows to guarantee
    // safety easier, as `WorldCellState` is now free to evict cache at any time without consequences
    query_entry: Rc<QueryCacheEntry>,
}

impl<'w, 's, Q, F> Drop for CellQueryIter<'w, 's, Q, F>
where
    Q: WorldQuery,
    F: WorldQuery,
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
    Q: WorldQuery + 'static,
    F: WorldQuery + 'static,
    F::Fetch: FilterFetch,
{
    fn new(cell_query: &'s CellQuery<'w, Q, F>) -> Self {
        let query = cell_query
            .query_entry
            .query
            .as_any()
            .downcast_ref::<QueryState<Q, F>>()
            .unwrap();
        // cast away the query_entry lifetime, so we can return an iterator that's self-referential
        // SAFETY:
        // - we hold onto the entry Rc for the entire lifetime of this reference, as it's cloned into returned WorldCellIter
        let query = unsafe { (query as *const QueryState<Q, F>).as_ref().unwrap() };

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

        Self { query_entry, inner }
    }
}

impl<'w, 's, Q, F> Iterator for CellQueryIter<'w, 's, Q, F>
where
    Q: WorldQuery,
    F: WorldQuery,
    F::Fetch: FilterFetch,
{
    type Item = <QueryIter<'w, 's, Q, F> as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'w, 's, Q, F> ExactSizeIterator for CellQueryIter<'w, 's, Q, F>
where
    Q: WorldQuery,
    F: WorldQuery,
    F::Fetch: FilterFetch,
    QueryIter<'w, 's, Q, F>: ExactSizeIterator,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}

#[cfg(test)]
mod tests {

    use super::BASE_ACCESS;
    use crate::{
        self as bevy_ecs,
        archetype::ArchetypeId,
        component::Component,
        prelude::Without,
        world::{QueryToken, World, WorldCell},
    };
    use std::any::TypeId;

    #[test]
    fn world_cell() {
        let mut world = World::default();
        world.insert_resource(1u32);
        world.insert_resource(1u64);
        let cell = world.cell();
        {
            let mut a = cell.get_resource_mut::<u32>().unwrap();
            assert_eq!(1, *a);
            *a = 2;
        }
        {
            let a = cell.get_resource::<u32>().unwrap();
            assert_eq!(2, *a, "ensure access is dropped");

            let b = cell.get_resource::<u32>().unwrap();
            assert_eq!(
                2, *b,
                "ensure multiple immutable accesses can occur at the same time"
            );
        }
        {
            let a = cell.get_resource_mut::<u32>().unwrap();
            assert_eq!(
                2, *a,
                "ensure both immutable accesses are dropped, enabling a new mutable access"
            );

            let b = cell.get_resource::<u64>().unwrap();
            assert_eq!(
                1, *b,
                "ensure multiple non-conflicting mutable accesses can occur at the same time"
            );
        }
    }

    #[test]
    fn world_access_reused() {
        let mut world = World::default();
        world.insert_resource(1u32);
        {
            let cell = world.cell();
            {
                let mut a = cell.get_resource_mut::<u32>().unwrap();
                assert_eq!(1, *a);
                *a = 2;
            }
        }

        let u32_component_id = world
            .components
            .get_resource_id(TypeId::of::<u32>())
            .unwrap();
        let resource_archetype = world.archetypes.get(ArchetypeId::RESOURCE).unwrap();
        let u32_archetype_component_id = resource_archetype
            .get_archetype_component_id(u32_component_id)
            .unwrap();
        assert_eq!(
            world.world_cell_state.resource_access.borrow().access.len(),
            1
        );
        assert_eq!(
            world
                .world_cell_state
                .resource_access
                .borrow()
                .access
                .get(u32_archetype_component_id),
            Some(&BASE_ACCESS),
            "reused access count is 'base'"
        );
    }

    #[test]
    #[should_panic]
    fn world_cell_double_mut() {
        let mut world = World::default();
        world.insert_resource(1u32);
        let cell = world.cell();
        let _value_a = cell.get_resource_mut::<u32>().unwrap();
        let _value_b = cell.get_resource_mut::<u32>().unwrap();
    }

    #[test]
    #[should_panic]
    fn world_cell_ref_and_mut() {
        let mut world = World::default();
        world.insert_resource(1u32);
        let cell = world.cell();
        let _value_a = cell.get_resource::<u32>().unwrap();
        let _value_b = cell.get_resource_mut::<u32>().unwrap();
    }

    #[test]
    #[should_panic]
    fn world_cell_mut_and_ref() {
        let mut world = World::default();
        world.insert_resource(1u32);
        let cell = world.cell();
        let _value_a = cell.get_resource_mut::<u32>().unwrap();
        let _value_b = cell.get_resource::<u32>().unwrap();
    }

    #[test]
    #[should_panic]
    fn world_cell_ref_and_ref() {
        let mut world = World::default();
        world.insert_resource(1u32);
        let cell = world.cell();
        let _value_a = cell.get_resource_mut::<u32>().unwrap();
        let _value_b = cell.get_resource::<u32>().unwrap();
    }

    #[derive(Component, Debug, Clone, PartialEq)]
    struct A;
    #[derive(Component, Debug, Clone, PartialEq)]
    struct B;

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
}
