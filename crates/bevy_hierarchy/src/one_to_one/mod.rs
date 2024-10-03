//! Provides access to the [`OneToOne`] component, allowing for one-to-one relationships
//! of an arbitrary type to be automatically managed in the ECS.

mod component;
pub use component::OneToOne;

mod event;
pub use event::OneToOneEvent;

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;

    use bevy_ecs::{event::Events, world::World};

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
        world.init_resource::<Events<OneToOneEvent<Friendship>>>();

        let a = world.spawn_empty().id();
        let b = world.spawn(Friend::new(a)).id();

        world.flush();

        assert_eq!(world.get::<Friend>(a), Some(&Friend::new(b)));
        assert_eq!(world.get::<Friend>(b), Some(&Friend::new(a)));

        assert_eq!(
            world
                .resource_mut::<Events<OneToOneEvent<Friendship>>>()
                .drain()
                .collect::<Vec<_>>(),
            vec![OneToOneEvent::<Friendship>::Added(b, a, PhantomData)]
        );

        world.entity_mut(a).remove::<Friend>();

        world.flush();

        assert_eq!(world.get::<Friend>(a), None);
        assert_eq!(world.get::<Friend>(b), None);

        assert_eq!(
            world
                .resource_mut::<Events<OneToOneEvent<Friendship>>>()
                .drain()
                .collect::<Vec<_>>(),
            vec![OneToOneEvent::<Friendship>::Removed(a, b, PhantomData)]
        );
    }
}
