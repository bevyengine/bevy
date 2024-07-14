//! This example demonstrates how to use run conditions to control when systems run.

use bevy::prelude::*;

fn main() {
    println!();
    println!("For the first 2 seconds you will not be able to increment the counter");
    println!("Once that time has passed you can press space, enter, left mouse, right mouse or touch the screen to increment the counter");
    println!();

    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<InputCounter>()
        .add_systems(
            Update,
            (
                increment_input_counter
                    // The common_conditions module has a few useful run conditions
                    // for checking resources and states. These are included in the prelude.
                    .run_if(resource_exists::<InputCounter>)
                    // `.or()` is a run condition combinator that only evaluates the second condition
                    // if the first condition returns `false`. This behavior is known as "short-circuiting",
                    // and is how the `||` operator works in Rust (as well as most C-family languages).
                    // In this case, the `has_user_input` run condition will be evaluated since the `Unused` resource has not been initialized.
                    .run_if(resource_exists::<Unused>.or(
                        // This is a custom run condition, defined using a system that returns
                        // a `bool` and which has read-only `SystemParam`s.
                        // Both run conditions must return `true` in order for the system to run.
                        // Note that this second run condition will be evaluated even if the first returns `false`.
                        has_user_input,
                    )),
                print_input_counter
                    // `.and()` is a run condition combinator that only evaluates the second condition
                    // if the first condition returns `true`, analogous to the `&&` operator.
                    // In this case, the short-circuiting behavior prevents the second run condition from
                    // panicking if the `InputCounter` resource has not been initialized.
                    .run_if(resource_exists::<InputCounter>.and(
                        // This is a custom run condition in the form of a closure.
                        // This is useful for small, simple run conditions you don't need to reuse.
                        // All the normal rules still apply: all parameters must be read only except for local parameters.
                        |counter: Res<InputCounter>| counter.is_changed() && !counter.is_added(),
                    )),
                print_time_message
                    // This function returns a custom run condition, much like the common conditions module.
                    // It will only return true once 2 seconds have passed.
                    .run_if(time_passed(2.0))
                    // You can use the `not` condition from the common_conditions module
                    // to inverse a run condition. In this case it will return true if
                    // less than 2.5 seconds have elapsed since the app started.
                    .run_if(not(time_passed(2.5))),
            ),
        )
        .run();
}

#[derive(Resource, Default)]
struct InputCounter(usize);

#[derive(Resource)]
struct Unused;

/// Return true if any of the defined inputs were just pressed.
/// This is a custom run condition, it can take any normal system parameters as long as
/// they are read only (except for local parameters which can be mutable).
/// It returns a bool which determines if the system should run.
fn has_user_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    touch_input: Res<Touches>,
) -> bool {
    keyboard_input.just_pressed(KeyCode::Space)
        || keyboard_input.just_pressed(KeyCode::Enter)
        || mouse_button_input.just_pressed(MouseButton::Left)
        || mouse_button_input.just_pressed(MouseButton::Right)
        || touch_input.any_just_pressed()
}

/// This is a function that returns a closure which can be used as a run condition.
/// This is useful because you can reuse the same run condition but with different variables.
/// This is how the common conditions module works.
fn time_passed(t: f32) -> impl FnMut(Local<f32>, Res<Time>) -> bool {
    move |mut timer: Local<f32>, time: Res<Time>| {
        // Tick the timer
        *timer += time.delta_seconds();
        // Return true if the timer has passed the time
        *timer >= t
    }
}

/// SYSTEM: Increment the input counter
/// Notice how we can take just the `ResMut` and not have to wrap
/// it in an option in case it hasn't been initialized, this is because
/// it has a run condition that checks if the `InputCounter` resource exists
fn increment_input_counter(mut counter: ResMut<InputCounter>) {
    counter.0 += 1;
}

/// SYSTEM: Print the input counter
fn print_input_counter(counter: Res<InputCounter>) {
    println!("Input counter: {}", counter.0);
}

/// SYSTEM: Adds the input counter resource
fn print_time_message() {
    println!("It has been more than 2 seconds since the program started and less than 2.5 seconds");
}
