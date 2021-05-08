use bevy_ecs::prelude::*;
use rand::Rng;
use std::ops::Deref;

#[derive(Debug)]
struct EntityCounter {
    pub value: i32,
}

// In this example we will simulate a population of entities. In every tick we will:
// 1. spawn a new entity with a certain possibility
// 2. age all entities
// 3. despawn entities with age > 2
//
// To demonstrate change detection, there are some console outputs based on changes in the EntityCounter resource and updated Age components
fn main() {
    // Create a world
    let mut world = World::new();

    // Add the counter resource to remember how many entities where spawned
    world.insert_resource(EntityCounter { value: 0 });

    // Create a schedule and a stage
    let mut schedule = Schedule::default();
    let mut update = SystemStage::parallel();

    // Add systems sending and receiving events
    update.add_system(spawn_entities.system().label("spawn"));
    update.add_system(print_counter_when_changed.system().after("spawn"));
    update.add_system(age_all_entities.system().label("age"));
    update.add_system(remove_old_entities.system().after("age"));
    update.add_system(print_changed_entities.system().after("age"));
    schedule.add_stage("update", update);

    for iteration in 1..=10 {
        println!("Simulating frame {}/10", iteration);
        schedule.run_once(&mut world);
    }
}

#[derive(Default, Debug)]
struct Age {
    ticks: i32,
}

fn spawn_entities(mut commands: Commands, mut entity_counter: ResMut<EntityCounter>) {
    if rand::thread_rng().gen_bool(0.6) {
        let mut entity = commands.spawn();
        entity.insert(Age::default());
        println!("    spawning {:?}", entity.id());
        entity_counter.value += 1;
    }
}

fn remove_old_entities(mut commands: Commands, entities: Query<(Entity, &Age)>) {
    for (entity, age) in entities.iter() {
        if age.ticks > 2 {
            println!("    despawning {:?} due to age > 2", entity);
            commands.entity(entity).despawn();
        }
    }
}

fn age_all_entities(mut entities: Query<&mut Age>) {
    for mut age in entities.iter_mut() {
        age.ticks += 1;
    }
}

fn print_changed_entities(
    entity_with_added_component: Query<Entity, Added<Age>>,
    entity_with_mutated_component: Query<(Entity, &Age), Changed<Age>>,
) {
    for entity in entity_with_added_component.iter() {
        println!("    {:?} has it's first birthday!", entity);
    }
    for (entity, value) in entity_with_mutated_component.iter() {
        println!("    {:?} now has {:?}", entity, value);
    }
}

fn print_counter_when_changed(entity_counter: Res<EntityCounter>) {
    if entity_counter.is_changed() {
        println!(
            "    total number of entities spawned: {}",
            entity_counter.deref().value
        );
    }
}
