pub use bevy_ecs_macros::VisitEntities;

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
        f(*self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as bevy_ecs, entity::Entities};
    use bevy_utils::HashSet;

    use super::*;

    #[derive(VisitEntities)]
    struct Foo {
        ordered: Vec<Entity>,
        unordered: HashSet<Entity>,
        single: Entity,
        #[allow(dead_code)]
        #[visit_entities(ignore)]
        not_an_entity: String,
    }

    #[test]
    fn visit_entities() {
        let entities = Entities::new();
        let foo = Foo {
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

        // Note: this assumes that the IterEntities derive is field-ordered,
        //       which isn't explicitly stated/guaranteed.
        //       If that changes, this test will fail, but that might be OK if
        //       we're intentionally breaking that assumption.
        let mut i = 0;
        foo.visit_entities(|entity| {
            if i < foo.ordered.len() {
                assert_eq!(entity, foo.ordered[i]);
            } else if i < foo.ordered.len() + foo.unordered.len() {
                assert!(foo.unordered.contains(&entity));
            } else {
                assert_eq!(entity, foo.single);
            }

            i += 1;
        });
    }
}
