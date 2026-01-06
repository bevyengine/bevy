//! Demonstrates a combination of automatic directional navigation and manual directional navigation.
//!
//! This example shows how to leverage both automatic navigation and manual navigation to create
//! a desired user navigation experience without much boilerplate code. The `AutoDirectionalNavigation`
//! component is used to add basic, intuitive navigation to UI elements. Manual edges between certain
//! UI elements are added to the `DirectionalNavigationMap` to override auto navigation and to add
//! special navigation rules.
//!
//! The directional navigation system provided by `AutoDirectionalNavigator` uses manually defined edges
//! first. If no manual edge is defined, automatic navigation is used.

use core::time::Duration;

use bevy::{
    camera::NormalizedRenderTarget,
    input_focus::{
        directional_navigation::{
            AutoNavigationConfig, DirectionalNavigationMap, DirectionalNavigationPlugin,
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
    ui::auto_directional_navigation::{AutoDirectionalNavigation, AutoDirectionalNavigator},
};

fn main() {
    App::new()
        // Input focus is not enabled by default, so we need to add the corresponding plugins
        // The navigation system's resources are initialized by the DirectionalNavigationPlugin.
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
            // Do not prefer nodes that are well-aligned. In a cascading layout, nodes may not be well-aligned.
            prefer_aligned: false,
        })
        .init_resource::<ActionState>()
        // For automatic navigation, UI entities will have the component `AutoDirectionalNavigation`
        // and will be automatically connected by the navigation system.
        // To override some automatically created navigation edges, manual edges are created
        // in `setup_cascading_ui`. We will also add some new edges that the automatic navigation system
        // does not create.
        .add_systems(Startup, setup_cascading_ui)
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

/// Spawn a cascading layout of buttons to demonstrate automatic and manual navigation.
///
/// This will create a grid of buttons, but each row cascades from the top left to the bottom right.
/// Manual navigation will connect the end of one row with the beginning of the next row.
/// Manual navigation will also make vertical navigation inverted (pressing down moves up)
fn setup_cascading_ui(
    mut commands: Commands,
    mut manual_directional_nav_map: ResMut<DirectionalNavigationMap>,
    mut input_focus: ResMut<InputFocus>,
) {
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
                "Navigation Demo\n\n\
                 Use arrow keys or D-pad to navigate.\n\
                 Press Enter or A button to interact.\n\n\
                 Buttons are scattered in cascading rows:\n\
                 Horizontal navigation within rows is automatic.\n\
                 Horizontal navigation between rows is manual.\n\
                 Vertical navigation between rows is inverted and manual.",
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

    // Spawn buttons in cascading rows
    // Auto-navigation will configure navigation within rows.
    let button_positions = [
        // Row 0
        [(350.0, 80.0), (550.0, 125.0), (750.0, 170.0)],
        // Row 1
        [(350.0, 215.0), (550.0, 260.0), (750.0, 305.0)],
        // Row 2
        [(350.0, 350.0), (550.0, 395.0), (750.0, 440.0)],
        // Row 3
        [(350.0, 485.0), (550.0, 530.0), (750.0, 575.0)],
    ];

    let mut first_button = None;
    let mut entities = Vec::with_capacity(4);
    for (i, row) in button_positions.iter().enumerate() {
        for (j, (x, y)) in row.iter().enumerate() {
            let button_entity = commands
                .spawn((
                    Button,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(*x),
                        top: px(*y),
                        width: px(140),
                        height: px(90),
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
                    Name::new(format!("Row {}, Button {}", i + 1, j + 1)),
                ))
                .with_child((
                    Text::new(format!("Row {}, Button {}", i + 1, j + 1)),
                    TextLayout {
                        justify: Justify::Center,
                        ..default()
                    },
                ))
                .id();

            if first_button.is_none() {
                first_button = Some(button_entity);
            }
            entities.push(button_entity);
        }
    }

    // Add manual edges for inverted vertical navigation
    // These manual edges override any automatic navigation vertically
    let mut col = Vec::with_capacity(4);
    for col_index in 0..=2 {
        for (i, &entity) in entities.iter().enumerate() {
            if i % 3 == col_index {
                col.push(entity);
            }
        }
        // edges are connected in the opposite vertical direction
        manual_directional_nav_map.add_looping_edges(&col, CompassOctant::North);
        col.clear();
    }

    // Add manual edges for navigation between rows
    // These manual edges do not override any automatic navigation
    let entity_pairs = [
        // the end of the first row should connect to the beginning of the second
        ((0, 2), (1, 0)),
        // the end of the second row should connect to the beginning of the third
        ((1, 2), (2, 0)),
        // the end of the third row should connect to the beginning of the fourth
        ((2, 2), (3, 0)),
        // the end of the fourth row should connect to the beginning of the first (the end wraps to the beginning)
        ((3, 2), (0, 0)),
    ];
    for ((entity_a_row, entity_a_col), (entity_b_row, entity_b_col)) in entity_pairs.iter() {
        manual_directional_nav_map.add_symmetrical_edge(
            entities[entity_a_row * 3 + entity_a_col],
            entities[entity_b_row * 3 + entity_b_col],
            CompassOctant::East,
        );
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

fn navigate(
    action_state: Res<ActionState>,
    mut auto_directional_navigator: AutoDirectionalNavigator,
) {
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
        match auto_directional_navigator.navigate(direction) {
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
