//! This examples illustrates the different ways you can employ observers

use bevy::prelude::*;

#[derive(Component, Debug)]
struct CompA(Entity);

#[derive(Component, Debug)]
struct CompB;

#[derive(Component)]
struct Resize(u64, u64);

#[derive(Resource, Default)]
struct ResizeCount(usize);

fn main() {
    App::new().add_systems(Startup, setup).run();
}

fn setup(world: &mut World) {
    world.init_resource::<ResizeCount>();
    world.init_component::<Resize>();

    // Triggered when &CompA is added to an entity, runs any non-exclusive system
    let observer = world.observer(
        |observer: Observer<OnAdd, CompA>,
         mut commands: Commands,
         query: Query<&CompA, With<CompB>>| {
            // Get source entity that triggered the observer
            let source = observer.source();
            // Able to read component data via a query
            if let Ok(data) = query.get(source) {
                // Can submit commands for any structural changes
                commands.entity(source).remove::<CompB>();
                // Or to raise other events
                commands.event(Resize(2, 4)).entity(data.0).emit();
            }
        },
    );

    let entity = world
        // This will not trigger the observer as the entity does not have CompB
        .spawn(CompA(observer))
        // Respond to events targeting a specific entity
        .observe(
            |observer: Observer<Resize>, query: Query<&CompA>, mut res: ResMut<ResizeCount>| {
                // Since Resize carries data you can read/write that data from the observer
                let size = observer.data();
                // Simultaneously read components
                if let Ok(data) = query.get(observer.source()) {
                    println!(
                        "Received resize: {}, {} while data was: {:?}",
                        size.0, size.1, data
                    );
                    // Write to resources
                    res.0 += 1;
                }
            },
        )
        .id();

    world.flush_commands();

    assert_eq!(world.resource::<ResizeCount>().0, 0);

    // This will spawn an entity with CompA
    // - Which will trigger the first observer
    //   - Removing CompB
    //   - Emitting Resize targeting `entity`
    //      - Which will trigger it's entity observer
    //          - Incrementing ResizeCount
    let entity_b = world.spawn((CompA(entity), CompB)).flush();

    assert!(!world.entity(entity_b).contains::<CompB>());
    assert_eq!(world.resource::<ResizeCount>().0, 1);
}
