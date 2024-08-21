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

#[derive(Component)]
struct ImeEnabledText;

#[derive(Component)]
struct ImeActiveText;

#[derive(Component)]
struct ImeBufferText;

#[derive(Component)]
struct EditText;

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands
        .spawn(TextBundle::default().with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }))
        .with_children(|parent| {
            parent.spawn(TextSection {
                value: "IME Enabled: ".to_string(),
                style: TextStyle {
                    font: font.clone_weak(),
                    ..default()
                },
            });
            parent.spawn((
                TextSection {
                    value: "false\n".to_string(),
                    style: TextStyle {
                        font: font.clone_weak(),
                        font_size: 30.0,
                        ..default()
                    },
                },
                ImeEnabledText,
            ));
            parent.spawn(TextSection {
                value: "IME Active: ".to_string(),
                style: TextStyle {
                    font: font.clone_weak(),
                    ..default()
                },
            });
            parent.spawn((
                TextSection {
                    value: "false\n".to_string(),
                    style: TextStyle {
                        font: font.clone_weak(),
                        font_size: 30.0,
                        ..default()
                    },
                },
                ImeActiveText,
            ));
            parent.spawn(TextSection {
                value: "click to toggle IME, press return to start a new line\n\n".to_string(),
                style: TextStyle {
                    font: font.clone_weak(),
                    font_size: 18.0,
                    ..default()
                },
            });
            parent.spawn((
                TextSection {
                    value: "".to_string(),
                    style: TextStyle {
                        font,
                        font_size: 25.0,
                        ..default()
                    },
                },
                ImeBufferText,
            ));
        });

    commands.spawn(Text2dBundle::default()).with_child((
        TextSection::new(
            "".to_string(),
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 100.0,
                ..default()
            },
        ),
        EditText,
    ));
}

fn toggle_ime(
    input: Res<ButtonInput<MouseButton>>,
    mut windows: Query<&mut Window>,
    mut text: Query<&mut TextSection, With<ImeEnabledText>>,
) {
    if input.just_pressed(MouseButton::Left) {
        let mut window = windows.single_mut();

        window.ime_position = window.cursor_position().unwrap();
        window.ime_enabled = !window.ime_enabled;

        let mut text = text.single_mut();
        text.value = format!("{}\n", window.ime_enabled);
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
            commands.entity(entity).despawn_recursive();
        }
        transform.translation.y += time.delta_seconds() * 100.0;
    }
}

fn listen_ime_events(
    mut events: EventReader<Ime>,
    mut text_query: Query<&mut TextSection>,
    ime_buffer_query: Query<Entity, With<ImeBufferText>>,
    ime_active_query: Query<Entity, With<ImeActiveText>>,
    edit_text_query: Query<Entity, With<EditText>>,
) {
    for event in events.read() {
        match event {
            Ime::Preedit { value, cursor, .. } if !cursor.is_none() => {
                let ime_buffer_entity = ime_buffer_query.single();
                text_query.get_mut(ime_buffer_entity).unwrap().value =
                    format!("IME buffer: {value}");
            }
            Ime::Preedit { cursor, .. } if cursor.is_none() => {
                let ime_buffer_entity = ime_buffer_query.single();
                text_query.get_mut(ime_buffer_entity).unwrap().value = "".to_string();
            }
            Ime::Commit { value, .. } => {
                let mut edit_text = text_query.get_mut(edit_text_query.single()).unwrap();
                edit_text.value.push_str(value);
            }
            Ime::Enabled { .. } => {
                let ime_buffer_entity = ime_active_query.single();
                text_query.get_mut(ime_buffer_entity).unwrap().value = "true\n".to_string();
            }
            Ime::Disabled { .. } => {
                let ime_buffer_entity = ime_active_query.single();
                text_query.get_mut(ime_buffer_entity).unwrap().value = "false\n".to_string();
            }
            _ => (),
        }
    }
}

fn listen_keyboard_input_events(
    mut commands: Commands,
    mut events: EventReader<KeyboardInput>,
    mut edit_text: Query<&mut TextSection, With<EditText>>,
) {
    for event in events.read() {
        // Only trigger changes when the key is first pressed.
        if !event.state.is_pressed() {
            continue;
        }

        match &event.logical_key {
            Key::Enter => {
                let mut text = edit_text.single_mut();
                if text.value.is_empty() {
                    continue;
                }
                let old_value = mem::take(&mut text.value);

                commands
                    .spawn((
                        Text2dBundle::default(),
                        Bubble {
                            timer: Timer::from_seconds(5.0, TimerMode::Once),
                        },
                    ))
                    .with_child(TextSection::new(old_value, text.style.clone()));
            }
            Key::Space => {
                edit_text.single_mut().value.push(' ');
            }
            Key::Backspace => {
                edit_text.single_mut().value.pop();
            }
            Key::Character(character) => {
                edit_text.single_mut().value.push_str(character);
            }
            _ => continue,
        }
    }
}
