use bevy::{ecs::component::Component, prelude::*};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    MainMenu,
    InGame,
}

struct TextToPrint(String);

struct MenuClose;
struct LevelUnload;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_state(AppState::MainMenu)
        .add_startup_system(setup)
        .add_system(print_text)
        .add_system_set(SystemSet::on_update(AppState::MainMenu).with_system(transition_to_in_game))
        // add the cleanup systems
        .add_system_set(
            // Pass in the types your system should operate on using the ::<T> (turbofish) syntax
            SystemSet::on_exit(AppState::MainMenu).with_system(cleanup_system::<MenuClose>),
        )
        .add_system_set(
            SystemSet::on_exit(AppState::InGame).with_system(cleanup_system::<LevelUnload>),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands
        .spawn()
        .insert(Timer::from_seconds(1.0, true))
        .insert(TextToPrint(
            "I will print until you press space.".to_string(),
        ))
        .insert(MenuClose);

    commands
        .spawn()
        .insert(Timer::from_seconds(1.0, true))
        .insert(TextToPrint("I will always print".to_string()))
        .insert(LevelUnload);
}

fn print_text(time: Res<Time>, mut query: Query<(&mut Timer, &TextToPrint)>) {
    for (mut timer, text) in query.iter_mut() {
        if timer.tick(time.delta()).just_finished() {
            info!("{}", text.0);
        }
    }
}

fn transition_to_in_game(mut state: ResMut<State<AppState>>, keyboard_input: Res<Input<KeyCode>>) {
    if keyboard_input.pressed(KeyCode::Space) {
        state.set(AppState::InGame).unwrap();
    }
}

// Type arguments on functions come after the function name, but before ordinary arguments.
// Here, the `Component` trait is a trait bound on T, our generic type
fn cleanup_system<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for e in query.iter() {
        commands.entity(e).despawn_recursive();
    }
}
