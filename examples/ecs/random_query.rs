use bevy::prelude::*;
use rand::seq::IteratorRandom;

#[derive(Component, Clone, Copy)]
struct Pet(char);

#[derive(Component, Clone)]
struct Name(String);

fn main() {
    App::new()
        .add_startup_system(generate_names)
        .add_startup_system(generate_pets)
        .add_system(random_people)
        .run();
}

fn generate_names(mut commands: Commands) {
    let names = ["Linus", "Noah", "Elijah", "Olivia", "Ava", "Charlotte"];
    for name in names {
        commands.spawn().insert(Name(name.to_owned()));
    }
}

fn generate_pets(mut commands: Commands) {
    let pets = ['🐀', '🐄', '🐅', '🐇', '🐊', '🐓', '🐖'];
    for pet in pets {
        commands.spawn().insert(Pet(pet));
    }
}

fn random_people(name_query: Query<&Name>, pet_query: Query<&Pet>) {
    let mut rng = rand::thread_rng();
    // Use [`IteratorRandom::choose_multiple`] to pick three random names
    let names: Vec<&Name> = name_query.iter().choose_multiple(&mut rng, 3);
    for name in names {
        // Use [`IteratorRandom::choose`] to pick one random pet
        let pet: Option<&Pet> = pet_query.iter().choose(&mut rng);
        // `pet` will only be `None` if the query found no pets. That shouldn't happen here.
        let pet = pet.unwrap();
        println!("{} owns a {}.", name.0, pet.0);
    }
}
