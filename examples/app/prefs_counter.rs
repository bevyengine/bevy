//! Demonstrates persistence of user preferences.
use std::time::Duration;

use bevy::{
    // user_prefs::{Preferences, StartAutosaveTimer},
    preferences::{
        LoadPreferences as _, PreferencesPlugin, ReflectSettingsGroup, SavePreferencesDeferred,
        SavePreferencesSync, SettingsGroup,
    },
    prelude::*,
    window::{ExitCondition, WindowCloseRequested},
};

fn main() {
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
        .add_plugins(PreferencesPlugin::new("org.bevy.examples.prefs_counter"))
        .add_systems(Startup, setup)
        .add_systems(Update, (show_count, change_count, on_window_close))
        .load_preferences()
        .run();
}

#[derive(Resource, SettingsGroup, Reflect, Default)]
#[reflect(Resource, SettingsGroup, Default)]
struct Counter {
    count: i32,
}

/// A different settings group which has the name group name as the previous. The two groups will be
/// merged into a single section in the config file.
#[derive(Resource, SettingsGroup, Reflect, Default)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "counter")]
#[allow(dead_code)]
struct OtherSettings {
    enabled: bool,
}

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
                    font_size: FontSize::Px(33.0),
                    ..default()
                },
                CounterDisplay,
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));
            parent.spawn((
                Text::new("Press SPACE to increment"),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
            ));
        });
}

fn show_count(mut query: Query<&mut Text, With<CounterDisplay>>, counter: Res<Counter>) {
    if counter.is_changed() {
        for mut text in query.iter_mut() {
            text.0 = format!("Count: {}", counter.count);
        }
    }
}

fn change_count(
    mut counter: ResMut<Counter>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    let mut changed = false;
    if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::Period) {
        counter.count += 1;
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::Backspace)
        || keyboard.just_pressed(KeyCode::Delete)
        || keyboard.just_pressed(KeyCode::Comma)
    {
        counter.count -= 1;
        changed = true;
    }

    if changed {
        commands.queue(SavePreferencesDeferred(Duration::from_secs_f32(0.1)));
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
