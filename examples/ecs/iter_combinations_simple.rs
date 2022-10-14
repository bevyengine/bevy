//! A simple example showing how iter_combinations works

use bevy::prelude::*;

fn main() {
    App::new()
        .add_startup_system(add_entities)
        .add_system(example)
        .run();
}

#[derive(Component)]
struct A(usize);

fn add_entities(mut commands: Commands) {
    commands.spawn(A(1));
    commands.spawn(A(2));
    commands.spawn(A(3));
    commands.spawn(A(4));
}

// Below is a simple example of iter_combinations.
// The iter_combinations method produces the set of combinations of
// components without repeats.

// The iter combintions method is useful when there are a set of entities with components that all need to interact with
// eachother in some way. Since iter_combinations does not repeat entities, you can call it without worrying about entities
// interacting with themselves.
fn example(query: Query<&A>) {
    // The below loop will print out...
    // [1, 2], [1, 3], [1, 4], [2, 3], [2, 4], [3, 4]

    // Notice how it does not print out [1, 1], or [2, 2], because the function does not allow repeats.
    // The function also does not repeat [2, 1], because order in a combination does not matter.
    // So [2, 1] is equivalent to [1, 2].
    // The set of items where order does matter ([1, 2] != [2, 1]) would be called a permutation.
    let mut total = 0;
    for [a_one, a_two] in query.iter_combinations() {
        total += a_one.0 + a_two.0;
    }

    assert_eq!(total, 30);

    // The below loop will print out...
    // [1, 2, 3], [1, 2, 4], [1, 3, 4], [2, 3, 4]

    // The size of the items array output by each iteration of the loop is parameterized by K (in this case 3).
    // The number of total items output (in this case 4), is defined as N.
    // An "item" refers to each combination of numbers, not each number inside the array.
    // See the iter_combinations documentation for more.
    let mut total_two = 0;
    for [a_one, a_two, a_three] in query.iter_combinations::<3>() {
        total_two += a_one.0 + a_two.0 + a_three.0;
    }

    assert_eq!(total_two, 30);
}
