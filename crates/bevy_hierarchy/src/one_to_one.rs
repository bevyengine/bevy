#[cfg(feature = "reflect")]
use bevy_ecs::reflect::{
    ReflectComponent, ReflectFromWorld, ReflectMapEntities, ReflectVisitEntities,
    ReflectVisitEntitiesMut,
};
use bevy_ecs::{
    component::Component,
    entity::{Entity, VisitEntities, VisitEntitiesMut},
    traversal::Traversal,
    world::{FromWorld, World},
};
use core::{fmt::Debug, marker::PhantomData, ops::Deref};

use crate::relationship::Relationship;

/// Represents one half of a one-to-one relationship between two [entities](Entity).
///
/// The type of relationship is denoted by the parameter `R`.
#[derive(Component)]
#[component(
    on_insert = <Self as Relationship>::associate,
    on_replace = <Self as Relationship>::disassociate,
)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(
    feature = "reflect",
    reflect(
        Component,
        MapEntities,
        VisitEntities,
        VisitEntitiesMut,
        PartialEq,
        Debug,
        FromWorld
    )
)]
pub struct OneToOne<FK, PK = FK> {
    entity: Entity,
    #[cfg_attr(feature = "reflect", reflect(ignore))]
    _phantom: PhantomData<fn(&FK, &PK)>,
}

impl<FK: 'static, PK: 'static> Relationship for OneToOne<FK, PK> {
    type Other = OneToOne<PK, FK>;

    fn has(&self, entity: Entity) -> bool {
        self.entity == entity
    }

    fn new(entity: Entity) -> Self {
        Self {
            entity,
            _phantom: PhantomData,
        }
    }

    fn with(self, entity: Entity) -> Self {
        Self {
            entity,
            _phantom: PhantomData,
        }
    }

    fn without(self, entity: Entity) -> Option<Self> {
        (self.entity != entity).then_some(self)
    }

    fn iter(&self) -> impl ExactSizeIterator<Item = Entity> {
        [self.entity].into_iter()
    }
}

impl<FK, PK> PartialEq for OneToOne<FK, PK> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
    }
}

impl<FK, PK> Eq for OneToOne<FK, PK> {}

impl<FK, PK> Debug for OneToOne<FK, PK> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Has One {} ({}) With One ({})",
            self.entity,
            core::any::type_name::<FK>(),
            core::any::type_name::<PK>()
        )
    }
}

impl<FK, PK> VisitEntities for OneToOne<FK, PK> {
    fn visit_entities<F: FnMut(Entity)>(&self, mut f: F) {
        f(self.entity);
    }
}

impl<FK, PK> VisitEntitiesMut for OneToOne<FK, PK> {
    fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, mut f: F) {
        f(&mut self.entity);
    }
}

// TODO: We need to impl either FromWorld or Default so OneToOne<R> can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However OneToOne<R> should only ever be set with a real user-defined entity. It's worth looking into
// better ways to handle cases like this.
impl<FK, PK> FromWorld for OneToOne<FK, PK> {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            _phantom: PhantomData,
        }
    }
}

impl<FK, PK> Deref for OneToOne<FK, PK> {
    type Target = Entity;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

/// This provides generalized hierarchy traversal for use in [event propagation].
///
/// [event propagation]: bevy_ecs::observer::Trigger::propagate
impl<FK: 'static, PK: 'static> Traversal for &OneToOne<FK, PK> {
    fn traverse(item: Self::Item<'_>) -> Option<Entity> {
        Some(item.entity)
    }
}

impl<FK, PK> OneToOne<FK, PK> {
    /// Gets the [`Entity`] ID of the other member of this one-to-one relationship.
    #[inline(always)]
    pub fn get(&self) -> Entity {
        self.entity
    }

    /// Gets the other [`Entity`] as a slice of length 1.
    #[inline(always)]
    pub fn as_slice(&self) -> &[Entity] {
        core::slice::from_ref(&self.entity)
    }

    /// Create a new relationship with the provided [`Entity`].
    #[inline(always)]
    #[must_use]
    pub fn new(other: Entity) -> Self {
        Self {
            entity: other,
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{event::Events, world::World};

    use crate::RelationshipEvent;

    use super::*;

    /// An example relationship between two entities
    struct Friendship;

    /// Shorthand for a best friend relationship
    type Friend = OneToOne<Friendship>;

    #[test]
    fn simple_add_then_remove() {
        let mut world = World::new();

        world.register_component::<Friend>();

        let a = world.spawn_empty().id();
        let b = world.spawn(Friend::new(a)).id();

        world.flush();

        assert_eq!(world.get::<Friend>(a), Some(&Friend::new(b)));
        assert_eq!(world.get::<Friend>(b), Some(&Friend::new(a)));

        world.entity_mut(a).remove::<Friend>();

        world.flush();

        assert_eq!(world.get::<Friend>(a), None);
        assert_eq!(world.get::<Friend>(b), None);
    }

    #[test]
    fn triangular_break_up() {
        let mut world = World::new();

        world.register_component::<Friend>();

        let a = world.spawn_empty().id();
        let b = world.spawn(Friend::new(a)).id();
        let c = world.spawn_empty().id();

        world.flush();

        assert_eq!(world.get::<Friend>(a), Some(&Friend::new(b)));
        assert_eq!(world.get::<Friend>(b), Some(&Friend::new(a)));
        assert_eq!(world.get::<Friend>(c), None);

        world.entity_mut(a).insert(Friend::new(c));

        world.flush();

        assert_eq!(world.get::<Friend>(a), Some(&Friend::new(c)));
        assert_eq!(world.get::<Friend>(b), None);
        assert_eq!(world.get::<Friend>(c), Some(&Friend::new(a)));
    }

    #[test]
    fn repeated_adding() {
        let mut world = World::new();

        world.register_component::<Friend>();

        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        world.entity_mut(a).insert(Friend::new(b));
        world.entity_mut(a).insert(Friend::new(b));
        world.entity_mut(a).insert(Friend::new(b));

        world.flush();

        assert_eq!(world.get::<Friend>(a), Some(&Friend::new(b)));
        assert_eq!(world.get::<Friend>(b), Some(&Friend::new(a)));
    }

    #[test]
    fn swap() {
        let mut world = World::new();

        world.register_component::<Friend>();

        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();
        let d = world.spawn_empty().id();

        world.entity_mut(a).insert(Friend::new(b));
        world.entity_mut(c).insert(Friend::new(d));

        world.flush();

        assert_eq!(world.get::<Friend>(a), Some(&Friend::new(b)));
        assert_eq!(world.get::<Friend>(b), Some(&Friend::new(a)));
        assert_eq!(world.get::<Friend>(c), Some(&Friend::new(d)));
        assert_eq!(world.get::<Friend>(d), Some(&Friend::new(c)));

        world.entity_mut(a).insert(Friend::new(c));
        world.entity_mut(b).insert(Friend::new(d));

        world.flush();

        assert_eq!(world.get::<Friend>(a), Some(&Friend::new(c)));
        assert_eq!(world.get::<Friend>(b), Some(&Friend::new(d)));
        assert_eq!(world.get::<Friend>(c), Some(&Friend::new(a)));
        assert_eq!(world.get::<Friend>(d), Some(&Friend::new(b)));
    }

    #[test]
    fn looped_add_and_remove() {
        let mut world = World::new();

        world.register_component::<Friend>();

        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        for _ in 0..10_000 {
            world.entity_mut(a).insert(Friend::new(b));
            world.entity_mut(a).remove::<Friend>();
        }

        world.flush();

        assert_eq!(world.get::<Friend>(a), None);
        assert_eq!(world.get::<Friend>(b), None);
    }

    #[test]
    fn invalid_chaining() {
        let mut world = World::new();

        world.register_component::<Friend>();

        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();
        let d = world.spawn_empty().id();
        let e = world.spawn_empty().id();
        let f = world.spawn_empty().id();

        world.entity_mut(a).insert(Friend::new(b));
        world.entity_mut(b).insert(Friend::new(c));
        world.entity_mut(c).insert(Friend::new(d));
        world.entity_mut(d).insert(Friend::new(e));
        world.entity_mut(e).insert(Friend::new(f));
        world.entity_mut(f).insert(Friend::new(a));

        world.flush();

        assert_eq!(world.get::<Friend>(a), Some(&Friend::new(b)));
        assert_eq!(world.get::<Friend>(b), Some(&Friend::new(a)));
        assert_eq!(world.get::<Friend>(c), Some(&Friend::new(d)));
        assert_eq!(world.get::<Friend>(d), Some(&Friend::new(c)));
        assert_eq!(world.get::<Friend>(e), Some(&Friend::new(f)));
        assert_eq!(world.get::<Friend>(f), Some(&Friend::new(e)));

        // The pairing is caused by the first member of the pair (e.g., a, c, e) replacing
        // the relationship on the second member of the pair (e.g, b, d, f).
        // When the replacement occurs, it checks if the second member had a valid relationship
        // with it's old data (e.g., b -> c, d -> e, etc.) and if it did not, then no action is taken.
    }

    #[test]
    fn event_testing() {
        let mut world = World::new();

        world.register_component::<Friend>();
        world.init_resource::<Events<RelationshipEvent<Friend>>>();

        let a = world.spawn_empty().id();
        let b = world.spawn(Friend::new(a)).id();

        world.flush();

        assert_eq!(world.get::<Friend>(a), Some(&Friend::new(b)));
        assert_eq!(world.get::<Friend>(b), Some(&Friend::new(a)));

        assert_eq!(
            world
                .resource_mut::<Events<RelationshipEvent<Friend>>>()
                .drain()
                .collect::<Vec<_>>(),
            vec![RelationshipEvent::<Friend>::added(b, a)]
        );

        world.entity_mut(a).remove::<Friend>();

        world.flush();

        assert_eq!(world.get::<Friend>(a), None);
        assert_eq!(world.get::<Friend>(b), None);

        assert_eq!(
            world
                .resource_mut::<Events<RelationshipEvent<Friend>>>()
                .drain()
                .collect::<Vec<_>>(),
            vec![RelationshipEvent::<Friend>::removed(a, b)]
        );
    }
}
