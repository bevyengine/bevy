//! Generic types allow us to reuse logic across many related systems,
//! allowing us to specialize our function's behavior based on which type (or types) are passed in.
//!
//! This is commonly useful for working on related components or resources,
//! where we want to have unique types for querying purposes but want them all to work the same way.
//! This is particularly powerful when combined with user-defined traits to add more functionality to these related types.
//! Remember to insert a specialized copy of the system into the schedule for each type that you want to operate on!
//!
//! For more advice on working with generic types in Rust, check out <https://doc.rust-lang.org/book/ch10-01-syntax.html>
//! or <https://doc.rust-lang.org/rust-by-example/generics.html>

use bevy::prelude::*;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    MainMenu,
    InGame,
}

#[derive(Component)]
struct TextToPrint(String);

#[derive(Component, Deref, DerefMut)]
struct PrinterTick(Timer);

#[derive(Component)]
struct MenuClose;

#[derive(Component)]
struct LevelUnload;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_state::<AppState>()
        .add_systems(Startup, setup_system)
        .add_systems(
            Update,
            (
                print_text_system,
                transition_to_in_game_system.run_if(in_state(AppState::MainMenu)),
            ),
        )
        // Cleanup systems.
        // Pass in the types your system should operate on using the ::<T> (turbofish) syntax
        .add_systems(OnExit(AppState::MainMenu), cleanup_system::<MenuClose>)
        .add_systems(OnExit(AppState::InGame), cleanup_system::<LevelUnload>)
        .run();
}

fn setup_system(mut commands: Commands) {
    commands.spawn((
        PrinterTick(Timer::from_seconds(1.0, TimerMode::Repeating)),
        TextToPrint("I will print until you press space.".to_string()),
        MenuClose,
    ));

    commands.spawn((
        PrinterTick(Timer::from_seconds(1.0, TimerMode::Repeating)),
        TextToPrint("I will always print".to_string()),
        LevelUnload,
    ));
}

fn print_text_system(time: Res<Time>, mut query: Query<(&mut PrinterTick, &TextToPrint)>) {
    for (mut timer, text) in &mut query {
        if timer.tick(time.delta()).just_finished() {
            info!("{}", text.0);
        }
    }
}

fn transition_to_in_game_system(
    mut next_state: ResMut<NextState<AppState>>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if keyboard_input.pressed(KeyCode::Space) {
        next_state.set(AppState::InGame);
    }
}

// Type arguments on functions come after the function name, but before ordinary arguments.
// Here, the `Component` trait is a trait bound on T, our generic type
fn cleanup_system<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for e in &query {
        commands.entity(e).despawn_recursive();
    }
}
