use crate::{
    component::{ComponentId, Tick},
    query::{QueryBuilder, QueryData, QueryFilter, QueryState, With},
    system::{Query, Res, SystemMeta, SystemParam},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

use super::{Index, IndexableComponent};

/// This system parameter allows querying by an [indexable component](`IndexableComponent`) value.
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
/// world.add_index::<Player>();
/// # for i in 0..6 {
/// #   for _ in 0..(i + 1) {
/// #       world.spawn(Player(i));
/// #   }
/// # }
/// #
/// # world.flush();
///
/// fn find_all_player_one_entities(mut query: QueryByIndex<Player, Entity>) {
///     for entity in query.at(&Player(0)).iter() {
///         println!("{entity:?} belongs to Player 1!");
///     }
/// #   assert_eq!((
/// #       query.at(&Player(0)).iter().count(),
/// #       query.at(&Player(1)).iter().count(),
/// #       query.at(&Player(2)).iter().count(),
/// #       query.at(&Player(3)).iter().count(),
/// #       query.at(&Player(4)).iter().count(),
/// #       query.at(&Player(5)).iter().count(),
/// #    ), (1, 2, 3, 4, 5, 6));
/// }
/// # world.run_system_cached(find_all_player_one_entities);
/// ```
pub struct QueryByIndex<'world, C: IndexableComponent, D: QueryData, F: QueryFilter = ()> {
    world: UnsafeWorldCell<'world>,
    state: Option<QueryState<D, (F, With<C>)>>,
    last_run: Tick,
    this_run: Tick,
    index: Res<'world, Index<C>>,
}

impl<C: IndexableComponent, D: QueryData, F: QueryFilter> QueryByIndex<'_, C, D, F> {
    /// Return a [`Query`] only returning entities with a component `C` of the provided value.
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
    /// world.add_index::<FavoriteColor>();
    ///
    /// fn find_red_fans(mut query: QueryByIndex<FavoriteColor, Entity>) {
    ///     for entity in query.at(&FavoriteColor::Red).iter() {
    ///         println!("{entity:?} likes the color Red!");
    ///     }
    /// }
    /// ```
    pub fn at(&mut self, value: &C) -> Query<'_, '_, D, (F, With<C>)> {
        self.state = {
            // SAFETY: Mutable references do not alias and will be dropped after this block
            let mut builder = unsafe { QueryBuilder::new(self.world.world_mut()) };

            self.index.filter_query_for(&mut builder, value);

            Some(builder.build())
        };

        // SAFETY: We have registered all of the query's world accesses,
        // so the caller ensures that `world` has permission to access any
        // world data that the query needs.
        unsafe {
            Query::new(
                self.world,
                self.state.as_mut().unwrap(),
                self.last_run,
                self.this_run,
            )
        }
    }
}

// SAFETY: We rely on the known-safe implementations of `SystemParam` for `Res` and `Query`.
unsafe impl<C: IndexableComponent, D: QueryData + 'static, F: QueryFilter + 'static> SystemParam
    for QueryByIndex<'_, C, D, F>
{
    type State = (QueryState<D, (F, With<C>)>, ComponentId);
    type Item<'w, 's> = QueryByIndex<'w, C, D, F>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let query_state = <Query<D, (F, With<C>)> as SystemParam>::init_state(world, system_meta);
        let res_state = <Res<Index<C>> as SystemParam>::init_state(world, system_meta);

        (query_state, res_state)
    }

    unsafe fn new_archetype(
        (query_state, res_state): &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        <Query<D, (F, With<C>)> as SystemParam>::new_archetype(query_state, archetype, system_meta);
    }

    #[inline]
    unsafe fn validate_param(
        (query_state, res_state): &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        let query_valid = <Query<D, (F, With<C>)> as SystemParam>::validate_param(
            query_state,
            system_meta,
            world,
        );
        let res_valid =
            <Res<Index<C>> as SystemParam>::validate_param(res_state, system_meta, world);

        query_valid && res_valid
    }

    unsafe fn get_param<'world, 'state>(
        (query_state, res_state): &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        query_state.validate_world(world.id());

        let index =
            <Res<Index<C>> as SystemParam>::get_param(res_state, system_meta, world, change_tick);

        QueryByIndex {
            world,
            state: None,
            last_run: system_meta.last_run,
            this_run: change_tick,
            index,
        }
    }
}
