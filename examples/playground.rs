
#![allow(missing_docs)]

use bevy::prelude::*;

// TODO GRACE: remove this file

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, on_space_press)
        .add_observer(on_a_mutated)
        .run();
}

#[derive(Component)]
struct A(i32);

fn setup(mut commands: Commands) {
    commands.spawn(A(0));
}

fn on_space_press(input: Res<ButtonInput<KeyCode>>, mut query: Query<&mut A>) {
    if input.just_pressed(KeyCode::Space) {
        for mut a in query.iter_mut() {
            println!("asdfjkaskldfjlksadfljsdfk");
            a.0 = 100;
        }
    }
}

fn on_a_mutated(trigger: Trigger<OnMutate, A>) {
    println!("A mutation happened!");
}


// fn main() {
//     App::new()
//         .add_plugins(DefaultPlugins)
//         .add_systems(Startup, setup)
//         .add_observer(on_a_added)
//         .run();
// }

// #[derive(Component)]
// struct A(i32);

// fn setup(mut commands: Commands) {
//     commands.spawn_empty().insert(A(0)).insert(A(1));
// }

// fn on_a_added(trigger: Trigger<OnAdd, A>) {
//     println!("Abafjgklasdfgjp;kl")
// }