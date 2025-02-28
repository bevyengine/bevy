//! Demonstrates simple integration testing of Bevy applications.
//!
//! By substituting [`DefaultPlugins`] with [`MinimalPlugins`], Bevy can run completely headless.
//!
//! The list of minimal plugins does not include things like window or input handling. The downside
//! of this is that resources or entities associated with those systems (for example:
//! `ButtonInput::<KeyCode>`) need to be manually added, either directly or via e.g.
//! [`InputPlugin`]. The upside, however, is that the test has complete control over these
//! resources, meaning we can fake user input, fake the window being moved around, and more.
use bevy::prelude::*;

#[derive(Component)]
struct Player {
    mana: u32,
}

impl Default for Player {
    fn default() -> Self {
        Self { mana: 10 }
    }
}

/// Splitting a Bevy project into multiple smaller plugins can make it more testable. We can
/// write tests for individual plugins in isolation, as well as for the entire project.
fn game_plugin(app: &mut App) {
    app.add_systems(Startup, (spawn_player, window_title_system).chain());
    app.add_systems(Update, spell_casting);
}

fn window_title_system(mut windows: Query<&mut Window>) {
    for (index, mut window) in windows.iter_mut().enumerate() {
        window.title = format!("This is window {index}!");
    }
}

fn spawn_player(mut commands: Commands) {
    commands.spawn(Player::default());
}

fn spell_casting(mut player: Query<&mut Player>, keyboard_input: Res<ButtonInput<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        let Ok(mut player) = player.single_mut() else {
            return;
        };

        if player.mana > 0 {
            player.mana -= 1;
        }
    }
}

fn create_test_app() -> App {
    let mut app = App::new();

    // Note the use of `MinimalPlugins` instead of `DefaultPlugins`, as described above.
    app.add_plugins(MinimalPlugins);
    // Inserting a `KeyCode` input resource allows us to inject keyboard inputs, as if the user had
    // pressed them.
    app.insert_resource(ButtonInput::<KeyCode>::default());

    // Spawning a fake window allows testing systems that require a window.
    app.world_mut().spawn(Window::default());

    app
}

#[test]
fn test_player_spawn() {
    let mut app = create_test_app();
    app.add_plugins(game_plugin);

    // The `update` function needs to be called at least once for the startup
    // systems to run.
    app.update();

    // Now that the startup systems have run, we can check if the player has
    // spawned as expected.
    let expected = Player::default();
    let actual = app.world_mut().query::<&Player>().single(app.world());
    assert!(actual.is_ok(), "There should be exactly 1 player.");
    assert_eq!(
        expected.mana,
        actual.unwrap().mana,
        "Player does not have expected starting mana."
    );
}

#[test]
fn test_spell_casting() {
    let mut app = create_test_app();
    app.add_plugins(game_plugin);

    // Simulate pressing space to trigger the spell casting system.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    // Allow the systems to recognize the input event.
    app.update();

    let expected = Player::default();
    let actual = app
        .world_mut()
        .query::<&Player>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        expected.mana - 1,
        actual.mana,
        "A single mana point should have been used."
    );

    // Clear the `just_pressed` status for all `KeyCode`s
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();

    // No extra spells have been cast, so no mana should have been used.
    let after_keypress_event = app
        .world_mut()
        .query::<&Player>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        expected.mana - 1,
        after_keypress_event.mana,
        "No further mana should have been used."
    );
}

#[test]
fn test_window_title() {
    let mut app = create_test_app();
    app.add_plugins(game_plugin);

    app.update();

    let window = app
        .world_mut()
        .query::<&Window>()
        .single(app.world())
        .unwrap();
    assert_eq!(window.title, "This is window 0!");
}
