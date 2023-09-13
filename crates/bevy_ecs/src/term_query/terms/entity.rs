use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{ComponentId, Tick},
    entity::Entity,
    prelude::{EntityMut, EntityRef, World},
    query::{Access, DebugCheckedUnwrap, FilteredAccess},
    storage::{Table, TableRow},
    world::unsafe_world_cell::{UnsafeEntityCell, UnsafeWorldCell},
};

use super::{Fetchable, FetchedTerm, QueryTerm, Term, TermAccess};

#[derive(Clone)]
pub struct EntityTerm {
    access: Option<TermAccess>,
}

impl EntityTerm {
    pub fn none() -> Self {
        Self { access: None }
    }

    pub fn read() -> Self {
        Self {
            access: Some(TermAccess::Read),
        }
    }

    pub fn write() -> Self {
        Self {
            access: Some(TermAccess::Write),
        }
    }
}

#[derive(Clone)]
pub struct FetchedEntity<'w> {
    entity: Entity,
    cell: Option<UnsafeEntityCell<'w>>,
}

impl Fetchable for EntityTerm {
    type State<'w> = UnsafeWorldCell<'w>;
    type Item<'w> = FetchedEntity<'w>;

    #[inline]
    unsafe fn init_state<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
    ) -> UnsafeWorldCell<'w> {
        world
    }

    #[inline]
    unsafe fn set_table<'w>(&self, _state: &mut Self::State<'w>, _table: &'w Table) {}

    #[inline]
    unsafe fn fetch<'w>(
        &self,
        world: &Self::State<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> FetchedEntity<'w> {
        FetchedEntity {
            entity,
            cell: if self.access.is_some() {
                Some(world.get_entity(entity).debug_checked_unwrap())
            } else {
                None
            },
        }
    }

    #[inline]
    unsafe fn filter_fetch<'w>(
        &self,
        _state: &Self::State<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> bool {
        true
    }

    #[inline]
    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        debug_assert!(
            self.access.is_none() || !access.access().has_any_write(),
            "EntityTerm has conflicts with a previous access in this query. Exclusive access cannot coincide with any other accesses.",
        );
        match self.access {
            Some(TermAccess::Read) => access.read_all(),
            Some(TermAccess::Write) => access.write_all(),
            None => {}
        }
    }

    #[inline]
    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if let Some(term_access) = &self.access {
            for component_id in archetype.components() {
                match term_access {
                    TermAccess::Read => {
                        access.add_read(archetype.get_archetype_component_id(component_id).unwrap())
                    }
                    TermAccess::Write => access
                        .add_write(archetype.get_archetype_component_id(component_id).unwrap()),
                }
            }
        }
    }

    #[inline]
    fn matches_component_set(&self, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        true
    }
}

impl QueryTerm for Entity {
    type Item<'w> = Self;
    type ReadOnly = Self;

    fn init_term(_world: &mut World) -> Term {
        Term::Entity(EntityTerm::none())
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        let term = term.entity().debug_checked_unwrap();

        term.entity
    }
}

impl QueryTerm for EntityRef<'_> {
    type Item<'w> = EntityRef<'w>;
    type ReadOnly = Self;

    fn init_term(_world: &mut World) -> Term {
        Term::Entity(EntityTerm::read())
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        let term = term.entity().debug_checked_unwrap();
        EntityRef::new(term.cell.debug_checked_unwrap())
    }
}

impl<'r> QueryTerm for EntityMut<'r> {
    type Item<'w> = EntityMut<'w>;
    type ReadOnly = EntityRef<'r>;

    fn init_term(_world: &mut World) -> Term {
        Term::Entity(EntityTerm::write())
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        let term = term.entity().debug_checked_unwrap();
        EntityMut::new(term.cell.debug_checked_unwrap())
    }
}
