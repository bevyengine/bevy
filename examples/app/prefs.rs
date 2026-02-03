//! Demonstrates persistence of user preferences.
use bevy::{
    prelude::*,
    user_prefs::{AutosavePrefsPlugin, Preferences, SavePreferencesSync, StartAutosaveTimer},
    window::{ExitCondition, WindowCloseRequested},
};

fn main() {
    // Configure preferences store
    let mut preferences = Preferences::new("org.bevy.example.prefs");
    let count: i32 = preferences
        .get("prefs")
        .map(|file| {
            file.get_group("counter")
                .map(|group| group.get::<i32>("count").unwrap_or(0))
                .unwrap_or(0)
        })
        .unwrap_or(0);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            // We want to intercept the exit so that we can save prefs.
            exit_condition: ExitCondition::DontExit,
            primary_window: Some(Window {
                title: "Prefs Counter".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(AutosavePrefsPlugin)
        .insert_resource(preferences)
        .insert_resource(Counter(count))
        .add_systems(Startup, setup)
        .add_systems(Update, (show_count, change_count, on_window_close))
        .run();
}

#[derive(Resource)]
struct Counter(i32);

#[derive(Component)]
struct CounterDisplay;

fn setup(mut commands: Commands) {
    commands.spawn((Camera::default(), Camera2d));
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new("---"),
                TextFont {
                    font_size: 33.0,
                    ..default()
                },
                CounterDisplay,
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));
            parent.spawn((
                Text::new("Press SPACE to increment"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
            ));
        });
}

fn show_count(mut query: Query<&mut Text, With<CounterDisplay>>, counter: Res<Counter>) {
    if counter.is_changed() {
        for mut text in query.iter_mut() {
            text.0 = format!("Count: {}", counter.0);
        }
    }
}

fn change_count(
    mut counter: ResMut<Counter>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut prefs: ResMut<Preferences>,
    mut commands: Commands,
) {
    let mut changed = false;
    if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::Period) {
        counter.0 += 1;
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::Backspace)
        || keyboard.just_pressed(KeyCode::Delete)
        || keyboard.just_pressed(KeyCode::Comma)
    {
        counter.0 -= 1;
        changed = true;
    }

    if changed && let Some(app_prefs) = prefs.get_mut("prefs") {
        let mut counter_prefs = app_prefs.get_group_mut("counter").unwrap();
        counter_prefs.set("count", counter.0);
        commands.queue(StartAutosaveTimer);
    }
}

fn on_window_close(mut close: MessageReader<WindowCloseRequested>, mut commands: Commands) {
    // Save preferences immediately, then quit.
    if let Some(_close_event) = close.read().next() {
        commands.queue(SavePreferencesSync::IfChanged);
        commands.queue(ExitAfterSave);
    }
}

struct ExitAfterSave;

impl Command for ExitAfterSave {
    fn apply(self, world: &mut World) {
        world.write_message(AppExit::Success);
    }
}
