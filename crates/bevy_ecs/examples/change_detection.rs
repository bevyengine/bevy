use bevy_ecs::prelude::*;
use std::ops::Deref;

#[derive(Debug)]
struct Counter {
    pub value: i32,
}

fn main() {
    // Create a world
    let mut world = World::new();

    // Add the counter resource
    world.insert_resource(Counter { value: 0 });

    // Create a schedule and a stage
    let mut schedule = Schedule::default();
    let mut update = SystemStage::parallel();

    // Add systems sending and receiving events
    update.add_system(increase_counter.system().label("increase"));
    update.add_system(print_counter_when_changed.system().after("increase"));
    update.add_system(manipulate_entities.system().label("mutate"));
    update.add_system(print_changed_entities.system().after("mutate"));
    schedule.add_stage("update", update);

    for iteration in 1..=10 {
        println!("Simulating frame {}/10", iteration);
        schedule.run_once(&mut world);
    }
}

fn manipulate_entities(
    mut commands: Commands,
    last_spawned_entity: Query<Entity, Without<i32>>,
    mut entities: Query<&mut i32>,
) {
    if rand::random::<f32>() > 0.5 {
        commands.spawn();
    }
    for mut value in entities.iter_mut() {
        *value += 1;
    }
    for entity in last_spawned_entity.iter() {
        commands.entity(entity).insert(0);
    }
}

fn print_changed_entities(
    entity_with_added_component: Query<Entity, Added<i32>>,
    entity_with_mutated_component: Query<&i32, Changed<i32>>,
) {
    for entity in entity_with_added_component.iter() {
        println!("    i32 component was just added to '{:?}'", entity);
    }
    for value in entity_with_mutated_component.iter() {
        println!("    component was mutated to {:?}", value);
    }
}

fn increase_counter(mut counter: ResMut<Counter>) {
    let random_value: f32 = rand::random();
    if random_value > 0.5 {
        counter.value += 1;
        println!("    Increased counter value");
    }
}

fn print_counter_when_changed(counter: Res<Counter>) {
    if counter.is_changed() && !counter.is_added() {
        println!("    Changed to {:?}", counter.deref());
    }
}
