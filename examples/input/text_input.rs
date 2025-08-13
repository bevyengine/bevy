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

    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        children![
            TextSpan::new("Click to toggle IME. Press return to start a new line.\n\n",),
            TextSpan::new("IME Enabled: "),
            TextSpan::new("false\n"),
            TextSpan::new("IME Active:  "),
            TextSpan::new("false\n"),
            TextSpan::new("IME Buffer:  "),
            (
                TextSpan::new("\n"),
                TextFont {
                    font: font.clone(),
                    ..default()
                },
            ),
        ],
    ));

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font,
            font_size: 100.0,
            ..default()
        },
    ));
}

fn toggle_ime(
    input: Res<ButtonInput<MouseButton>>,
    mut window: Single<&mut Window>,
    status_text: Single<Entity, (With<Node>, With<Text>)>,
    mut ui_writer: TextUiWriter,
) {
    if input.just_pressed(MouseButton::Left) {
        window.ime_position = window.cursor_position().unwrap();
        window.ime_enabled = !window.ime_enabled;

        *ui_writer.text(*status_text, 3) = format!("{}\n", window.ime_enabled);
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
        transform.translation.y += time.delta_secs() * 100.0;
    }
}

fn listen_ime_events(
    mut events: EventReader<Ime>,
    status_text: Single<Entity, (With<Node>, With<Text>)>,
    mut edit_text: Single<&mut Text2d, (Without<Node>, Without<Bubble>)>,
    mut ui_writer: TextUiWriter,
) {
    for event in events.read() {
        match event {
            Ime::Preedit { value, cursor, .. } if !cursor.is_none() => {
                *ui_writer.text(*status_text, 7) = format!("{value}\n");
            }
            Ime::Preedit { cursor, .. } if cursor.is_none() => {
                *ui_writer.text(*status_text, 7) = "\n".to_string();
            }
            Ime::Commit { value, .. } => {
                edit_text.push_str(value);
            }
            Ime::Enabled { .. } => {
                *ui_writer.text(*status_text, 5) = "true\n".to_string();
            }
            Ime::Disabled { .. } => {
                *ui_writer.text(*status_text, 5) = "false\n".to_string();
            }
            _ => (),
        }
    }
}

fn listen_keyboard_input_events(
    mut commands: Commands,
    mut events: EventReader<KeyboardInput>,
    edit_text: Single<(&mut Text2d, &TextFont), (Without<Node>, Without<Bubble>)>,
) {
    let (mut text, style) = edit_text.into_inner();
    for event in events.read() {
        // Only trigger changes when the key is first pressed.
        if !event.state.is_pressed() {
            continue;
        }

        match (&event.logical_key, &event.text) {
            (Key::Enter, _) => {
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
            (Key::Backspace, _) => {
                text.pop();
            }
            (_, Some(inserted_text)) => {
                // Make sure the text doesn't have any control characters,
                // which can happen when keys like Escape are pressed
                if inserted_text.chars().all(is_printable_char) {
                    text.push_str(inserted_text);
                }
            }
            _ => continue,
        }
    }
}

// this logic is taken from egui-winit:
// https://github.com/emilk/egui/blob/adfc0bebfc6be14cee2068dee758412a5e0648dc/crates/egui-winit/src/lib.rs#L1014-L1024
fn is_printable_char(chr: char) -> bool {
    let is_in_private_use_area = ('\u{e000}'..='\u{f8ff}').contains(&chr)
        || ('\u{f0000}'..='\u{ffffd}').contains(&chr)
        || ('\u{100000}'..='\u{10fffd}').contains(&chr);

    !is_in_private_use_area && !chr.is_ascii_control()
}
