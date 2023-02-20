use bevy_ecs::prelude::*;

fn empty_system() {}

#[derive(Component)]
struct Counter(usize);

#[derive(Component)]
struct HitPoint(usize);

/// World diagnostic example.
/// can be called directly on World or run as a system.
fn diagnostic_world(world: &World) {
    let bundle_size = world.bundles().len();
    let component_size = world.components().len();
    let archetype_size = world.archetypes().len();
    let entity_size = world.entities().len();

    println!("***************");
    println!("World shape:");
    println!("  component: {bundle_size}");
    println!("  bundle: {component_size}");
    println!("  archetype: {archetype_size}");
    println!("  entity: {entity_size}");

    println!("World detail:");
    let bundles = world.bundles().iter().collect::<Vec<_>>();
    if bundle_size > 0 {
        println!("  bundles:");
        for bundle in bundles.iter() {
            println!("    {:?}: {:?}", bundle.id(), bundle.components());
        }
    }

    if archetype_size > 0 {
        println!("  archetypes:");
        for archetype in world.archetypes().iter() {
            println!(
                "    {:?}: components:{:?} entity count: {}",
                archetype.id(),
                archetype.components().collect::<Vec<_>>(),
                archetype.len()
            );
        }
    }
}

// In this example we add a counter resource and increase it's value in one system,
// while a different system prints the current count to the console.
fn main() {
    let mut world = World::new();
    let mut schedule = Schedule::default();
    schedule.add_system(empty_system);
    schedule.add_system(diagnostic_world);

    schedule.run(&mut world);

    let player = world.spawn(HitPoint(100)).id();
    schedule.run(&mut world);

    world.entity_mut(player).insert((Counter(0),));
    schedule.run(&mut world);

    world.entity_mut(player).despawn();
    schedule.run(&mut world);
}
