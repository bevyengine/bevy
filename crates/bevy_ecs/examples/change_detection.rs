use bevy_ecs::prelude::*;
use rand::Rng;
use std::ops::Deref;

// In this example we will simulate a population of entities. In every tick we will:
// 1. spawn a new entity with a certain possibility
// 2. age all entities
// 3. despawn entities with age > 2
//
// To demonstrate change detection, there are some console outputs based on changes in
// the EntityCounter resource and updated Age components
fn main() {
    // Create a new empty World to hold our Entities, Components and Resources
    let mut world = World::new();

    // Add the counter resource to remember how many entities where spawned
    world.insert_resource(EntityCounter { value: 0 });

    // Create a new Schedule, which stores systems and controls their relative ordering
    let mut schedule = Schedule::default();

    // Add systems to the Schedule to execute our app logic
    // We can label our systems to force a specific run-order between some of them
    schedule.add_systems((
        spawn_entities.in_set(SimulationSet::Spawn),
        print_counter_when_changed.after(SimulationSet::Spawn),
        age_all_entities.in_set(SimulationSet::Age),
        remove_old_entities.after(SimulationSet::Age),
        print_changed_entities.after(SimulationSet::Age),
    ));

    // Simulate 10 frames in our world
    for iteration in 1..=10 {
        println!("Simulating frame {iteration}/10");
        schedule.run(&mut world);
    }
}

// This struct will be used as a Resource keeping track of the total amount of spawned entities
#[derive(Debug, Resource)]
struct EntityCounter {
    pub value: i32,
}

// This struct represents a Component and holds the age in frames of the entity it gets assigned to
#[derive(Component, Default, Debug)]
struct Age {
    frames: i32,
}

// System sets can be used to group systems and configured to control relative ordering
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum SimulationSet {
    Spawn,
    Age,
}

// This system randomly spawns a new entity in 60% of all frames
// The entity will start with an age of 0 frames
// If an entity gets spawned, we increase the counter in the EntityCounter resource
fn spawn_entities(mut commands: Commands, mut entity_counter: ResMut<EntityCounter>) {
    if rand::thread_rng().gen_bool(0.6) {
        let entity_id = commands.spawn(Age::default()).id();
        println!("    spawning {entity_id:?}");
        entity_counter.value += 1;
    }
}

// This system prints out changes in our entity collection
// For every entity that just got the Age component added we will print that it's the
// entities first birthday. These entities where spawned in the previous frame.
// For every entity with a changed Age component we will print the new value.
// In this example the Age component is changed in every frame, so we don't actually
// need the `Changed` here, but it is still used for the purpose of demonstration.
fn print_changed_entities(
    entity_with_added_component: Query<Entity, Added<Age>>,
    entity_with_mutated_component: Query<(Entity, &Age), Changed<Age>>,
) {
    for entity in &entity_with_added_component {
        println!("    {entity:?} has it's first birthday!");
    }
    for (entity, value) in &entity_with_mutated_component {
        println!("    {entity:?} is now {value:?} frames old");
    }
}

// This system iterates over all entities and increases their age in every frame
fn age_all_entities(mut entities: Query<&mut Age>) {
    for mut age in &mut entities {
        age.frames += 1;
    }
}

// This system iterates over all entities in every frame and despawns entities older than 2 frames
fn remove_old_entities(mut commands: Commands, entities: Query<(Entity, &Age)>) {
    for (entity, age) in &entities {
        if age.frames > 2 {
            println!("    despawning {entity:?} due to age > 2");
            commands.entity(entity).despawn();
        }
    }
}

// This system will print the new counter value every time it was changed since
// the last execution of the system.
fn print_counter_when_changed(entity_counter: Res<EntityCounter>) {
    if entity_counter.is_changed() {
        println!(
            "    total number of entities spawned: {}",
            entity_counter.deref().value
        );
    }
}
