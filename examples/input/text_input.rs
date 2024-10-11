//! Simple text input support
//!
//! Return creates a new line, backspace removes the last character.
//! Clicking toggle IME (Input Method Editor) support, but the font used as limited support of characters.
//! You should change the provided font with another one to test other languages input.

use std::mem;

use bevy::{
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                toggle_ime,
                listen_ime_events,
                listen_keyboard_input_events,
                bubbling_text,
            ),
        )
        .run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // The default font has a limited number of glyphs, so use the full version for
    // sections that will hold text input.
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands
        .spawn((
            Text::default(),
            Style {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn(TextSpan::new(
                "Click to toggle IME. Press return to start a new line.\n\n",
            ));
            p.spawn(TextSpan::new("IME Enabled: "));
            p.spawn(TextSpan::new("false\n"));
            p.spawn(TextSpan::new("IME Active:  "));
            p.spawn(TextSpan::new("false\n"));
            p.spawn(TextSpan::new("IME Buffer:  "));
            p.spawn((
                TextSpan::new("\n"),
                TextStyle {
                    font: font.clone(),
                    ..default()
                },
            ));
        });

    commands.spawn((
        Text2d::new(""),
        TextStyle {
            font,
            font_size: 100.0,
            ..default()
        },
    ));
}

fn toggle_ime(
    input: Res<ButtonInput<MouseButton>>,
    mut windows: Query<&mut Window>,
    status_text: Query<Entity, (With<Node>, With<Text>)>,
    mut ui_writer: UiTextWriter,
) {
    if input.just_pressed(MouseButton::Left) {
        let mut window = windows.single_mut();

        window.ime_position = window.cursor_position().unwrap();
        window.ime_enabled = !window.ime_enabled;

        *ui_writer.text(status_text.single(), 3) = format!("{}\n", window.ime_enabled);
    }
}

#[derive(Component)]
struct Bubble {
    timer: Timer,
}

fn bubbling_text(
    mut commands: Commands,
    mut bubbles: Query<(Entity, &mut Transform, &mut Bubble)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut bubble) in bubbles.iter_mut() {
        if bubble.timer.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn();
        }
        transform.translation.y += time.delta_seconds() * 100.0;
    }
}

fn listen_ime_events(
    mut events: EventReader<Ime>,
    status_text: Query<Entity, (With<Node>, With<Text>)>,
    mut edit_text: Query<&mut Text2d, (Without<Node>, Without<Bubble>)>,
    mut ui_writer: UiTextWriter,
) {
    for event in events.read() {
        match event {
            Ime::Preedit { value, cursor, .. } if !cursor.is_none() => {
                *ui_writer.text(status_text.single(), 7) = format!("{value}\n");
            }
            Ime::Preedit { cursor, .. } if cursor.is_none() => {
                *ui_writer.text(status_text.single(), 7) = "\n".to_string();
            }
            Ime::Commit { value, .. } => {
                edit_text.single_mut().push_str(value);
            }
            Ime::Enabled { .. } => {
                *ui_writer.text(status_text.single(), 5) = "true\n".to_string();
            }
            Ime::Disabled { .. } => {
                *ui_writer.text(status_text.single(), 5) = "false\n".to_string();
            }
            _ => (),
        }
    }
}

fn listen_keyboard_input_events(
    mut commands: Commands,
    mut events: EventReader<KeyboardInput>,
    mut edit_text: Query<(&mut Text2d, &TextStyle), (Without<Node>, Without<Bubble>)>,
) {
    for event in events.read() {
        // Only trigger changes when the key is first pressed.
        if !event.state.is_pressed() {
            continue;
        }

        match &event.logical_key {
            Key::Enter => {
                let (mut text, style) = edit_text.single_mut();
                if text.is_empty() {
                    continue;
                }
                let old_value = mem::take(&mut **text);

                commands.spawn((
                    Text2d::new(old_value),
                    style.clone(),
                    Bubble {
                        timer: Timer::from_seconds(5.0, TimerMode::Once),
                    },
                ));
            }
            Key::Space => {
                edit_text.single_mut().0.push(' ');
            }
            Key::Backspace => {
                edit_text.single_mut().0.pop();
            }
            Key::Character(character) => {
                edit_text.single_mut().0.push_str(character);
            }
            _ => continue,
        }
    }
}
