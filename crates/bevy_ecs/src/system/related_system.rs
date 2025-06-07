use std::iter::Map;

use bevy_ecs::{
    component::Tick,
    entity::Entity,
    query::{QueryData, QueryFilter, QueryIter, QueryManyIter, QueryState, With},
    relationship::{Relationship, RelationshipTarget},
    system::{Query, SystemMeta, SystemParam},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

// SystemParam for combine 2 related queries
pub struct Related<'w, 's, D: QueryData, R: RelationshipTarget, F: QueryFilter> {
    data_query: Query<'w, 's, D, With<R>>,
    filter_query: QueryIter<'w, 's, &'static R::Relationship, F>,
}

impl<'w, 's, D: QueryData, R: RelationshipTarget, F: QueryFilter> Related<'w, 's, D, R, F> {
    /// Read iterator
    pub fn iter(
        &self,
    ) -> QueryManyIter<
        '_,
        '_,
        <D as QueryData>::ReadOnly,
        With<R>,
        Map<
            QueryIter<'w, 's, &'static R::Relationship, F>,
            impl FnMut(&'w R::Relationship) -> Entity,
        >,
    > {
        self.data_query
            .iter_many(self.filter_query.clone().map(|r| r.get()))
    }
    /// Mutate iterator
    pub fn iter_mut(
        &mut self,
    ) -> QueryManyIter<
        '_,
        '_,
        D,
        With<R>,
        Map<
            QueryIter<'w, 's, &'static R::Relationship, F>,
            impl FnMut(&'w R::Relationship) -> Entity,
        >,
    > {
        self.data_query
            .iter_many_mut(self.filter_query.clone().map(|r| r.get()))
    }
}

/// Just make 2 independent queries and then combine them.
unsafe impl<'w, 's, R: RelationshipTarget + 'static, D: QueryData + 'static, F: QueryFilter + 'static>
    SystemParam for Related<'w, 's, D, R, F>
{
    type State = (
        QueryState<D, With<R>>,
        QueryState<&'static R::Relationship, F>,
    );
    type Item<'world, 'state> = Related<'world, 'state, D, R, F>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
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
        let data_query = unsafe { state.0.query_unchecked_manual(world) };
        let filter_query = unsafe { state.1.query_unchecked_manual(world).into_iter() };
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

        let orc_id = world.spawn((Orc, children![(Head, Fangs)])).id();
        let human_id = world.spawn((Human, children![Head])).id();
        let wolf_id = world.spawn((Wolf, children![(Head, Fangs)])).id();

        let _ = world.run_system(with_head_sys);
        let _ = world.run_system(with_head_and_fangs_sys);
        let _ = world.run_system(with_head_and_without_fangs_sys);

        let wolf2_id = world.spawn((Wolf, children![(Head, Fangs)])).id();
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

    fn my_with_head(q: Related<Entity, Children, With<Head>>) -> usize {
        q.iter().count()
    }

    fn with_head(q: Query<Entity, With<Children>>, q2: Query<&ChildOf, With<Head>>) -> usize {
        q.iter().fold(0, |acc, e| {
            if q2.iter().map(|c| c.parent()).find(|c| *c == e).is_some() {
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
            if q2.iter().map(|c| c.parent()).find(|c| *c == e).is_some() {
                acc + 1
            } else {
                acc
            }
        })
    }

    fn my_with_head_and_fangs(q: Related<Entity, Children, (With<Head>, With<Fangs>)>) -> usize {
        q.iter().count()
    }

    fn with_head_and_without_fangs(
        q: Query<Entity, With<Children>>,
        q2: Query<&ChildOf, (With<Head>, Without<Fangs>)>,
    ) -> usize {
        q.iter().fold(0, |acc, e| {
            if q2.iter().map(|c| c.parent()).find(|c| *c == e).is_some() {
                acc + 1
            } else {
                acc
            }
        })
    }

    fn my_with_head_and_without_fangs(
        q: Related<Entity, Children, (With<Head>, Without<Fangs>)>,
    ) -> usize {
        q.iter().count()
    }
}
