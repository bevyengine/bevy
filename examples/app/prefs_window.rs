//! Demonstrates persistence of user preferences for saving window position.
use std::time::Duration;

use bevy::{
    preferences::{
        PreferencesPlugin, ReflectSettingsGroup, SavePreferencesDeferred, SavePreferencesSync,
        SettingsGroup,
    },
    prelude::*,
    window::{ExitCondition, WindowCloseRequested, WindowMode, WindowResized, WindowResolution},
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
        .add_plugins(PreferencesPlugin::new("org.bevy.examples.prefs_window"))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                show_count,
                change_count,
                on_window_close,
                update_window_settings,
            ),
        )
        .add_plugins(init_window_pos)
        .run();
}

#[derive(Resource, SettingsGroup, Reflect, Default)]
#[reflect(Resource, SettingsGroup, Default)]
struct Counter {
    count: i32,
}

/// Settings group which remembers the current window position and size
#[derive(Resource, SettingsGroup, Reflect, Default, Clone, PartialEq)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "window")]
struct WindowSettings {
    position: Option<IVec2>,
    size: Option<UVec2>,
    fullscreen: bool,
}

#[derive(Component)]
struct CounterDisplay;

fn init_window_pos(app: &mut App) {
    let world = app.world_mut();
    let Some(window_settings) = world.get_resource::<WindowSettings>() else {
        return;
    };
    let window_settings = window_settings.clone();

    let Ok(mut window) = world.query::<&mut Window>().single_mut(world) else {
        warn!("window not found");
        return;
    };

    if let Some(position) = window_settings.position {
        window.position = WindowPosition::new(position);
    }

    if let Some(size) = window_settings.size {
        window.resolution = WindowResolution::new(size.x, size.y);
    }

    window.mode = if window_settings.fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
}

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
        commands.queue(SavePreferencesDeferred::default());
    }
}

/// System which keeps the window settings up to date when the user resizes or moves the window.
fn update_window_settings(
    mut move_events: MessageReader<WindowMoved>,
    mut resize_events: MessageReader<WindowResized>,
    windows: Query<&mut Window>,
    window_settings: ResMut<WindowSettings>,
    mut commands: Commands,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let mut window_changed = false;
    for _ in move_events.read() {
        window_changed = true;
    }

    for _ in resize_events.read() {
        window_changed = true;
    }

    if window_changed && store_window_settings(window_settings, window) {
        commands.queue(SavePreferencesDeferred(Duration::from_secs_f32(0.5)));
    }
}

fn store_window_settings(mut window_settings: ResMut<WindowSettings>, window: &Window) -> bool {
    window_settings.set_if_neq(WindowSettings {
        position: match window.position {
            WindowPosition::At(pos) => Some(pos),
            _ => None,
        },
        size: Some(UVec2::new(
            window.resolution.width() as u32,
            window.resolution.height() as u32,
        )),
        fullscreen: window.mode != WindowMode::Windowed,
    })
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
