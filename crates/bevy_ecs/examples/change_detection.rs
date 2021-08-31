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

    // Create a new Schedule, which defines an execution strategy for Systems
    let mut schedule = Schedule::default();
    // Create a Stage to add to our Schedule. Each Stage in a schedule runs all of its systems
    // before moving on to the next Stage
    let mut update = SystemStage::parallel();

    // Add systems to the Stage to execute our app logic
    // We can label our systems to force a specific run-order between some of them
    update.add_system(spawn_entities.label(SimulationSystem::Spawn));
    update.add_system(print_counter_when_changed.after(SimulationSystem::Spawn));
    update.add_system(age_all_entities.label(SimulationSystem::Age));
    update.add_system(remove_old_entities.after(SimulationSystem::Age));
    update.add_system(print_changed_entities.after(SimulationSystem::Age));
    // Add the Stage with our systems to the Schedule
    schedule.add_stage("update", update);

    // Simulate 10 frames in our world
    for iteration in 1..=10 {
        println!("Simulating frame {}/10", iteration);
        schedule.run(&mut world);
    }
}

// This struct will be used as a Resource keeping track of the total amount of spawned entities
#[derive(Debug)]
struct EntityCounter {
    pub value: i32,
}

// This struct represents a Component and holds the age in frames of the entity it gets assigned to
#[derive(Default, Debug)]
struct Age {
    frames: i32,
}

// System labels to enforce a run order of our systems
#[derive(SystemLabel, Debug, Clone, PartialEq, Eq, Hash)]
enum SimulationSystem {
    Spawn,
    Age,
}

// This system randomly spawns a new entity in 60% of all frames
// The entity will start with an age of 0 frames
// If an entity gets spawned, we increase the counter in the EntityCounter resource
fn spawn_entities(mut commands: Commands, mut entity_counter: ResMut<EntityCounter>) {
    if rand::thread_rng().gen_bool(0.6) {
        let entity_id = commands.spawn().insert(Age::default()).id();
        println!("    spawning {:?}", entity_id);
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
    for entity in entity_with_added_component.iter() {
        println!("    {:?} has it's first birthday!", entity);
    }
    for (entity, value) in entity_with_mutated_component.iter() {
        println!("    {:?} is now {:?} frames old", entity, value);
    }
}

// This system iterates over all entities and increases their age in every frame
fn age_all_entities(mut entities: Query<&mut Age>) {
    for mut age in entities.iter_mut() {
        age.frames += 1;
    }
}

// This system iterates over all entities in every frame and despawns entities older than 2 frames
fn remove_old_entities(mut commands: Commands, entities: Query<(Entity, &Age)>) {
    for (entity, age) in entities.iter() {
        if age.frames > 2 {
            println!("    despawning {:?} due to age > 2", entity);
            commands.entity(entity).despawn();
        }
    }
}

// This system will print the new counter value everytime it was changed since
// the last execution of the system.
fn print_counter_when_changed(entity_counter: Res<EntityCounter>) {
    if entity_counter.is_changed() {
        println!(
            "    total number of entities spawned: {}",
            entity_counter.deref().value
        );
    }
}
