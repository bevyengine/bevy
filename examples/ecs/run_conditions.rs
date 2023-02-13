//! This example demonstrates how to use run criterias to control when systems run.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<InputCounter>()
        .add_system(
            increment_input_counter
                .run_if(resource_exists::<InputCounter>())
                .run_if(is_input),
        )
        .add_system(
            print_input_counter
                .run_if(resource_exists::<InputCounter>())
                .run_if(|c: Res<InputCounter>| c.is_changed() && !c.is_added()),
        )
        .run();
}

#[derive(Resource, Default)]
struct InputCounter(usize);

// Return true if the user has clicked, tapped or pressed the space bar
fn is_input(
    keyboard_input: Res<Input<KeyCode>>,
    mouse_button_input: Res<Input<MouseButton>>,
    touch_input: Res<Touches>,
) -> bool {
    keyboard_input.just_pressed(KeyCode::Space)
        || mouse_button_input.just_pressed(MouseButton::Left)
        || touch_input.any_just_pressed()
}

fn increment_input_counter(mut counter: ResMut<InputCounter>) {
    counter.0 += 1;
}

fn print_input_counter(counter: Res<InputCounter>) {
    println!("Input counter: {}", counter.0);
}
