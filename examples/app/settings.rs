//! Demonstrates persistence of settings.
//!
//! A counter is shown in the window. It can be incremented and decremented via input press.
//! Its value persists between app sessions via settings.
//!
//! On desktop, if you quit the app and then restart it, the counter value should display
//! the most recent value the app had before exiting. Settings are saved as TOML at
//! `{preferences_dir}/org.bevy.examples.settings/settings.toml` (see [`SettingsPlugin`]).
//! On web, if you navigate away and then come back to the window, the counter
//! should display the most recent value the app had before navigating away.
use std::time::Duration;

use bevy::{
    prelude::*,
    settings::{
        ReflectSettingsGroup, SaveSettingsDeferred, SaveSettingsSync, SettingsGroup, SettingsPlugin,
    },
    window::{ExitCondition, WindowCloseRequested},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            // We want to intercept the exit so that we can save settings.
            exit_condition: ExitCondition::DontExit,
            primary_window: Some(Window {
                title: "Settings Counter".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SettingsPlugin::new("org.bevy.examples.settings"))
        .add_systems(Startup, setup)
        .add_systems(Update, (show_count, change_count, on_window_close))
        .run();
}

#[derive(Resource, SettingsGroup, Reflect, Default)]
#[reflect(Resource, SettingsGroup, Default)]
struct Counter {
    count: i32,
}

/// A different settings group which has the name group name as the previous. The two groups will be
/// merged into a single section in the config file.
#[derive(Resource, SettingsGroup, Reflect)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "counter")]
struct OtherSettings {
    enabled: bool,
}

impl Default for OtherSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Component)]
struct CounterDisplay;

fn setup(mut commands: Commands) {
    commands.spawn((Camera::default(), Camera2d));
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
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
                Text::new("Press SPACE to increment, BACKSPACE to decrement."),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
            ));
        });
}

fn show_count(
    mut query: Query<&mut Text, With<CounterDisplay>>,
    counter: Res<Counter>,
    other: Res<OtherSettings>,
) {
    if other.enabled {
        if counter.is_changed() {
            for mut text in query.iter_mut() {
                text.0 = format!("Count: {}", counter.count);
            }
        }
    } else {
        for mut text in query.iter_mut() {
            text.0 = "Disabled".into();
        }
    }
}

fn change_count(
    mut counter: ResMut<Counter>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    let mut changed = false;
    if keyboard.just_pressed(KeyCode::Space) {
        counter.count += 1;
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::Backspace) || keyboard.just_pressed(KeyCode::Delete) {
        counter.count -= 1;
        changed = true;
    }

    if changed {
        commands.queue(SaveSettingsDeferred(Duration::from_secs_f32(0.1)));
    }
}

fn on_window_close(mut close: MessageReader<WindowCloseRequested>, mut commands: Commands) {
    // Save settings immediately, then quit.
    if let Some(_close_event) = close.read().next() {
        commands.queue(SaveSettingsSync::IfChanged);
        commands.write_message(AppExit::Success);
    }
}
