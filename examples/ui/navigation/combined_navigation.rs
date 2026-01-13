//! Demonstrates a combination of automatic and manual directional navigation.
//!
//! This example shows how to leverage both automatic navigation and manual navigation to create
//! a desired user navigation experience without much boilerplate code. In this example, there are
//! multiple pages of UI Buttons that depict different scenarios in which both automatic and manual
//! navigation are leveraged to produce a desired navigation experience.
//!
//! Manual navigation can also be used to define navigation in any situation where automatic
//! navigation fails to create an edge due to lack of proximity. For example, when creating
//! navigation that loops around to an opposite side, manual navigation should be used to define
//! this behavior. If one input is too far away from the others and `AutoNavigationConfig`
//! cannot be tweaked, manual navigation can connect that input to the others. Manual navigation
//! can also be used to override any undesired navigation.
//!
//! The `AutoDirectionalNavigation` component is used to create basic, intuitive navigation to UI
//! elements within a page. Manual navigation edges are added to the `DirectionalNavigationMap`
//! to create special navigation rules. The `AutoDirectionalNavigator` system parameter navigates
//! using manual navigation rules first and automatic navigation second.

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
            // Don't connect nodes more than 200 pixels apart
            max_search_distance: Some(200.0),
            // Prefer nodes that are well-aligned
            prefer_aligned: true,
        })
        .init_resource::<ActionState>()
        // For automatic navigation, UI entities will have the component `AutoDirectionalNavigation`
        // and will be automatically connected by the navigation system.
        // We will also add some new edges that the automatic navigation system
        // cannot create by itself by inserting them into `DirectionalNavigationMap`
        .add_systems(Startup, setup_paged_ui)
        // Input is generally handled during PreUpdate
        .add_systems(PreUpdate, (process_inputs, navigate).chain())
        .add_systems(
            Update,
            (
                highlight_focused_element,
                interact_with_focused_button,
                reset_button_after_interaction,
                update_focus_display
                    .run_if(|input_focus: Res<InputFocus>| input_focus.is_changed()),
                update_key_display,
            ),
        )
        .add_observer(universal_button_click_behavior)
        .run();
}

const PAGE_1_NORMAL_BUTTON: Srgba = bevy::color::palettes::tailwind::BLUE_400;
const PAGE_1_PRESSED_BUTTON: Srgba = bevy::color::palettes::tailwind::BLUE_500;
const PAGE_1_FOCUSED_BORDER: Srgba = bevy::color::palettes::tailwind::BLUE_50;

const PAGE_2_NORMAL_BUTTON: Srgba = bevy::color::palettes::tailwind::RED_400;
const PAGE_2_PRESSED_BUTTON: Srgba = bevy::color::palettes::tailwind::RED_500;
const PAGE_2_FOCUSED_BORDER: Srgba = bevy::color::palettes::tailwind::RED_50;

const PAGE_3_NORMAL_BUTTON: Srgba = bevy::color::palettes::tailwind::GREEN_400;
const PAGE_3_PRESSED_BUTTON: Srgba = bevy::color::palettes::tailwind::GREEN_500;
const PAGE_3_FOCUSED_BORDER: Srgba = bevy::color::palettes::tailwind::GREEN_50;

const NORMAL_BUTTON_COLORS: [Srgba; 3] = [
    PAGE_1_NORMAL_BUTTON,
    PAGE_2_NORMAL_BUTTON,
    PAGE_3_NORMAL_BUTTON,
];
const PRESSED_BUTTON_COLORS: [Srgba; 3] = [
    PAGE_1_PRESSED_BUTTON,
    PAGE_2_PRESSED_BUTTON,
    PAGE_3_PRESSED_BUTTON,
];
const FOCUSED_BORDER_COLORS: [Srgba; 3] = [
    PAGE_1_FOCUSED_BORDER,
    PAGE_2_FOCUSED_BORDER,
    PAGE_3_FOCUSED_BORDER,
];

/// Marker component for the text that displays the currently focused button
#[derive(Component)]
struct FocusDisplay;

/// Marker component for the text that displays the last key pressed
#[derive(Component)]
struct KeyDisplay;

/// Component that stores which page a button is on
#[derive(Component)]
struct Page(usize);

// Observer for button clicks
fn universal_button_click_behavior(
    mut click: On<Pointer<Click>>,
    mut button_query: Query<(&mut BackgroundColor, &Page, &mut ResetTimer)>,
) {
    let button_entity = click.entity;
    if let Ok((mut color, page, mut reset_timer)) = button_query.get_mut(button_entity) {
        color.0 = PRESSED_BUTTON_COLORS[page.0].into();
        reset_timer.0 = Timer::from_seconds(0.3, TimerMode::Once);
        click.propagate(false);
    }
}

#[derive(Component, Default, Deref, DerefMut)]
struct ResetTimer(Timer);

fn reset_button_after_interaction(
    time: Res<Time>,
    mut query: Query<(&mut ResetTimer, &mut BackgroundColor, &Page)>,
) {
    for (mut reset_timer, mut color, page) in query.iter_mut() {
        reset_timer.tick(time.delta());
        if reset_timer.just_finished() {
            color.0 = NORMAL_BUTTON_COLORS[page.0].into();
        }
    }
}

/// Spawn pages of buttons to demonstrate automatic and manual navigation.
///
/// This function creates three pages of buttons. All buttons have automatic navigation.
/// Manual navigation is specified with the DirectionalNavigationMap.
/// Page 1 has a simple grid of buttons where transitions between rows is defined using
/// the DirectionalNavigationMap.
/// Page 2 has a cluster of buttons to the top left and a lonely button on the bottom right.
/// Navigation between the cluster and the lonely button is defined using the
/// DirectionalNavigationMap.
/// Page 3 has the same simple grid of buttons as page 1, but automatic navigation has been
/// overridden in the vertical direction with the DirectionalNavigationMap.
fn setup_paged_ui(
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
                "Combined Navigation Demo\n\n\
                 Use arrow keys or D-pad to navigate.\n\
                 Press Enter or A button to interact.\n\n\
                 Navigation on each page is a combination of \
                 both automatic and manual navigation.",
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

    let mut pages_entities = [
        Vec::with_capacity(12),
        Vec::with_capacity(12),
        Vec::with_capacity(12),
    ];
    let mut text_entities = Vec::with_capacity(6);
    for (page_num, page_button_entities) in pages_entities.iter_mut().enumerate() {
        if page_num == 1 {
            // the second page
            setup_buttons_for_triangle_page(
                &mut commands,
                page_num,
                (page_button_entities, &mut text_entities),
            )
        } else {
            // the first and third pages are regular grids
            setup_buttons_for_grid_page(
                &mut commands,
                page_num,
                (page_button_entities, &mut text_entities),
            );
        }

        // Only the first page is visible at setup.
        let visibility = if page_num == 0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let page = commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                visibility,
            ))
            .id();

        commands
            .entity(page)
            .add_children(page_button_entities)
            .add_children(&text_entities);

        text_entities.clear();
    }
    let first_button = Some(pages_entities[0][0]);

    // For Pages 1 and 3, add manual edges within the grid page for navigation between rows.
    let entity_pairs = [
        // the end of the first row should connect to the beginning of the second
        ((0, 2), (1, 0)),
        // the end of the second row should connect to the beginning of the third
        ((1, 2), (2, 0)),
        // the end of the third row should connect to the beginning of the fourth
        ((2, 2), (3, 0)),
    ];
    for (page_num, page_entities) in pages_entities.iter().enumerate() {
        // Skip Page 2; we are only adding these manual edges for the grid pages.
        if page_num == 1 {
            continue;
        }
        for ((entity_a_row, entity_a_col), (entity_b_row, entity_b_col)) in entity_pairs.iter() {
            manual_directional_nav_map.add_symmetrical_edge(
                page_entities[entity_a_row * 3 + entity_a_col],
                page_entities[entity_b_row * 3 + entity_b_col],
                CompassOctant::East,
            );
        }
    }

    // Add manual edges within the triangle page (Page 2) between buttons 3 and 4.
    // The `AutoNavigationConfig` is set to our desired values, but automatic
    // navigation does not connect Button 3 to Button 4, so we have to add
    // this navigation manually.
    manual_directional_nav_map.add_symmetrical_edge(
        pages_entities[1][2],
        pages_entities[1][3],
        CompassOctant::East,
    );
    manual_directional_nav_map.add_symmetrical_edge(
        pages_entities[1][2],
        pages_entities[1][3],
        CompassOctant::South,
    );
    manual_directional_nav_map.add_symmetrical_edge(
        pages_entities[1][2],
        pages_entities[1][3],
        CompassOctant::SouthEast,
    );

    // For Page 3, we override the navigation North and South to be inverted.
    let mut col_entities = Vec::with_capacity(4);
    for col in 0..=2 {
        for row in 0..=3 {
            col_entities.push(pages_entities[2][row * 3 + col])
        }
        manual_directional_nav_map.add_looping_edges(&col_entities, CompassOctant::North);
        col_entities.clear();
    }

    // Add manual edges between pages.
    // When navigating east (right) from the last button of page 1,
    // go to the first button of page 2. This edge is symmetrical.
    manual_directional_nav_map.add_symmetrical_edge(
        pages_entities[0][11],
        pages_entities[1][0],
        CompassOctant::East,
    );
    // When navigating south (down) from the last button of page 2,
    // go to the first button of page 3. This edge is NOT symmetrical.
    // This means going north (up) from the first button of page 3 does
    // NOT go to the last button of page 2.
    manual_directional_nav_map.add_edge(
        pages_entities[1][3],
        pages_entities[2][0],
        CompassOctant::South,
    );
    // When navigating west (left) from the first button of page 3,
    // go back to the last button of page 2. This edge is NOT symmetrical.
    manual_directional_nav_map.add_edge(
        pages_entities[2][0],
        pages_entities[1][3],
        CompassOctant::West,
    );
    // When navigating east (right) from the last button of page 1,
    // go to the first button of page 2. This edge is symmetrical.
    manual_directional_nav_map.add_symmetrical_edge(
        pages_entities[2][11],
        pages_entities[0][0],
        CompassOctant::East,
    );

    commands.entity(root_node).add_children(&[instructions]);

    // Set initial focus
    if let Some(button) = first_button {
        input_focus.set(button);
    }
}

/// Creates the buttons and text for a grid page and places the ids into their
/// respective Vecs in `entities`.
fn setup_buttons_for_grid_page(
    commands: &mut Commands,
    page_num: usize,
    entities: (&mut Vec<Entity>, &mut Vec<Entity>),
) {
    let (page_button_entities, text_entities) = entities;

    // Spawn buttons in a grid
    // Auto-navigation will automatically configure navigation within rows.
    let button_positions = [
        // Row 0
        [(450.0, 80.0), (650.0, 80.0), (850.0, 80.0)],
        // Row 1
        [(450.0, 215.0), (650.0, 215.0), (850.0, 215.0)],
        // Row 2
        [(450.0, 350.0), (650.0, 350.0), (850.0, 350.0)],
        // Row 3
        [(450.0, 485.0), (650.0, 485.0), (850.0, 485.0)],
    ];
    for (i, row) in button_positions.iter().enumerate() {
        for (j, (left, top)) in row.iter().enumerate() {
            let button_entity = spawn_auto_nav_button(
                commands,
                format!("Btn {}-{}", i + 1, j + 1),
                left,
                top,
                page_num,
            );
            page_button_entities.push(button_entity);
        }
    }

    // Text describing current page
    let current_page_entity = spawn_small_text_node(
        commands,
        format!("Currently on Page {}", page_num + 1),
        650,
        20,
        Justify::Center,
    );
    text_entities.push(current_page_entity);

    // Text describing direction to go to the previous page, placed left of the top-left button.
    let previous_page = if page_num == 0 { 3 } else { page_num };
    let previous_page_entity = spawn_small_text_node(
        commands,
        format!("Page {} << ", previous_page),
        310,
        120,
        Justify::Right,
    );
    text_entities.push(previous_page_entity);

    // Text describing direction to go to the next page, placed right of the bottom-right button.
    let next_page_entity = spawn_small_text_node(
        commands,
        format!(">> Page {}", (page_num + 1) % 3 + 1),
        1000,
        525,
        Justify::Left,
    );
    text_entities.push(next_page_entity);

    // Texts describing that moving right wraps to the next row.
    let right_1 = spawn_small_text_node(commands, "> Btn 2-1".into(), 1000, 120, Justify::Left);
    let right_2 = spawn_small_text_node(commands, "> Btn 3-1".into(), 1000, 255, Justify::Left);
    let right_3 = spawn_small_text_node(commands, "> Btn 4-1".into(), 1000, 390, Justify::Left);
    let left_1 = spawn_small_text_node(commands, "Btn 1-3 < ".into(), 310, 255, Justify::Right);
    let left_2 = spawn_small_text_node(commands, "Btn 2-3 < ".into(), 310, 390, Justify::Right);
    let left_3 = spawn_small_text_node(commands, "Btn 3-3 < ".into(), 310, 525, Justify::Right);
    text_entities.push(right_1);
    text_entities.push(right_2);
    text_entities.push(right_3);
    text_entities.push(left_1);
    text_entities.push(left_2);
    text_entities.push(left_3);

    // For the third page, add a notice about vertical navigation being inverted in the grid.
    if page_num == 2 {
        let footer_info = commands
            .spawn((
                Text::new(
                    "Vertical Navigation has been manually overridden to be inverted! \
                ^ moves down, and v (down) moves up.",
                ),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(450),
                    top: px(600),
                    width: px(540),
                    padding: UiRect::all(px(12)),
                    ..default()
                },
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
            ))
            .id();
        text_entities.push(footer_info);
    }
}

/// Creates the buttons and text for a the "triangle" page and places the ids into their
/// respective Vecs in `entities`.
fn setup_buttons_for_triangle_page(
    commands: &mut Commands,
    page_num: usize,
    entities: (&mut Vec<Entity>, &mut Vec<Entity>),
) {
    let button_positions = [
        (450.0, 80.0),   // top left
        (700.0, 80.0),   // top right
        (575.0, 215.0),  // middle
        (1050.0, 350.0), // bottom right
    ];
    let (page_button_entities, text_entities) = entities;
    for (i, (left, top)) in button_positions.iter().enumerate() {
        let button_entity =
            spawn_auto_nav_button(commands, format!("Btn {}", i + 1), left, top, page_num);
        page_button_entities.push(button_entity);
    }

    // Text describing current page
    let current_page_entity = spawn_small_text_node(
        commands,
        format!("Page {}", page_num + 1),
        650,
        20,
        Justify::Center,
    );
    text_entities.push(current_page_entity);

    // Text describing direction to go to the previous page, placed left of the top-left button.
    let previous_page = if page_num == 0 { 3 } else { page_num };
    let previous_page_entity = spawn_small_text_node(
        commands,
        format!("Page {} << ", previous_page),
        310,
        120,
        Justify::Right,
    );
    text_entities.push(previous_page_entity);

    // Direction to navigate from button 3 to button 4, placed below center button
    let below_button_three_entity =
        spawn_small_text_node(commands, "v\nButton 4".into(), 575, 325, Justify::Center);
    text_entities.push(below_button_three_entity);

    // Direction to navigate from button 3 to button 4, placed right of center button
    let right_of_button_three_entity =
        spawn_small_text_node(commands, "> Button 4".into(), 735, 255, Justify::Left);
    text_entities.push(right_of_button_three_entity);

    // Direction to navigate from button 4 to button 3, placed above bottom right button
    let below_button_three_entity =
        spawn_small_text_node(commands, "Button 3\n^".into(), 1050, 300, Justify::Center);
    text_entities.push(below_button_three_entity);

    // Direction to navigate from button 4 to button 3, placed left of bottom right button
    let right_of_button_three_entity =
        spawn_small_text_node(commands, "Button 3 < ".into(), 910, 390, Justify::Right);
    text_entities.push(right_of_button_three_entity);

    // Direction to go to the next page, placed bottom of the bottom-right button.
    let next_page_entity = spawn_small_text_node(
        commands,
        format!("V\nV\nPage {}", (page_num + 1) % 3 + 1),
        1050,
        460,
        Justify::Center,
    );
    text_entities.push(next_page_entity);
}

fn spawn_auto_nav_button(
    commands: &mut Commands,
    text: String,
    left: &f64,
    top: &f64,
    page_num: usize,
) -> Entity {
    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: px(*left),
                top: px(*top),
                width: px(140),
                height: px(100),
                border: UiRect::all(px(4)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            Page(page_num),
            BackgroundColor(NORMAL_BUTTON_COLORS[page_num].into()),
            // This is the key: just add this component for automatic navigation!
            AutoDirectionalNavigation::default(),
            ResetTimer::default(),
            Name::new(text.clone()),
        ))
        .with_child((
            Text::new(text),
            TextLayout {
                justify: Justify::Center,
                ..default()
            },
        ))
        .id()
}

fn spawn_small_text_node(
    commands: &mut Commands,
    text: String,
    left: i32,
    top: i32,
    justify: Justify,
) -> Entity {
    commands
        .spawn((
            Text::new(text),
            Node {
                position_type: PositionType::Absolute,
                left: px(left),
                top: px(top),
                width: px(140),
                padding: UiRect::all(px(12)),
                ..default()
            },
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextLayout {
                justify,
                ..default()
            },
        ))
        .id()
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
    parent_query: Query<&ChildOf>,
    mut visibility_query: Query<&mut Visibility>,
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

    // Store the previous focus in case navigation switches pages.
    let previous_focus = auto_directional_navigator.input_focus();
    if let Some(direction) = maybe_direction {
        match auto_directional_navigator.navigate(direction) {
            Ok(new_focus) => {
                // Successfully navigated!

                // If navigation switches between pages, change the visibilities of pages
                if let Ok(current_child_of) = parent_query.get(new_focus)
                    && let Ok(mut current_page_visibility) =
                        visibility_query.get_mut(current_child_of.parent())
                {
                    *current_page_visibility = Visibility::Visible;

                    if let Some(previous_focus_entity) = previous_focus
                        && let Ok(previous_child_of) = parent_query.get(previous_focus_entity)
                        && previous_child_of.parent() != current_child_of.parent()
                        && let Ok(mut previous_page_visibility) =
                            visibility_query.get_mut(previous_child_of.parent())
                    {
                        *previous_page_visibility = Visibility::Hidden;
                    }
                }
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
    mut query: Query<(Entity, &mut BorderColor, &Page)>,
) {
    for (entity, mut border_color, page) in query.iter_mut() {
        if input_focus.0 == Some(entity) && input_focus_visible.0 {
            *border_color = BorderColor::all(FOCUSED_BORDER_COLORS[page.0]);
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
