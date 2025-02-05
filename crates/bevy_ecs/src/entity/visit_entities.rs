pub use bevy_ecs_macros::{VisitEntities, VisitEntitiesMut};

use crate::entity::Entity;

/// Apply an operation to all entities in a container.
///
/// This is implemented by default for types that implement [`IntoIterator`].
///
/// It may be useful to implement directly for types that can't produce an
/// iterator for lifetime reasons, such as those involving internal mutexes.
pub trait VisitEntities {
    /// Apply an operation to all contained entities.
    fn visit_entities<F: FnMut(Entity)>(&self, f: F);
}

impl<T> VisitEntities for T
where
    for<'a> &'a T: IntoIterator<Item = &'a Entity>,
{
    fn visit_entities<F: FnMut(Entity)>(&self, f: F) {
        self.into_iter().copied().for_each(f);
    }
}

impl VisitEntities for Entity {
    fn visit_entities<F: FnMut(Entity)>(&self, mut f: F) {
        f(*self);
    }
}

/// Apply an operation to mutable references to all entities in a container.
///
/// This is implemented by default for types that implement [`IntoIterator`].
///
/// It may be useful to implement directly for types that can't produce an
/// iterator for lifetime reasons, such as those involving internal mutexes.
pub trait VisitEntitiesMut: VisitEntities {
    /// Apply an operation to mutable references to all contained entities.
    fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, f: F);
}

impl<T: VisitEntities> VisitEntitiesMut for T
where
    for<'a> &'a mut T: IntoIterator<Item = &'a mut Entity>,
{
    fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, f: F) {
        self.into_iter().for_each(f);
    }
}

impl VisitEntitiesMut for Entity {
    fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, mut f: F) {
        f(self);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::{hash_map::EntityHashMap, MapEntities, SceneEntityMapper},
        world::World,
    };
    use alloc::{string::String, vec, vec::Vec};
    use bevy_platform_support::collections::HashSet;

    use super::*;

    #[derive(VisitEntities, Debug, PartialEq)]
    struct Foo {
        ordered: Vec<Entity>,
        unordered: HashSet<Entity>,
        single: Entity,
        #[visit_entities(ignore)]
        not_an_entity: String,
    }

    // Need a manual impl since VisitEntitiesMut isn't implemented for `HashSet`.
    // We don't expect users to actually do this - it's only for test purposes
    // to prove out the automatic `MapEntities` impl we get with `VisitEntitiesMut`.
    impl VisitEntitiesMut for Foo {
        fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, mut f: F) {
            self.ordered.visit_entities_mut(&mut f);
            self.unordered = self
                .unordered
                .drain()
                .map(|mut entity| {
                    f(&mut entity);
                    entity
                })
                .collect();
            f(&mut self.single);
        }
    }

    #[test]
    fn visit_entities() {
        let mut world = World::new();
        let entities = world.entities();
        let mut foo = Foo {
            ordered: vec![entities.reserve_entity(), entities.reserve_entity()],
            unordered: [
                entities.reserve_entity(),
                entities.reserve_entity(),
                entities.reserve_entity(),
            ]
            .into_iter()
            .collect(),
            single: entities.reserve_entity(),
            not_an_entity: "Bar".into(),
        };

        let mut entity_map = EntityHashMap::<Entity>::default();
        let mut remapped = Foo {
            ordered: vec![],
            unordered: HashSet::default(),
            single: Entity::PLACEHOLDER,
            not_an_entity: foo.not_an_entity.clone(),
        };

        // Note: this assumes that the VisitEntities derive is field-ordered,
        //       which isn't explicitly stated/guaranteed.
        //       If that changes, this test will fail, but that might be OK if
        //       we're intentionally breaking that assumption.
        let mut i = 0;
        foo.visit_entities(|entity| {
            let new_entity = entities.reserve_entity();
            if i < foo.ordered.len() {
                assert_eq!(entity, foo.ordered[i]);
                remapped.ordered.push(new_entity);
            } else if i < foo.ordered.len() + foo.unordered.len() {
                assert!(foo.unordered.contains(&entity));
                remapped.unordered.insert(new_entity);
            } else {
                assert_eq!(entity, foo.single);
                remapped.single = new_entity;
            }

            entity_map.insert(entity, new_entity);

            i += 1;
        });

        SceneEntityMapper::world_scope(&mut entity_map, &mut world, |_, mapper| {
            foo.map_entities(mapper);
        });

        assert_eq!(foo, remapped);
    }
}
