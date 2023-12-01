//! This examples illustrates the different ways you can employ observers

use bevy::prelude::*;

#[derive(Component, Debug)]
struct CompA(Entity);

#[derive(Component, Debug)]
struct CompB;

#[derive(Component, Debug)]
struct Resize(u64, u64);

#[derive(Resource, Default)]
struct ResizeCount(usize);

fn main() {
    App::new().add_systems(Startup, setup).run();
}

fn setup(world: &mut World) {
    world.init_resource::<ResizeCount>();

    // Triggered when &ComponentA is added to any component that also has ComponentB
    // This can take any query types that implement WorldQueryData and WorldQueryFilter
    let observer = world.observer(|mut observer: Observer<OnAdd, &CompA, With<CompB>>| {
        // Get source entity that triggered the observer
        let source = observer.source();
        // Able to read requested component data as if it was a query
        let data = observer.fetch().0;
        // Access to all resources and components through DeferredWorld
        let world = observer.world_mut();
        // Can submit commands for any structural changes
        world.commands().entity(source).remove::<CompB>();
        // Or to raise other events
        world.commands().ecs_event(Resize(2, 4)).target(data).emit();
    });

    let entity = world
        // This will not trigger the observer as the entity does not have CompB
        .spawn(CompA(observer))
        // Respond to events targeting a specific entity
        // Still must match the query in order to trigger
        .observe(|mut observer: Observer<Resize, &CompA>| {
            // Since Resize carries data you can read/write that data from the observer
            let size = observer.data();
            // Simultaneously read components
            let data = observer.fetch();
            println!("Received resize: {:?} while data was: {:?}", size, data);
            // Write to resources
            observer.world_mut().resource_mut::<ResizeCount>().0 += 1;
        })
        .id();

    world.flush_commands();

    assert_eq!(world.resource::<ResizeCount>().0, 0);

    // This will spawn an entity with CompA
    // - Which will trigger the first observer
    //   - Removing CompB
    //   - Emitting Resize targetting `entity`
    //      - Which will trigger it's entity observer
    //          - Incrementing ResizeCount
    let entity_b = world.spawn((CompA(entity), CompB)).flush();

    assert!(!world.entity(entity_b).contains::<CompB>());
    assert_eq!(world.resource::<ResizeCount>().0, 1);
}
