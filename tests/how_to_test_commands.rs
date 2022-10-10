use bevy::{diagnostic::Execute, ecs::system::EntityCommands, prelude::*};

#[derive(Component, Default)]
struct Position(Vec2);

#[derive(Component)]
struct Destination(Vec2);

trait NavigateTo {
    fn navigate_to(self, target: Entity);
}

impl NavigateTo for &mut EntityCommands<'_, '_, '_> {
    /// Start navigation for this [`Entity`] towards [`Position`] of `target`.
    fn navigate_to(self, target: Entity) {
        let entity = self.id();
        self.commands().add(move |world: &mut World| {
            // Get position from target
            let &Position(xy) = world
                .get::<Position>(target)
                .expect("target must have a position");
            // Set navigation destination
            world.entity_mut(entity).insert(Destination(xy));
        });
    }
}

/* ... */

#[test]
fn did_set_destination() {
    const TARGET_POS: Vec2 = Vec2::new(5.0, 2.0);

    let mut world = World::default();

    // Spawn entity
    let entity = world.spawn_empty().id();

    // Spawn target at `TARGET_POS`
    let target = world.spawn(Position(TARGET_POS)).id();

    world.execute(|_world, mut commands| {
        commands.entity(entity).navigate_to(target);
    });

    // Ensure destination is set correctly to position of target
    let &Destination(destination) = world
        .get::<Destination>(entity)
        .expect("entity must have a destination");
    assert_eq!(destination, TARGET_POS);
}
