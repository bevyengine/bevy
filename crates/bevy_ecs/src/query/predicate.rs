use std::marker::PhantomData;

use super::{QueryData, QueryFilter, QueryItem, WorldQuery};

pub trait PredicateFilter {
    type Data: QueryData;
    fn filter_predicate(item: QueryItem<Self::Data>) -> bool;
}

pub struct Predicate<P>(PhantomData<P>);

unsafe impl<D: QueryData, P: PredicateFilter<Data = D>> WorldQuery for Predicate<P> {
    type Item<'a> = D::Item<'a>;

    type Fetch<'a> = D::Fetch<'a>;

    type State = D::State;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        D::shrink(item)
    }

    unsafe fn init_fetch<'w>(
        world: crate::world::unsafe_world_cell::UnsafeWorldCell<'w>,
        state: &Self::State,
        last_run: crate::component::Tick,
        this_run: crate::component::Tick,
    ) -> Self::Fetch<'w> {
        D::init_fetch(world, state, last_run, this_run)
    }

    const IS_DENSE: bool = D::IS_DENSE;

    unsafe fn set_archetype<'w>(
        fetch: &mut Self::Fetch<'w>,
        state: &Self::State,
        archetype: &'w crate::archetype::Archetype,
        table: &'w crate::storage::Table,
    ) {
        D::set_archetype(fetch, state, archetype, table)
    }

    unsafe fn set_table<'w>(
        fetch: &mut Self::Fetch<'w>,
        state: &Self::State,
        table: &'w crate::storage::Table,
    ) {
        D::set_table(fetch, state, table)
    }

    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: crate::entity::Entity,
        table_row: crate::storage::TableRow,
    ) -> Self::Item<'w> {
        D::fetch(fetch, entity, table_row)
    }

    fn update_component_access(
        state: &Self::State,
        access: &mut super::FilteredAccess<crate::component::ComponentId>,
    ) {
        D::update_component_access(state, access)
    }

    fn init_state(world: &mut crate::prelude::World) -> Self::State {
        D::init_state(world)
    }

    fn get_state(components: &crate::component::Components) -> Option<Self::State> {
        D::get_state(components)
    }

    fn matches_component_set(
        state: &Self::State,
        set_contains_id: &impl Fn(crate::component::ComponentId) -> bool,
    ) -> bool {
        D::matches_component_set(state, set_contains_id)
    }
}

impl<D: QueryData, P: PredicateFilter<Data = D>> QueryFilter for Predicate<P> {
    const IS_ARCHETYPAL: bool = false;

    unsafe fn filter_fetch(
        fetch: &mut Self::Fetch<'_>,
        entity: crate::entity::Entity,
        table_row: crate::storage::TableRow,
    ) -> bool {
        P::filter_predicate(Self::fetch(fetch, entity, table_row))
    }
}
