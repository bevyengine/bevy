//! Demonstrates automatic directional navigation with zero configuration.
//!
//! Unlike the manual `directional_navigation` example, this shows how to use automatic
//! navigation by simply adding the `AutoDirectionalNavigation` component to UI elements.
//! The navigation graph is automatically built and maintained based on screen positions.
//!
//! This is especially useful for:
//! - Dynamic UIs where elements may be added, removed, or repositioned
//! - Irregular layouts that don't fit a simple grid pattern
//! - Prototyping where you want navigation without tedious manual setup
//!
//! The automatic system finds the nearest neighbor in each compass direction for every node,
//! completely eliminating the need to manually specify navigation relationships.

use core::time::Duration;

use bevy::{
    camera::NormalizedRenderTarget,
    input_focus::{
        directional_navigation::{
            AutoDirectionalNavigation, AutoNavigationConfig, DirectionalNavigation,
            DirectionalNavigationPlugin,
        },
        InputDispatchPlugin, InputFocus, InputFocusVisible,
    },
    math::{CompassOctant, Dir2},
    picking::{
        backend::HitData,
        pointer::{Location, PointerId},
    },
    platform::collections::HashSet,
    prelude::*,
};

fn main() {
    App::new()
        // Input focus is not enabled by default, so we need to add the corresponding plugins
        .add_plugins((
            DefaultPlugins,
            InputDispatchPlugin,
            DirectionalNavigationPlugin,
        ))
        // This resource is canonically used to track whether or not to render a focus indicator
        // It starts as false, but we set it to true here as we would like to see the focus indicator
        .insert_resource(InputFocusVisible(true))
        // Configure auto-navigation behavior
        .insert_resource(AutoNavigationConfig {
            // Require at least 10% overlap in perpendicular axis for cardinal directions
            min_alignment_factor: 0.1,
            // Don't connect nodes more than 500 pixels apart
            max_search_distance: Some(500.0),
            // Prefer nodes that are well-aligned
            prefer_aligned: true,
        })
        .init_resource::<ActionState>()
        .add_systems(Startup, setup_scattered_ui)
        // Navigation graph is automatically maintained by DirectionalNavigationPlugin!
        // No manual system needed - just add AutoDirectionalNavigation to entities.
        // Input is generally handled during PreUpdate
        .add_systems(PreUpdate, (process_inputs, navigate).chain())
        .add_systems(
            Update,
            (
                highlight_focused_element,
                interact_with_focused_button,
                reset_button_after_interaction,
                update_focus_display,
                update_key_display,
            ),
        )
        .add_observer(universal_button_click_behavior)
        .run();
}

const NORMAL_BUTTON: Srgba = bevy::color::palettes::tailwind::BLUE_400;
const PRESSED_BUTTON: Srgba = bevy::color::palettes::tailwind::BLUE_500;
const FOCUSED_BORDER: Srgba = bevy::color::palettes::tailwind::BLUE_50;

/// Marker component for the text that displays the currently focused button
#[derive(Component)]
struct FocusDisplay;

/// Marker component for the text that displays the last key pressed
#[derive(Component)]
struct KeyDisplay;

// Observer for button clicks
fn universal_button_click_behavior(
    mut click: On<Pointer<Click>>,
    mut button_query: Query<(&mut BackgroundColor, &mut ResetTimer)>,
) {
    let button_entity = click.entity;
    if let Ok((mut color, mut reset_timer)) = button_query.get_mut(button_entity) {
        color.0 = PRESSED_BUTTON.into();
        reset_timer.0 = Timer::from_seconds(0.3, TimerMode::Once);
        click.propagate(false);
    }
}

#[derive(Component, Default, Deref, DerefMut)]
struct ResetTimer(Timer);

fn reset_button_after_interaction(
    time: Res<Time>,
    mut query: Query<(&mut ResetTimer, &mut BackgroundColor)>,
) {
    for (mut reset_timer, mut color) in query.iter_mut() {
        reset_timer.tick(time.delta());
        if reset_timer.just_finished() {
            color.0 = NORMAL_BUTTON.into();
        }
    }
}

/// Spawn a scattered layout of buttons to demonstrate automatic navigation.
///
/// Unlike a regular grid, these buttons are irregularly positioned,
/// but auto-navigation will still figure out the correct connections!
fn setup_scattered_ui(mut commands: Commands, mut input_focus: ResMut<InputFocus>) {
    commands.spawn(Camera2d);

    // Create a full-screen background node
    let root_node = commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            ..default()
        })
        .id();

    // Instructions
    let instructions = commands
        .spawn((
            Text::new(
                "Automatic Navigation Demo\n\n\
                 Use arrow keys or D-pad to navigate.\n\
                 Press Enter or A button to interact.\n\n\
                 Buttons are scattered irregularly,\n\
                 but navigation is automatic!",
            ),
            Node {
                position_type: PositionType::Absolute,
                left: px(20),
                top: px(20),
                width: px(280),
                padding: UiRect::all(px(12)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
        ))
        .id();

    // Focus display - shows which button is currently focused
    commands.spawn((
        Text::new("Focused: None"),
        FocusDisplay,
        Node {
            position_type: PositionType::Absolute,
            left: px(20),
            bottom: px(80),
            width: px(280),
            padding: UiRect::all(px(12)),
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.5, 0.1, 0.8)),
        TextFont {
            font_size: 20.0,
            ..default()
        },
    ));

    // Key display - shows the last key pressed
    commands.spawn((
        Text::new("Last Key: None"),
        KeyDisplay,
        Node {
            position_type: PositionType::Absolute,
            left: px(20),
            bottom: px(20),
            width: px(280),
            padding: UiRect::all(px(12)),
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.5, 0.1, 0.5, 0.8)),
        TextFont {
            font_size: 20.0,
            ..default()
        },
    ));

    // Spawn buttons in a scattered/irregular pattern
    // The auto-navigation system will figure out the connections!
    let button_positions = [
        // Top row (irregular spacing)
        (350.0, 100.0),
        (520.0, 120.0),
        (700.0, 90.0),
        // Middle-top row
        (380.0, 220.0),
        (600.0, 240.0),
        // Center
        (450.0, 340.0),
        (620.0, 360.0),
        // Lower row
        (360.0, 480.0),
        (540.0, 460.0),
        (720.0, 490.0),
    ];

    let mut first_button = None;
    for (i, (x, y)) in button_positions.iter().enumerate() {
        let button_entity = commands
            .spawn((
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(*x),
                    top: px(*y),
                    width: px(140),
                    height: px(80),
                    border: UiRect::all(px(4)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                // This is the key: just add this component for automatic navigation!
                AutoDirectionalNavigation::default(),
                ResetTimer::default(),
                BackgroundColor::from(NORMAL_BUTTON),
                Name::new(format!("Button {}", i + 1)),
            ))
            .with_child((
                Text::new(format!("Button {}", i + 1)),
                TextLayout {
                    justify: Justify::Center,
                    ..default()
                },
            ))
            .id();

        if first_button.is_none() {
            first_button = Some(button_entity);
        }
    }

    commands.entity(root_node).add_children(&[instructions]);

    // Set initial focus
    if let Some(button) = first_button {
        input_focus.set(button);
    }
}

// Action state and input handling (same as the manual navigation example)
#[derive(Debug, PartialEq, Eq, Hash)]
enum DirectionalNavigationAction {
    Up,
    Down,
    Left,
    Right,
    Select,
}

impl DirectionalNavigationAction {
    fn variants() -> Vec<Self> {
        vec![
            DirectionalNavigationAction::Up,
            DirectionalNavigationAction::Down,
            DirectionalNavigationAction::Left,
            DirectionalNavigationAction::Right,
            DirectionalNavigationAction::Select,
        ]
    }

    fn keycode(&self) -> KeyCode {
        match self {
            DirectionalNavigationAction::Up => KeyCode::ArrowUp,
            DirectionalNavigationAction::Down => KeyCode::ArrowDown,
            DirectionalNavigationAction::Left => KeyCode::ArrowLeft,
            DirectionalNavigationAction::Right => KeyCode::ArrowRight,
            DirectionalNavigationAction::Select => KeyCode::Enter,
        }
    }

    fn gamepad_button(&self) -> GamepadButton {
        match self {
            DirectionalNavigationAction::Up => GamepadButton::DPadUp,
            DirectionalNavigationAction::Down => GamepadButton::DPadDown,
            DirectionalNavigationAction::Left => GamepadButton::DPadLeft,
            DirectionalNavigationAction::Right => GamepadButton::DPadRight,
            DirectionalNavigationAction::Select => GamepadButton::South,
        }
    }
}

#[derive(Default, Resource)]
struct ActionState {
    pressed_actions: HashSet<DirectionalNavigationAction>,
}

fn process_inputs(
    mut action_state: ResMut<ActionState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    gamepad_input: Query<&Gamepad>,
) {
    action_state.pressed_actions.clear();

    for action in DirectionalNavigationAction::variants() {
        if keyboard_input.just_pressed(action.keycode()) {
            action_state.pressed_actions.insert(action);
        }
    }

    for gamepad in gamepad_input.iter() {
        for action in DirectionalNavigationAction::variants() {
            if gamepad.just_pressed(action.gamepad_button()) {
                action_state.pressed_actions.insert(action);
            }
        }
    }
}

fn navigate(action_state: Res<ActionState>, mut directional_navigation: DirectionalNavigation) {
    let net_east_west = action_state
        .pressed_actions
        .contains(&DirectionalNavigationAction::Right) as i8
        - action_state
            .pressed_actions
            .contains(&DirectionalNavigationAction::Left) as i8;

    let net_north_south = action_state
        .pressed_actions
        .contains(&DirectionalNavigationAction::Up) as i8
        - action_state
            .pressed_actions
            .contains(&DirectionalNavigationAction::Down) as i8;

    // Use Dir2::from_xy to convert input to direction, then convert to CompassOctant
    let maybe_direction = Dir2::from_xy(net_east_west as f32, net_north_south as f32)
        .ok()
        .map(CompassOctant::from);

    if let Some(direction) = maybe_direction {
        match directional_navigation.navigate(direction) {
            Ok(_entity) => {
                // Successfully navigated
            }
            Err(_e) => {
                // Navigation failed (no neighbor in that direction)
            }
        }
    }
}

fn update_focus_display(
    input_focus: Res<InputFocus>,
    button_query: Query<&Name, With<Button>>,
    mut display_query: Query<&mut Text, With<FocusDisplay>>,
) {
    if let Ok(mut text) = display_query.single_mut() {
        if let Some(focused_entity) = input_focus.0 {
            if let Ok(name) = button_query.get(focused_entity) {
                **text = format!("Focused: {}", name);
            } else {
                **text = "Focused: Unknown".to_string();
            }
        } else {
            **text = "Focused: None".to_string();
        }
    }
}

fn update_key_display(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    gamepad_input: Query<&Gamepad>,
    mut display_query: Query<&mut Text, With<KeyDisplay>>,
) {
    if let Ok(mut text) = display_query.single_mut() {
        // Check for keyboard inputs
        for action in DirectionalNavigationAction::variants() {
            if keyboard_input.just_pressed(action.keycode()) {
                let key_name = match action {
                    DirectionalNavigationAction::Up => "Up Arrow",
                    DirectionalNavigationAction::Down => "Down Arrow",
                    DirectionalNavigationAction::Left => "Left Arrow",
                    DirectionalNavigationAction::Right => "Right Arrow",
                    DirectionalNavigationAction::Select => "Enter",
                };
                **text = format!("Last Key: {}", key_name);
                return;
            }
        }

        // Check for gamepad inputs
        for gamepad in gamepad_input.iter() {
            for action in DirectionalNavigationAction::variants() {
                if gamepad.just_pressed(action.gamepad_button()) {
                    let button_name = match action {
                        DirectionalNavigationAction::Up => "D-Pad Up",
                        DirectionalNavigationAction::Down => "D-Pad Down",
                        DirectionalNavigationAction::Left => "D-Pad Left",
                        DirectionalNavigationAction::Right => "D-Pad Right",
                        DirectionalNavigationAction::Select => "A Button",
                    };
                    **text = format!("Last Key: {}", button_name);
                    return;
                }
            }
        }
    }
}

fn highlight_focused_element(
    input_focus: Res<InputFocus>,
    input_focus_visible: Res<InputFocusVisible>,
    mut query: Query<(Entity, &mut BorderColor)>,
) {
    for (entity, mut border_color) in query.iter_mut() {
        if input_focus.0 == Some(entity) && input_focus_visible.0 {
            *border_color = BorderColor::all(FOCUSED_BORDER);
        } else {
            *border_color = BorderColor::DEFAULT;
        }
    }
}

fn interact_with_focused_button(
    action_state: Res<ActionState>,
    input_focus: Res<InputFocus>,
    mut commands: Commands,
) {
    if action_state
        .pressed_actions
        .contains(&DirectionalNavigationAction::Select)
        && let Some(focused_entity) = input_focus.0
    {
        commands.trigger(Pointer::<Click> {
            entity: focused_entity,
            pointer_id: PointerId::Mouse,
            pointer_location: Location {
                target: NormalizedRenderTarget::None {
                    width: 0,
                    height: 0,
                },
                position: Vec2::ZERO,
            },
            event: Click {
                button: PointerButton::Primary,
                hit: HitData {
                    camera: Entity::PLACEHOLDER,
                    depth: 0.0,
                    position: None,
                    normal: None,
                },
                duration: Duration::from_secs_f32(0.1),
            },
        });
    }
}
