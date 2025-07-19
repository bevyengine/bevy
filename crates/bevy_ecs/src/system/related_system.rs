use core::iter::Map;

use bevy_ecs::{
    component::Tick,
    entity::Entity,
    query::{QueryData, QueryFilter, QueryIter, QueryManyIter, QueryState, With},
    relationship::{Relationship, RelationshipTarget},
    system::{Query, SystemMeta, SystemParam},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use derive_more::derive::Display;

use crate::query::QueryEntityError;

/// An error that occurs when retrieving a specific Entity’s query result from [`Related`].
#[derive(Debug, Display, Copy, Clone, PartialEq, Eq)]
pub enum RelatedQueryEntityError {
    /// Error retrieving data by source relationship entity.
    RelationshipEntityError(QueryEntityError),
    /// [`Entity`] does not have a relationship target that satisfies `F2`.
    RelationshipTargetEntityError(Entity),
}

impl core::error::Error for RelatedQueryEntityError {}

/// A Query like [system parameter] that provides selective access to the [`Component`] of  data stored in a [`World`],
/// where source of relationship match filter.
///
/// [system parameter]: crate::system::SystemParam
/// [`Component`]: crate::component::Component
///
/// `Related` is a generic data structure that accepts four type parameters:
///
/// - **`D` (query data)**:
///   The type of data fetched by the query, which will be returned as the query item.
///   Only entities that match the requested data will generate an item.
///   Must implement the [`QueryData`] trait.
/// - **`F1` (query filter)**:
///   The set of conditions that determine whether query items should be kept or discarded from
///   entities with source of relationship query.
///   Must implement the [`QueryFilter`] trait.
/// - **`R` (relationship target)
///   The target of the relationship, in relation to which the filtering by `F2` will be performed.
///   If the related entity contains as the target of the relationship an entity satisfying `F2`,
///   then it will correspond to the query.
///   Must implement the [`RelationshipTarget`] trait.
/// - **`F2` (query filter)
///   The set of conditions that determine whether query items should be kept or discarded
///   for relationship target query.
///   Must implement the [`QueryFilter`] trait.
pub struct Related<'w, 's, D: QueryData, F1: QueryFilter, R: RelationshipTarget, F2: QueryFilter> {
    data_query: Query<'w, 's, D, (F1, With<R>)>,
    filter_query: Query<'w, 's, &'static R::Relationship, F2>,
}

impl<'w, 's, D: QueryData, F1: QueryFilter, R: RelationshipTarget, F2: QueryFilter>
    Related<'w, 's, D, F1, R, F2>
{
    /// Returns an [`Iterator`] over the read-only items.
    pub fn iter(
        &'w self,
    ) -> QueryManyIter<
        'w,
        's,
        <D as QueryData>::ReadOnly,
        (F1, With<R>),
        Map<
            QueryIter<'w, 's, &'static R::Relationship, F2>,
            impl FnMut(&'w R::Relationship) -> Entity,
        >,
    > {
        self.data_query
            .iter_many(self.filter_query.iter().map(Relationship::get))
    }
    /// Returns an [`Iterator`] over items for mutation.
    pub fn iter_mut(
        &'w mut self,
    ) -> QueryManyIter<
        'w,
        's,
        D,
        (F1, With<R>),
        Map<
            QueryIter<'w, 's, &'static R::Relationship, F2>,
            impl FnMut(&'w R::Relationship) -> Entity,
        >,
    > {
        self.data_query
            .iter_many_mut(self.filter_query.iter().map(Relationship::get))
    }

    /// Returns the read-only item for the given [`Entity`].
    pub fn get(
        &'w self,
        entity: Entity,
    ) -> Result<<<D as QueryData>::ReadOnly as QueryData>::Item<'w>, RelatedQueryEntityError> {
        if self.contains(entity) {
            match self.data_query.get(entity) {
                Ok(item) => Ok(item),
                Err(err) => Err(RelatedQueryEntityError::RelationshipEntityError(err)),
            }
        } else {
            Err(RelatedQueryEntityError::RelationshipTargetEntityError(
                entity,
            ))
        }
    }

    /// Returns `true` if the given [`Entity`] matches the relative query.
    pub fn contains(&self, entity: Entity) -> bool {
        return self
            .filter_query
            .iter()
            .map(Relationship::get)
            .any(|e| e == entity)
            && self.data_query.contains(entity);
    }

    /// Returns the mutating item for the given [`Entity`].
    pub fn get_mut(
        &'w mut self,
        entity: Entity,
    ) -> Result<<D as QueryData>::Item<'w>, RelatedQueryEntityError> {
        if self.contains(entity) {
            match self.data_query.get_mut(entity) {
                Ok(item) => Ok(item),
                Err(err) => Err(RelatedQueryEntityError::RelationshipEntityError(err)),
            }
        } else {
            Err(RelatedQueryEntityError::RelationshipTargetEntityError(
                entity,
            ))
        }
    }
}

/// Just make 2 independent queries and then combine them.
/// SAFETY: delegates safety to [`Query`] for `ComponentId` and `ArchetypeComponentId` access.
unsafe impl<'w, 's, R, D, F1, F2> SystemParam for Related<'w, 's, D, F1, R, F2>
where
    R: RelationshipTarget,
    D: QueryData + 'static,
    F1: QueryFilter + 'static,
    F2: QueryFilter + 'static,
{
    type State = (
        QueryState<D, (F1, With<R>)>,
        QueryState<&'static R::Relationship, F2>,
    );
    type Item<'world, 'state> = Related<'world, 'state, D, F1, R, F2>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        // Register all of query's world accesses
        let data_query = Query::init_state(world, system_meta);
        let filter_query = Query::init_state(world, system_meta);
        (data_query, filter_query)
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        _: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        _: Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY: We have registered all of the query's world accesses,
        // so the caller ensures that `world` has permission to access any
        // world data that the query needs.
        // The caller ensures the world matches the one used in init_state.
        let data_query = unsafe { state.0.query_unchecked_manual(world) };
        // SAFETY: We have registered all of the query's world accesses,
        // so the caller ensures that `world` has permission to access any
        // world data that the query needs.
        // The caller ensures the world matches the one used in init_state.
        let filter_query = unsafe { state.1.query_unchecked_manual(world) };
        Related {
            data_query,
            filter_query,
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        children,
        component::Component,
        entity::Entity,
        hierarchy::{ChildOf, Children},
        query::{With, Without},
        spawn::SpawnRelated,
        system::Query,
        world::World,
    };

    use super::Related;

    #[derive(Component)]
    struct Orc;
    #[derive(Component)]
    struct Human;
    #[derive(Component)]
    struct Wolf;
    #[derive(Component)]
    struct Fangs;
    #[derive(Component)]
    struct Head;

    #[test]
    fn world_test() {
        let mut world = World::new();

        let with_head_sys = world.register_system(with_head);
        let with_head_and_fangs_sys = world.register_system(with_head_and_fangs);
        let with_head_and_without_fangs_sys = world.register_system(with_head_and_without_fangs);
        let test_whs = world.register_system(my_with_head);
        let test_whafs = world.register_system(my_with_head_and_fangs);
        let test_whawf = world.register_system(my_with_head_and_without_fangs);

        let _orc_id = world.spawn((Orc, children![(Head, Fangs)])).id();
        let _human_id = world.spawn((Human, children![Head])).id();
        let _wolf_id = world.spawn((Wolf, children![(Head, Fangs)])).id();

        assert_eq!(
            world.run_system(with_head_sys).unwrap(),
            world.run_system(test_whs).unwrap()
        );
        assert_eq!(
            world.run_system(with_head_and_fangs_sys).unwrap(),
            world.run_system(test_whafs).unwrap()
        );
        assert_eq!(
            world.run_system(with_head_and_without_fangs_sys).unwrap(),
            world.run_system(test_whawf).unwrap()
        );

        let _wolf2_id = world.spawn((Wolf, children![(Head, Fangs)])).id();

        assert_eq!(
            world.run_system(with_head_sys).unwrap(),
            world.run_system(test_whs).unwrap()
        );
        assert_eq!(
            world.run_system(with_head_and_fangs_sys).unwrap(),
            world.run_system(test_whafs).unwrap()
        );
        assert_eq!(
            world.run_system(with_head_and_without_fangs_sys).unwrap(),
            world.run_system(test_whawf).unwrap()
        );
    }

    fn my_with_head(q: Related<Entity, (), Children, With<Head>>) -> usize {
        q.iter().count()
    }

    fn with_head(q: Query<Entity, With<Children>>, q2: Query<&ChildOf, With<Head>>) -> usize {
        q.iter().fold(0, |acc, e| {
            if q2.iter().map(ChildOf::parent).any(|c| c == e) {
                acc + 1
            } else {
                acc
            }
        })
    }

    fn with_head_and_fangs(
        q: Query<Entity, With<Children>>,
        q2: Query<&ChildOf, (With<Head>, With<Fangs>)>,
    ) -> usize {
        q.iter().fold(0, |acc, e| {
            if q2.iter().map(ChildOf::parent).any(|c| c == e) {
                acc + 1
            } else {
                acc
            }
        })
    }

    fn my_with_head_and_fangs(
        q: Related<Entity, (), Children, (With<Head>, With<Fangs>)>,
    ) -> usize {
        q.iter().count()
    }

    fn with_head_and_without_fangs(
        q: Query<Entity, With<Children>>,
        q2: Query<&ChildOf, (With<Head>, Without<Fangs>)>,
    ) -> usize {
        q.iter().fold(0, |acc, e| {
            if q2.iter().map(ChildOf::parent).any(|c| c == e) {
                acc + 1
            } else {
                acc
            }
        })
    }

    fn my_with_head_and_without_fangs(
        q: Related<Entity, (), Children, (With<Head>, Without<Fangs>)>,
    ) -> usize {
        q.iter().count()
    }
}
