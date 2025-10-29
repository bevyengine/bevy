//! Demonstrates how to set up the directional navigation system to allow for navigation between widgets.
//!
//! Directional navigation is generally used to move between widgets in a user interface using arrow keys or gamepad input.
//! When compared to tab navigation, directional navigation is generally more direct, and less aware of the structure of the UI.
//!
//! In this example, we will set up a simple UI with a grid of buttons that can be navigated using the arrow keys or gamepad input.

use std::time::Duration;

use bevy::{
    camera::NormalizedRenderTarget,
    input_focus::{
        directional_navigation::{
            DirectionalNavigation, DirectionalNavigationMap, DirectionalNavigationPlugin,
        },
        InputDispatchPlugin, InputFocus, InputFocusVisible,
    },
    math::CompassOctant,
    picking::{
        backend::HitData,
        pointer::{Location, PointerId},
    },
    platform::collections::{HashMap, HashSet},
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
        // We've made a simple resource to keep track of the actions that are currently being pressed for this example
        .init_resource::<ActionState>()
        .add_systems(Startup, setup_ui)
        // Input is generally handled during PreUpdate
        // We're turning inputs into actions first, then using those actions to determine navigation
        .add_systems(PreUpdate, (process_inputs, navigate).chain())
        .add_systems(
            Update,
            (
                // We need to show which button is currently focused
                highlight_focused_element,
                // Pressing the "Interact" button while we have a focused element should simulate a click
                interact_with_focused_button,
                // We're doing a tiny animation when the button is interacted with,
                // so we need a timer and a polling mechanism to reset it
                reset_button_after_interaction,
            ),
        )
        // This observer is added globally, so it will respond to *any* trigger of the correct type.
        // However, we're filtering in the observer's query to only respond to button presses
        .add_observer(universal_button_click_behavior)
        .run();
}

const NORMAL_BUTTON: Srgba = bevy::color::palettes::tailwind::BLUE_400;
const PRESSED_BUTTON: Srgba = bevy::color::palettes::tailwind::BLUE_500;
const FOCUSED_BORDER: Srgba = bevy::color::palettes::tailwind::BLUE_50;

// This observer will be triggered whenever a button is pressed
// In a real project, each button would also have its own unique behavior,
// to capture the actual intent of the user
fn universal_button_click_behavior(
    mut click: On<Pointer<Click>>,
    mut button_query: Query<(&mut BackgroundColor, &mut ResetTimer)>,
) {
    let button_entity = click.entity;
    if let Ok((mut color, mut reset_timer)) = button_query.get_mut(button_entity) {
        // This would be a great place to play a little sound effect too!
        color.0 = PRESSED_BUTTON.into();
        reset_timer.0 = Timer::from_seconds(0.3, TimerMode::Once);

        // Picking events propagate up the hierarchy,
        // so we need to stop the propagation here now that we've handled it
        click.propagate(false);
    }
}

/// Resets a UI element to its default state when the timer has elapsed.
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

// We're spawning a simple grid of buttons and some instructions
// The buttons are just colored rectangles with text displaying the button's name
fn setup_ui(
    mut commands: Commands,
    mut directional_nav_map: ResMut<DirectionalNavigationMap>,
    mut input_focus: ResMut<InputFocus>,
) {
    const N_ROWS: u16 = 5;
    const N_COLS: u16 = 3;

    // Rendering UI elements requires a camera
    commands.spawn(Camera2d);

    // Create a full-screen background node
    let root_node = commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            ..default()
        })
        .id();

    // Add instruction to the left of the grid
    let instructions = commands
        .spawn((
            Text::new("Use arrow keys or D-pad to navigate. \
            Click the buttons, or press Enter / the South gamepad button to interact with the focused button."),
            Node {
                width: px(300),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(px(12)),
                ..default()
            },
        ))
        .id();

    // Set up the root entity to hold the grid
    let grid_root_entity = commands
        .spawn(Node {
            display: Display::Grid,
            // Allow the grid to take up the full height and the rest of the width of the window
            width: percent(100),
            height: percent(100),
            // Set the number of rows and columns in the grid
            // allowing the grid to automatically size the cells
            grid_template_columns: RepeatedGridTrack::auto(N_COLS),
            grid_template_rows: RepeatedGridTrack::auto(N_ROWS),
            ..default()
        })
        .id();

    // Add the instructions and grid to the root node
    commands
        .entity(root_node)
        .add_children(&[instructions, grid_root_entity]);

    let mut button_entities: HashMap<(u16, u16), Entity> = HashMap::default();
    for row in 0..N_ROWS {
        for col in 0..N_COLS {
            let button_name = format!("Button {row}-{col}");

            let button_entity = commands
                .spawn((
                    Button,
                    Node {
                        width: px(200),
                        height: px(120),
                        // Add a border so we can show which element is focused
                        border: UiRect::all(px(4)),
                        // Center the button's text label
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        // Center the button within the grid cell
                        align_self: AlignSelf::Center,
                        justify_self: JustifySelf::Center,
                        ..default()
                    },
                    ResetTimer::default(),
                    BorderRadius::all(px(16)),
                    BackgroundColor::from(NORMAL_BUTTON),
                    Name::new(button_name.clone()),
                ))
                // Add a text element to the button
                .with_child((
                    Text::new(button_name),
                    // And center the text if it flows onto multiple lines
                    TextLayout {
                        justify: Justify::Center,
                        ..default()
                    },
                ))
                .id();

            // Add the button to the grid
            commands.entity(grid_root_entity).add_child(button_entity);

            // Keep track of the button entities so we can set up our navigation graph
            button_entities.insert((row, col), button_entity);
        }
    }

    // Connect all of the buttons in the same row to each other,
    // looping around when the edge is reached.
    for row in 0..N_ROWS {
        let entities_in_row: Vec<Entity> = (0..N_COLS)
            .map(|col| button_entities.get(&(row, col)).unwrap())
            .copied()
            .collect();
        directional_nav_map.add_looping_edges(&entities_in_row, CompassOctant::East);
    }

    // Connect all of the buttons in the same column to each other,
    // but don't loop around when the edge is reached.
    // While looping is a very reasonable choice, we're not doing it here to demonstrate the different options.
    for col in 0..N_COLS {
        let entities_in_column: Vec<Entity> = (0..N_ROWS)
            .map(|row| button_entities.get(&(row, col)).unwrap())
            .copied()
            .collect();

        directional_nav_map.add_edges(&entities_in_column, CompassOctant::South);
    }

    // When changing scenes, remember to set an initial focus!
    let top_left_entity = *button_entities.get(&(0, 0)).unwrap();
    input_focus.set(top_left_entity);
}

// The indirection between inputs and actions allows us to easily remap inputs
// and handle multiple input sources (keyboard, gamepad, etc.) in our game
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
            // This is the "A" button on an Xbox controller,
            // and is conventionally used as the "Select" / "Interact" button in many games
            DirectionalNavigationAction::Select => GamepadButton::South,
        }
    }
}

// This keeps track of the inputs that are currently being pressed
#[derive(Default, Resource)]
struct ActionState {
    pressed_actions: HashSet<DirectionalNavigationAction>,
}

fn process_inputs(
    mut action_state: ResMut<ActionState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    gamepad_input: Query<&Gamepad>,
) {
    // Reset the set of pressed actions each frame
    // to ensure that we only process each action once
    action_state.pressed_actions.clear();

    for action in DirectionalNavigationAction::variants() {
        // Use just_pressed to ensure that we only process each action once
        // for each time it is pressed
        if keyboard_input.just_pressed(action.keycode()) {
            action_state.pressed_actions.insert(action);
        }
    }

    // We're treating this like a single-player game:
    // if multiple gamepads are connected, we don't care which one is being used
    for gamepad in gamepad_input.iter() {
        for action in DirectionalNavigationAction::variants() {
            // Unlike keyboard input, gamepads are bound to a specific controller
            if gamepad.just_pressed(action.gamepad_button()) {
                action_state.pressed_actions.insert(action);
            }
        }
    }
}

fn navigate(action_state: Res<ActionState>, mut directional_navigation: DirectionalNavigation) {
    // If the user is pressing both left and right, or up and down,
    // we should not move in either direction.
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

    // Compute the direction that the user is trying to navigate in
    let maybe_direction = match (net_east_west, net_north_south) {
        (0, 0) => None,
        (0, 1) => Some(CompassOctant::North),
        (1, 1) => Some(CompassOctant::NorthEast),
        (1, 0) => Some(CompassOctant::East),
        (1, -1) => Some(CompassOctant::SouthEast),
        (0, -1) => Some(CompassOctant::South),
        (-1, -1) => Some(CompassOctant::SouthWest),
        (-1, 0) => Some(CompassOctant::West),
        (-1, 1) => Some(CompassOctant::NorthWest),
        _ => None,
    };

    if let Some(direction) = maybe_direction {
        match directional_navigation.navigate(direction) {
            // In a real game, you would likely want to play a sound or show a visual effect
            // on both successful and unsuccessful navigation attempts
            Ok(entity) => {
                println!("Navigated {direction:?} successfully. {entity} is now focused.");
            }
            Err(e) => println!("Navigation failed: {e}"),
        }
    }
}

fn highlight_focused_element(
    input_focus: Res<InputFocus>,
    // While this isn't strictly needed for the example,
    // we're demonstrating how to be a good citizen by respecting the `InputFocusVisible` resource.
    input_focus_visible: Res<InputFocusVisible>,
    mut query: Query<(Entity, &mut BorderColor)>,
) {
    for (entity, mut border_color) in query.iter_mut() {
        if input_focus.0 == Some(entity) && input_focus_visible.0 {
            // Don't change the border size / radius here,
            // as it would result in wiggling buttons when they are focused
            *border_color = BorderColor::all(FOCUSED_BORDER);
        } else {
            *border_color = BorderColor::DEFAULT;
        }
    }
}

// By sending a Pointer<Click> trigger rather than directly handling button-like interactions,
// we can unify our handling of pointer and keyboard/gamepad interactions
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
            // We're pretending that we're a mouse
            pointer_id: PointerId::Mouse,
            // This field isn't used, so we're just setting it to a placeholder value
            pointer_location: Location {
                target: NormalizedRenderTarget::None {
                    width: 0,
                    height: 0,
                },
                position: Vec2::ZERO,
            },
            event: Click {
                button: PointerButton::Primary,
                // This field isn't used, so we're just setting it to a placeholder value
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
