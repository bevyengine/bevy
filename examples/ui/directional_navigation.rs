//! Demonstrates how to set up the directional navigation system to allow for navigation between widgets.
//!
//! Directional navigation is generally used to move between widgets in a user interface using arrow keys or gamepad input.
//! When compared to tab navigation, directional navigation is generally more direct, and less aware of the structure of the UI.
//!
//! In this example, we will set up a simple UI with a grid of buttons that can be navigated using the arrow keys or gamepad input.

use bevy::{
    input_focus::{
        directional_navigation::{
            DirectionalNavigation, DirectionalNavigationMap, DirectionalNavigationPlugin,
        },
        InputDispatchPlugin, InputFocus, InputFocusVisible,
    },
    math::CompassOctant,
    prelude::*,
    utils::{HashMap, HashSet},
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
                // We should use the focused element to determine what "Enter" / "A" does
                // And then respond to that action
                (interact_with_focused_button, say_button_name_on_interaction).chain(),
            ),
        )
        .run();
}

// We're spawning a simple 3x3 grid of buttons
// The buttons are just colored rectangles with text displaying the button's name
fn setup_ui(
    mut commands: Commands,
    mut directional_nav_map: ResMut<DirectionalNavigationMap>,
    mut input_focus: ResMut<InputFocus>,
) {
    const N_ROWS: u16 = 3;
    const N_COLS: u16 = 3;

    // Rendering UI elements requires a camera
    commands.spawn(Camera2d::default());

    // Set up the root entity to hold the grid
    let root_entity = commands
        .spawn(Node {
            display: Display::Grid,
            // Allow the grid to take up the full height and width of the window
            width: Val::Vw(100.),
            height: Val::Vh(100.),
            // Set the number of rows and columns in the grid
            // allowing the grid to automatically size the cells
            grid_template_columns: RepeatedGridTrack::auto(N_COLS),
            grid_template_rows: RepeatedGridTrack::auto(N_ROWS),
            ..default()
        })
        .id();

    let mut button_entities: HashMap<(u16, u16), Entity> = HashMap::default();

    for row in 0..N_ROWS {
        for col in 0..N_COLS {
            let button_name = format!("Button {}-{}", row, col);

            let button_entity = commands
                .spawn((
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(120.0),
                        // Add a border so we can show which element is focused
                        border: UiRect::all(Val::Px(4.0)),
                        // Center the button's text label
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        // Center the button within the grid cell
                        align_self: AlignSelf::Center,
                        justify_self: JustifySelf::Center,
                        ..default()
                    },
                    BorderRadius::all(Val::Px(16.0)),
                    BackgroundColor::from(bevy::color::palettes::tailwind::BLUE_300),
                    Name::new(button_name.clone()),
                ))
                // Add a text element to the button
                .with_child((
                    Text::new(button_name),
                    // And center the text within the label
                    TextLayout {
                        justify: JustifyText::Center,
                        ..default()
                    },
                ))
                .id();

            // Add the button to the grid
            commands.entity(root_entity).add_child(button_entity);

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
        // Don't iterate over the last row, as no lower row exists to connect to
        for row in 0..N_ROWS - 1 {
            let upper_entity = button_entities.get(&(row, col)).unwrap();
            let lower_entity = button_entities.get(&(row + 1, col)).unwrap();
            directional_nav_map.add_symmetrical_edge(
                *upper_entity,
                *lower_entity,
                CompassOctant::South,
            );
        }
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
            border_color.0 = Color::WHITE;
        } else {
            border_color.0 = Color::NONE;
        }
    }
}

// By modifying the [`Interaction`] component rather than directly handling button-like interactions,
// we can unify our handling of pointer and buttonlike interactions
fn interact_with_focused_button(
    action_state: Res<ActionState>,
    input_focus: Res<InputFocus>,
    mut query: Query<&mut Interaction>,
) {
    if action_state
        .pressed_actions
        .contains(&DirectionalNavigationAction::Select)
    {
        if let Some(focused_entity) = input_focus.0 {
            if let Ok(mut interaction) = query.get_mut(focused_entity) {
                // `Interaction::Pressed` is also set whenever the user clicks on a button
                // and `Interaction` is reset at the start of each frame
                *interaction = Interaction::Pressed;
            }
        }
    }
}

// This system will print the name of the button that was interacted with,
// regardless of whether the interaction was a click or an interaction with a focused button.
//
// Obviously, the actual behavior should be specialized for each button in a real game.
//
// We're filtering for `Changed<Interaction>` to only run this system when the interaction changes,
// to avoid spamming the console with the same message when the button is held down
fn say_button_name_on_interaction(query: Query<(&Interaction, &Name), Changed<Interaction>>) {
    for (interaction, button_name) in query.iter() {
        if *interaction == Interaction::Pressed {
            println!("Button clicked: {button_name}");
        }
    }
}
