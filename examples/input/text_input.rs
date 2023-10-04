//! Simple text input support
//!
//! Return creates a new line, backspace removes the last character.
//! Clicking toggle IME (Input Method Editor) support, but the font used as limited support of characters.
//! You should change the provided font with another one to test other languages input.

// This lint usually gives bad advice in the context of Bevy -- hiding complex queries behind
// type aliases tends to obfuscate code while offering no improvement in code cleanliness.
#![allow(clippy::type_complexity)]

use bevy::{input::keyboard::KeyboardInput, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                toggle_ime,
                listen_ime_events,
                listen_received_character_events,
                listen_keyboard_input_events,
                bubbling_text,
            ),
        )
        .run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands.spawn(
        TextBundle::from_sections([
            TextSection {
                value: "IME Enabled: ".to_string(),
                style: TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            },
            TextSection {
                value: "false\n".to_string(),
                style: TextStyle {
                    font: font.clone_weak(),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            },
            TextSection {
                value: "IME Active: ".to_string(),
                style: TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            },
            TextSection {
                value: "false\n".to_string(),
                style: TextStyle {
                    font: font.clone_weak(),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            },
            TextSection {
                value: "click to toggle IME, press return to start a new line\n\n".to_string(),
                style: TextStyle {
                    font: font.clone_weak(),
                    font_size: 18.0,
                    color: Color::WHITE,
                },
            },
            TextSection {
                value: "".to_string(),
                style: TextStyle {
                    font,
                    font_size: 25.0,
                    color: Color::WHITE,
                },
            },
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
    );

    commands.spawn(Text2dBundle {
        text: Text::from_section(
            "".to_string(),
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 100.0,
                color: Color::WHITE,
            },
        ),
        ..default()
    });
}

fn toggle_ime(
    input: Res<Input<MouseButton>>,
    mut windows: Query<&mut Window>,
    mut text: Query<&mut Text, With<Node>>,
) {
    if input.just_pressed(MouseButton::Left) {
        let mut window = windows.single_mut();

        window.ime_position = window.cursor_position().unwrap();
        window.ime_enabled = !window.ime_enabled;

        let mut text = text.single_mut();
        text.sections[1].value = format!("{}\n", window.ime_enabled);
    }
}

#[derive(Component)]
struct Bubble {
    timer: Timer,
}

#[derive(Component)]
struct ImePreedit;

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
    mut status_text: Query<&mut Text, With<Node>>,
    mut edit_text: Query<&mut Text, (Without<Node>, Without<Bubble>)>,
) {
    for event in events.read() {
        match event {
            Ime::Preedit { value, cursor, .. } if !cursor.is_none() => {
                status_text.single_mut().sections[5].value = format!("IME buffer: {value}");
            }
            Ime::Preedit { cursor, .. } if cursor.is_none() => {
                status_text.single_mut().sections[5].value = "".to_string();
            }
            Ime::Commit { value, .. } => {
                edit_text.single_mut().sections[0].value.push_str(value);
            }
            Ime::Enabled { .. } => {
                status_text.single_mut().sections[3].value = "true\n".to_string();
            }
            Ime::Disabled { .. } => {
                status_text.single_mut().sections[3].value = "false\n".to_string();
            }
            _ => (),
        }
    }
}

fn listen_received_character_events(
    mut events: EventReader<ReceivedCharacter>,
    mut edit_text: Query<&mut Text, (Without<Node>, Without<Bubble>)>,
) {
    for event in events.read() {
        edit_text.single_mut().sections[0].value.push(event.char);
    }
}

fn listen_keyboard_input_events(
    mut commands: Commands,
    mut events: EventReader<KeyboardInput>,
    mut edit_text: Query<(Entity, &mut Text), (Without<Node>, Without<Bubble>)>,
) {
    for event in events.read() {
        match event.key_code {
            Some(KeyCode::Return) => {
                let (entity, text) = edit_text.single();
                commands.entity(entity).insert(Bubble {
                    timer: Timer::from_seconds(5.0, TimerMode::Once),
                });

                commands.spawn(Text2dBundle {
                    text: Text::from_section("".to_string(), text.sections[0].style.clone()),
                    ..default()
                });
            }
            Some(KeyCode::Back) => {
                edit_text.single_mut().1.sections[0].value.pop();
            }
            _ => continue,
        }
    }
}
