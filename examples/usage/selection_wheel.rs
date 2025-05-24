//! Radial selection wheel example
//!
//! This example demonstrates a selection wheel with styling and as many different input methods as possible.
use std::f32::consts::{PI, TAU};

use bevy::{
    input::{keyboard::KeyboardInput, ButtonState},
    input_focus::{
        tab_navigation::{NavAction, TabGroup, TabIndex, TabNavigation, TabNavigationPlugin},
        InputDispatchPlugin, InputFocus,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MeshPickingPlugin,
            InputDispatchPlugin,
            TabNavigationPlugin,
        ))
        .init_resource::<WheelSelected>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (keyboard_system, gamepad_system, focus_system),
                update_selected_system,
            )
                .chain(),
        )
        .run();
}

const NUM_BUTTONS: usize = 8;

#[derive(Resource, Default)]
struct WheelSelected {
    selected: Option<Entity>,
}

#[derive(Component)]
struct WheelButton {
    label: String,
}

#[derive(Component)]
struct WheelCenter;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    let text_field = |text: &str| {
        (
            TextFont {
                font_size: 12.0,
                ..default()
            },
            Text::new(text),
        )
    };
    commands.spawn((
        Name::new("Input help"),
        Node {
            display: Display::Flex,
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            column_gap: Val::Px(5.0),
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
        children![
            text_field("Press Enter to select the focused button"),
            text_field("Press Escape to clear the selection"),
            text_field("Press Tab or Shift+Tab to cycle focus through buttons"),
            text_field("Press Arrow keys to cycle focus through buttons"),
            text_field("Press number keys to select a button"),
            text_field("Use the mouse to hover over buttons and click to select"),
        ],
    ));

    // TODO: This could use a ring segment primitive: https://github.com/bevyengine/bevy/issues/10572
    // TODO: This should be a UI node instead of a Mesh2d with MeshPicking, which requires custom UI meshes or primitives: https://github.com/bevyengine/bevy/issues/14187
    let button_mesh = meshes.add(CircularSector::new(200.0, PI / NUM_BUTTONS as f32));

    // Spawn a tab group as the parent of all buttons
    let tab_group = commands
        .spawn((
            Name::new("Selection Wheel"),
            Transform::default(),
            TabGroup::default(),
            Visibility::default(),
        ))
        .id();

    // Spawn all the buttons
    for i in 0..NUM_BUTTONS {
        let percent = i as f32 / NUM_BUTTONS as f32;
        let color = Color::hsl(360.0 * percent, 0.95, 0.7);
        let button_material = materials.add(color);
        let angle = -percent * TAU;
        let rotation = Quat::from_rotation_z(angle);
        let transform = Transform::from_rotation(rotation);

        commands
            .spawn((
                ChildOf(tab_group),
                WheelButton {
                    label: format!("{i}"),
                },
                Mesh2d(button_mesh.clone()),
                MeshMaterial2d(button_material),
                TabIndex(i as i32),
                transform,
                children![(
                    Transform::default()
                        .with_translation(Vec3::new(0.0, 150.0, 0.0))
                        .with_rotation(Quat::from_rotation_z(-angle)),
                    Text2d::new(format!("{i}")),
                )],
            ))
            .observe(on_click)
            .observe(on_hover);
    }

    // Spawn the center of the wheel which shows the selected button
    commands.spawn((
        WheelCenter,
        Pickable::IGNORE,
        Mesh2d(meshes.add(Circle::new(100.0))),
        MeshMaterial2d(materials.add(Color::BLACK)),
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        Text2d::new(""),
    ));
}

fn on_click(trigger: Trigger<Pointer<Click>>, mut selected: ResMut<WheelSelected>) {
    selected.selected = Some(trigger.target);
}

fn on_hover(trigger: Trigger<Pointer<Over>>, mut focus: ResMut<InputFocus>) {
    focus.set(trigger.target);
}

/// Updates the wheel when the selected button changes
fn update_selected_system(
    mut query_center: Query<&mut Text2d, With<WheelCenter>>,
    query_buttons: Query<(Entity, &WheelButton, &Children)>,
    mut commands: Commands,
    selected: Res<WheelSelected>,
) -> Result {
    if selected.is_changed() {
        // Update center text
        let mut text = query_center.single_mut()?;
        if let Some(selected) = selected.selected {
            let (_, button, _) = query_buttons.get(selected)?;
            text.0 = button.label.clone();
        } else {
            text.0 = "".to_string();
        }

        // Update button styling
        for (entity, _, children) in query_buttons.iter() {
            let child = children.first().unwrap();
            let color = if Some(entity) == selected.selected {
                Color::BLACK
            } else {
                Color::WHITE
            };
            commands.entity(*child).insert(TextColor(color));
        }
    }

    Ok(())
}

/// Update styling when the focus changes
fn focus_system(
    focus: Res<InputFocus>,
    mut query: Query<(Entity, &mut Transform), With<WheelButton>>,
) {
    if focus.is_changed() {
        for (entity, mut transform) in query.iter_mut() {
            if Some(entity) == focus.0 {
                transform.scale = Vec3::splat(1.2);
            } else {
                transform.scale = Vec3::splat(1.0);
            }
        }
    }
}

/// Handle all keyboard input for the selection wheel
///
/// Tab navigation is handled automatically by [`bevy::input_focus`].
fn keyboard_system(
    mut keyboard_input_events: EventReader<KeyboardInput>,
    mut focus: ResMut<InputFocus>,
    tab_navigation: TabNavigation,
    button_query: Query<(Entity, &TabIndex), With<WheelButton>>,
    mut selected: ResMut<WheelSelected>,
) {
    for event in keyboard_input_events.read() {
        if event.state == ButtonState::Pressed {
            match event.key_code {
                KeyCode::Escape => selected.selected = None,
                KeyCode::Enter => {
                    if let Some(entity) = focus.0 {
                        if button_query.get(entity).is_ok() {
                            selected.selected = Some(entity);
                        }
                    }
                }
                KeyCode::ArrowLeft => {
                    if let Ok(entity) = tab_navigation.navigate(&focus, NavAction::Previous) {
                        focus.set(entity);
                    }
                }
                KeyCode::ArrowRight => {
                    if let Ok(entity) = tab_navigation.navigate(&focus, NavAction::Next) {
                        focus.set(entity);
                    }
                }
                _ => (),
            }

            if let Some(digit) = as_digit(event.key_code) {
                if let Some(entity) = button_query
                    .iter()
                    .find(|(_, TabIndex(n))| *n == digit)
                    .map(|(entity, _)| entity)
                {
                    selected.selected = Some(entity);
                }
            }
        }
    }
}

fn gamepad_system(
    gamepads: Query<&Gamepad>,
    mut _focus: ResMut<InputFocus>,
    _button_query: Query<Entity, With<WheelButton>>,
) {
    for gamepad in gamepads.iter() {
        let x = gamepad.get(GamepadAxis::LeftStickX).unwrap();
        let y = gamepad.get(GamepadAxis::LeftStickY).unwrap();
        if x.abs() + y.abs() > 0.1 {
            // TODO: set focus based on angle, I don't have a gamepad to test this
        }
    }
}

// TODO: add this as a method on KeyCode?
fn as_digit(keycode: KeyCode) -> Option<i32> {
    let digit = match keycode {
        KeyCode::Digit0 | KeyCode::Numpad0 => 0,
        KeyCode::Digit1 | KeyCode::Numpad1 => 1,
        KeyCode::Digit2 | KeyCode::Numpad2 => 2,
        KeyCode::Digit3 | KeyCode::Numpad3 => 3,
        KeyCode::Digit4 | KeyCode::Numpad4 => 4,
        KeyCode::Digit5 | KeyCode::Numpad5 => 5,
        KeyCode::Digit6 | KeyCode::Numpad6 => 6,
        KeyCode::Digit7 | KeyCode::Numpad7 => 7,
        KeyCode::Digit8 | KeyCode::Numpad8 => 8,
        KeyCode::Digit9 | KeyCode::Numpad9 => 9,
        _ => return None,
    };

    Some(digit)
}
