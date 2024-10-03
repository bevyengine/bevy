mod event;
pub use event::OneToManyEvent;

mod one;
pub use one::OneToManyOne;

mod many;
pub use many::OneToManyMany;

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;

    use bevy_ecs::{event::Events, world::World};

    use super::*;

    /// A familial relationship
    struct Family;

    /// Shorthand for a Parent in a Family relationship
    type Parent = OneToManyOne<Family>;

    /// Shorthand for a Parent in a Family relationship
    type Children = OneToManyMany<Family>;

    #[test]
    fn simple_add_then_remove() {
        let mut world = World::new();

        world.register_component::<Parent>();
        world.register_component::<Children>();

        let a = world.spawn_empty().id();
        let b = world.spawn(Parent::new(a)).id();
        let c = world.spawn(Parent::new(a)).id();

        world.flush();

        assert_eq!(
            world
                .get::<Children>(a)
                .map(|c| c.iter().copied().collect::<Vec<_>>()),
            Some(vec![b, c])
        );
        assert_eq!(world.get::<Parent>(b), Some(&Parent::new(a)));
        assert_eq!(world.get::<Parent>(c), Some(&Parent::new(a)));

        world.entity_mut(a).remove::<Children>();

        world.flush();

        assert_eq!(world.get::<Children>(a), None);
        assert_eq!(world.get::<Parent>(b), None);
        assert_eq!(world.get::<Parent>(c), None);
    }

    #[test]
    fn partial_add_then_remove() {
        let mut world = World::new();

        world.register_component::<Parent>();
        world.register_component::<Children>();

        let a = world.spawn_empty().id();
        let b = world.spawn(Parent::new(a)).id();
        let c = world.spawn(Parent::new(a)).id();

        world.flush();

        assert_eq!(
            world
                .get::<Children>(a)
                .map(|c| c.iter().copied().collect::<Vec<_>>()),
            Some(vec![b, c])
        );
        assert_eq!(world.get::<Parent>(b), Some(&Parent::new(a)));
        assert_eq!(world.get::<Parent>(c), Some(&Parent::new(a)));

        world.entity_mut(c).remove::<Parent>();

        world.flush();

        assert_eq!(
            world
                .get::<Children>(a)
                .map(|c| c.iter().copied().collect::<Vec<_>>()),
            Some(vec![b])
        );
        assert_eq!(world.get::<Parent>(b), Some(&Parent::new(a)));
        assert_eq!(world.get::<Parent>(c), None);
    }

    #[test]
    fn take_and_return() {
        let mut world = World::new();

        world.register_component::<Parent>();
        world.register_component::<Children>();

        let a = world.spawn_empty().id();
        let b = world.spawn(Parent::new(a)).id();
        let c = world.spawn_empty().id();

        world.flush();

        assert_eq!(
            world
                .get::<Children>(a)
                .map(|c| c.iter().copied().collect::<Vec<_>>()),
            Some(vec![b])
        );
        assert_eq!(world.get::<Parent>(b), Some(&Parent::new(a)));
        assert_eq!(world.get::<Parent>(c), None);

        let component = world.entity_mut(a).take::<Children>().unwrap();

        let component = component.with(c);

        world.entity_mut(a).insert(component);

        world.flush();

        assert_eq!(
            world
                .get::<Children>(a)
                .map(|c| c.iter().copied().collect::<Vec<_>>()),
            Some(vec![b, c])
        );
        assert_eq!(world.get::<Parent>(b), Some(&Parent::new(a)));
        assert_eq!(world.get::<Parent>(c), Some(&Parent::new(a)));
    }

    #[test]
    fn event_testing() {
        let mut world = World::new();

        world.register_component::<Parent>();
        world.register_component::<Children>();
        world.init_resource::<Events<OneToManyEvent<Family>>>();

        let a = world.spawn_empty().id();
        let b = world.spawn(Parent::new(a)).id();

        world.flush();

        assert_eq!(
            world
                .get::<Children>(a)
                .map(|c| c.iter().copied().collect::<Vec<_>>()),
            Some(vec![b])
        );
        assert_eq!(world.get::<Parent>(b), Some(&Parent::new(a)));

        assert_eq!(
            world
                .resource_mut::<Events<OneToManyEvent<Family>>>()
                .drain()
                .collect::<Vec<_>>(),
            vec![OneToManyEvent::<Family>::Added(b, a, PhantomData)]
        );

        world.entity_mut(b).remove::<Parent>();

        world.flush();

        assert_eq!(world.get::<Children>(a), None);
        assert_eq!(world.get::<Parent>(b), None);

        assert_eq!(
            world
                .resource_mut::<Events<OneToManyEvent<Family>>>()
                .drain()
                .collect::<Vec<_>>(),
            vec![OneToManyEvent::<Family>::Removed(b, a, PhantomData)]
        );
    }
}
