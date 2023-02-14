//! This example demonstrates how to use run conditions to control when systems run.

use bevy::prelude::*;

fn main() {
    println!();
    println!("For the first 2 seconds you will not be able to increment the counter");
    println!("Once that time has passed you can press space, enter, left mouse, right mouse or touch the screen to increment the counter");
    println!();

    App::new()
        .add_plugins(DefaultPlugins.set(bevy::render::RenderPlugin {
            wgpu_settings: bevy::render::settings::WgpuSettings {
                backends: Some(bevy::render::settings::Backends::PRIMARY),
                ..Default::default()
            },
        }))
        .add_system(
            increment_input_counter
                // The common_conditions module has a few useful run conditions
                // for checking resources and states, these are included in prelude
                .run_if(resource_exists::<InputCounter>())
                // This is our custom run condition, both this and the
                // above condition must be true for the system to run
                .run_if(has_user_input),
        )
        .add_system(
            print_input_counter
                // This is also a custom run condition but this time in the form of a closure,
                // this is useful for small, simple run conditions you don't need to reuse.
                // All the normal rules still apply, all parameters must be read only except for local parameters
                // In this case we will only run if the input counter resource exists and has changed but not just been added
                .run_if(|res: Option<Res<InputCounter>>| {
                    if let Some(counter) = res {
                        counter.is_changed() && !counter.is_added()
                    } else {
                        false
                    }
                }),
        )
        .add_system(
            add_input_counter
                // This is a custom generator function that returns a run
                // condition, must like the common conditions module.
                // It will only return true once 2 seconds has passed
                .run_if(time_passed(2.0)),
        )
        .run();
}

#[derive(Resource, Default)]
struct InputCounter(usize);

/// Return true if any of the defined inputs were just pressed
/// This is a custom run condition, it can take any normal system parameters as long as
/// they are read only except for local parameters which can be mutable
/// It returns a bool which determines if the system should run
fn has_user_input(
    keyboard_input: Res<Input<KeyCode>>,
    mouse_button_input: Res<Input<MouseButton>>,
    touch_input: Res<Touches>,
) -> bool {
    keyboard_input.just_pressed(KeyCode::Space)
        || keyboard_input.just_pressed(KeyCode::Return)
        || mouse_button_input.just_pressed(MouseButton::Left)
        || mouse_button_input.just_pressed(MouseButton::Right)
        || touch_input.any_just_pressed()
}

/// This is a generator fuction that returns a closure which meets all the run condition rules
/// This is useful becuase you can reuse the same run condition but with different variables
/// This is how the common conditions module works
fn time_passed(t: f32) -> impl FnMut(Local<f32>, Res<Time>) -> bool {
    move |mut timer: Local<f32>, time: Res<Time>| {
        // Tick the timer
        *timer += time.delta_seconds();
        // Return true if the timer has passed the time
        *timer >= t
    }
}

/// SYSTEM: Adds the input counter resource
fn add_input_counter(mut commands: Commands) {
    commands.init_resource::<InputCounter>();
}

/// SYSTEM: Increment the input counter
/// Notice how we can take just the `ResMut` and not have to wrap
/// it in an option incase it hasen't been initialized, this is becuase
/// it has a run codition that checks if the `InputCounter` resource exsists
fn increment_input_counter(mut counter: ResMut<InputCounter>) {
    counter.0 += 1;
}

/// SYSTEM: Print the input counter
fn print_input_counter(counter: Res<InputCounter>) {
    println!("Input counter: {}", counter.0);
}
