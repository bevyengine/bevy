use bevy::prelude::*;

const DEFAULT_MANA: u32 = 10;

#[derive(Component)]
struct Player {
    mana: u32,
}

fn window_title_system(mut windows: Query<&mut Window>) {
    for (index, mut window) in windows.iter_mut().enumerate() {
        window.title = format!("This is window {index}!");
    }
}

fn spawn_player(mut commands: Commands) {
    commands.spawn(Player { mana: DEFAULT_MANA });
}

fn spell_casting(mut player: Query<&mut Player>, keyboard_input: Res<Input<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        let Ok(mut player) = player.get_single_mut() else {
            return;
        };

        if player.mana > 0 {
            player.mana -= 1;
        }
    }
}

fn create_test_app() -> App {
    let mut app = App::new();

    // Note how we use `MinimalPlugins` instead of `DefaultPlugins`.
    // This is what allows the test to run without a window, or real user input.
    app.add_plugins(MinimalPlugins);
    // Inserting a KeyCode input resource allows us to inject keyboard inputs,
    // as if the user had pressed them.
    app.insert_resource(Input::<KeyCode>::default());

    // Spawning a fake window allows testing systems that require a window.
    app.world.spawn(Window::default());

    app
}

fn add_game_systems(app: &mut App) {
    // This could be a subset of your game's systems, or the entire app.
    // As long as you make sure to add a fake version of inputs, windows, and any
    // other things that your game's systems rely on.
    app.add_startup_system(spawn_player)
        .add_startup_system(window_title_system)
        .add_system(spell_casting);
}

#[test]
fn test_player_spawn() {
    let mut app = create_test_app();
    add_game_systems(&mut app);

    // The `update` function needs to be called at least once for the startup
    // systems to run.
    app.update();

    // Now that the startup systems have run, we can check if the player has
    // spawned as expected.
    let player = app.world.query::<&Player>().get_single(&app.world);
    assert!(player.is_ok(), "There should be exactly 1 player.");
    assert_eq!(
        player.unwrap().mana,
        DEFAULT_MANA,
        "Player does not have expected starting mana."
    );
}

#[test]
fn test_spell_casting() {
    let mut app = create_test_app();
    add_game_systems(&mut app);

    // We simulate pressing `space` to trigger the spell casting system.
    app.world
        .resource_mut::<Input<KeyCode>>()
        .press(KeyCode::Space);
    // Allow the systems to realize space got pressed.
    app.update();

    // The spell casting should have used up a single mana.
    let player = app.world.query::<&Player>().single(&app.world);
    assert_eq!(player.mana, DEFAULT_MANA - 1);

    // Clear the `just_pressed` status for all `KeyCode`s
    app.world.resource_mut::<Input<KeyCode>>().clear();
    app.update();

    // No extra spells should have been cast, so no mana should have been lost.
    let player = app.world.query::<&Player>().single(&app.world);
    assert_eq!(player.mana, DEFAULT_MANA - 1);
}

#[test]
fn test_faking_windows() {
    let mut app = create_test_app();
    add_game_systems(&mut app);

    // The `update` function needs to be called at least once for the startup
    // systems to run.
    app.update();

    let window = app.world.query::<&Window>().single(&app.world);
    assert_eq!(window.title, "This is window 0!");
}
