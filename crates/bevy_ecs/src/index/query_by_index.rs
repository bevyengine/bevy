use alloc::vec::Vec;

use crate::{
    archetype::Archetype,
    component::{ComponentId, Immutable, Tick},
    prelude::Component,
    query::{QueryBuilder, QueryData, QueryFilter, QueryState, With},
    system::{Query, QueryLens, Res, SystemMeta, SystemParam},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

use super::Index;

/// This system parameter allows querying by an indexable component value.
///
/// # Examples
///
/// ```rust
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::new();
/// #[derive(Component, PartialEq, Eq, Hash, Clone)]
/// #[component(immutable)]
/// struct Player(u8);
///
/// // Indexing is opt-in through `World::add_index`
/// world.add_index(IndexOptions::<Player>::default());
/// # for i in 0..6 {
/// #   for _ in 0..(i + 1) {
/// #       world.spawn(Player(i));
/// #   }
/// # }
/// #
/// # world.flush();
///
/// fn find_all_player_one_entities(by_player: QueryByIndex<Player, Entity>) {
///     let mut lens = by_player.at(&Player(0));
///     
///     for entity in lens.query().iter() {
///         println!("{entity:?} belongs to Player 1!");
///     }
/// #   assert_eq!((
/// #       by_player.at(&Player(0)).query().iter().count(),
/// #       by_player.at(&Player(1)).query().iter().count(),
/// #       by_player.at(&Player(2)).query().iter().count(),
/// #       by_player.at(&Player(3)).query().iter().count(),
/// #       by_player.at(&Player(4)).query().iter().count(),
/// #       by_player.at(&Player(5)).query().iter().count(),
/// #    ), (1, 2, 3, 4, 5, 6));
/// }
/// # world.run_system_cached(find_all_player_one_entities);
/// ```
pub struct QueryByIndex<
    'world,
    'state,
    C: Component<Mutability = Immutable>,
    D: QueryData + 'static,
    F: QueryFilter + 'static = (),
> {
    world: UnsafeWorldCell<'world>,
    state: &'state QueryByIndexState<C, D, F>,
    last_run: Tick,
    this_run: Tick,
    index: Res<'world, Index<C>>,
}

impl<C: Component<Mutability = Immutable>, D: QueryData, F: QueryFilter>
    QueryByIndex<'_, '_, C, D, F>
{
    /// Return a [`QueryLens`] returning entities with a component `C` of the provided value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// #[derive(Component, PartialEq, Eq, Hash, Clone)]
    /// #[component(immutable)]
    /// enum FavoriteColor {
    ///     Red,
    ///     Green,
    ///     Blue,
    /// }
    ///
    /// world.add_index(IndexOptions::<FavoriteColor>::default());
    ///
    /// fn find_red_fans(mut by_color: QueryByIndex<FavoriteColor, Entity>) {
    ///     let mut lens = by_color.at(&FavoriteColor::Red);
    ///
    ///     for entity in lens.query().iter() {
    ///         println!("{entity:?} likes the color Red!");
    ///     }
    /// }
    /// ```
    pub fn at_mut(&mut self, value: &C) -> QueryLens<'_, D, (F, With<C>)>
    where
        QueryState<D, (F, With<C>)>: Clone,
    {
        let state = self.state.primary_query_state.clone();

        // SAFETY: We have registered all of the query's world accesses,
        // so the caller ensures that `world` has permission to access any
        // world data that the query needs.
        unsafe {
            QueryLens::new(
                self.world,
                self.filter_for_value(value, state),
                self.last_run,
                self.this_run,
            )
        }
    }

    /// Return a read-only [`QueryLens`] returning entities with a component `C` of the provided value.
    pub fn at(&self, value: &C) -> QueryLens<'_, D::ReadOnly, (F, With<C>)>
    where
        QueryState<D::ReadOnly, (F, With<C>)>: Clone,
    {
        let state = self.state.primary_query_state.as_readonly().clone();

        // SAFETY: We have registered all of the query's world accesses,
        // so the caller ensures that `world` has permission to access any
        // world data that the query needs.
        unsafe {
            QueryLens::new(
                self.world,
                self.filter_for_value(value, state),
                self.last_run,
                self.this_run,
            )
        }
    }

    fn filter_for_value<T: QueryData, U: QueryFilter>(
        &self,
        value: &C,
        mut state: QueryState<T, U>,
    ) -> QueryState<T, U> {
        match self.index.mapping.get(value) {
            Some(index) => {
                state = (0..self.index.markers.len())
                    .map(|i| (i, 1 << i))
                    .take_while(|&(_, mask)| mask <= self.index.slots.len())
                    .map(|(i, mask)| {
                        if index & mask > 0 {
                            &self.state.with_states[i]
                        } else {
                            &self.state.without_states[i]
                        }
                    })
                    .fold(state, |state, filter| {
                        state.join_filtered(self.world, filter)
                    });
            }
            None => {
                // Create a no-op filter by joining two conflicting filters together.
                let filter = &self.state.with_states[0];
                state = state.join_filtered(self.world, filter);

                let filter = &self.state.without_states[0];
                state = state.join_filtered(self.world, filter);
            }
        }

        state
    }
}

#[doc(hidden)]
pub struct QueryByIndexState<
    C: Component<Mutability = Immutable>,
    D: QueryData + 'static,
    F: QueryFilter + 'static,
> {
    primary_query_state: QueryState<D, (F, With<C>)>,
    index_state: ComponentId,

    // TODO: THERE MUST BE A BETTER WAY
    without_states: Vec<QueryState<(), With<C>>>, // No, With<C> is not a typo
    with_states: Vec<QueryState<(), With<C>>>,
}

impl<C: Component<Mutability = Immutable>, D: QueryData + 'static, F: QueryFilter + 'static>
    QueryByIndexState<C, D, F>
{
    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        let Some(index) = world.get_resource::<Index<C>>() else {
            panic!(
                "Index not setup prior to usage. Please call `app.add_index(IndexOptions::<{}>::default())` during setup",
                disqualified::ShortName::of::<C>(),
            );
        };

        let ids = index.markers.clone();

        let primary_query_state =
            <Query<D, (F, With<C>)> as SystemParam>::init_state(world, system_meta);
        let index_state = <Res<Index<C>> as SystemParam>::init_state(world, system_meta);

        let with_states = ids
            .iter()
            .map(|&id| QueryBuilder::new(world).with_id(id).build())
            .collect::<Vec<_>>();

        let without_states = ids
            .iter()
            .map(|&id| QueryBuilder::new(world).without_id(id).build())
            .collect::<Vec<_>>();

        Self {
            primary_query_state,
            index_state,
            without_states,
            with_states,
        }
    }

    unsafe fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
        <Query<D, (F, With<C>)> as SystemParam>::new_archetype(
            &mut self.primary_query_state,
            archetype,
            system_meta,
        );

        for state in self
            .with_states
            .iter_mut()
            .chain(self.without_states.iter_mut())
        {
            <Query<(), With<C>> as SystemParam>::new_archetype(state, archetype, system_meta);
        }
    }

    #[inline]
    unsafe fn validate_param(&self, system_meta: &SystemMeta, world: UnsafeWorldCell) -> bool {
        let mut valid = true;

        valid &= <Query<D, (F, With<C>)> as SystemParam>::validate_param(
            &self.primary_query_state,
            system_meta,
            world,
        );
        valid &=
            <Res<Index<C>> as SystemParam>::validate_param(&self.index_state, system_meta, world);

        for state in self.with_states.iter().chain(self.without_states.iter()) {
            valid &= <Query<(), With<C>> as SystemParam>::validate_param(state, system_meta, world);
        }

        valid
    }
}

// SAFETY: We rely on the known-safe implementations of `SystemParam` for `Res` and `Query`.
unsafe impl<C: Component<Mutability = Immutable>, D: QueryData + 'static, F: QueryFilter + 'static>
    SystemParam for QueryByIndex<'_, '_, C, D, F>
{
    type State = QueryByIndexState<C, D, F>;
    type Item<'w, 's> = QueryByIndex<'w, 's, C, D, F>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        Self::State::init_state(world, system_meta)
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        Self::State::new_archetype(state, archetype, system_meta);
    }

    #[inline]
    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        Self::State::validate_param(state, system_meta, world)
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        state.primary_query_state.validate_world(world.id());

        let index = <Res<Index<C>> as SystemParam>::get_param(
            &mut state.index_state,
            system_meta,
            world,
            change_tick,
        );

        QueryByIndex {
            world,
            state,
            last_run: system_meta.last_run,
            this_run: change_tick,
            index,
        }
    }
}
