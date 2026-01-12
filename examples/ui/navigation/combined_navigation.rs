//! Demonstrates a combination of automatic and manual directional navigation.
//!
//! This example shows how to leverage both automatic navigation and manual navigation to create
//! a desired user navigation experience without much boilerplate code. In this example, there are
//! multiple pages of UI Buttons. When navigating within the button grid on a page (up, down, left
//! and right), automatic navigation is used. However, to add more bespoke navigation that cannot
//! be automatically detected by screen position proximity alone, manual navigation must be used.
//!
//! Manual navigation is needed to create transitions between rows and between pages.
//! These transitions are not created by the automatic navigation system. Moving right at the
//! end of the previous row navigates to the beginning of the next row. At the end of a page
//! (the bottom right most button), moving right navigates to the first button on the next page.
//!
//! The `AutoDirectionalNavigation` component is used to add basic, intuitive navigation to UI
//! elements within a page. Manual edges between rows and between pages are added to the
//! `DirectionalNavigationMap` to allow special navigation rules. The `AutoDirectionalNavigator`
//! system parameter navigates using manual navigation first and automatic navigation second.

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
            // Prefer nodes that are well-aligned
            prefer_aligned: false,
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
/// This will create three pages of buttons. Within the rows and columns of a page,
/// automatic navigation will be utilized. Between rows of the same page, manual
/// navigation will connect the end of one row with the beginning of the next row. Between
/// pages themselves, manual navigation will connect the button on the bottom right with
/// the next page's button on the top left.
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
                 Horizontal navigation within rows is configured automatically.\n\
                 Horizontal navigation between rows is defined manually.\n\
                 Navigation between pages is defined manually.",
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
        [(500.0, 80.0), (700.0, 80.0), (900.0, 80.0)],
        // Row 1
        [(500.0, 215.0), (700.0, 215.0), (900.0, 215.0)],
        // Row 2
        [(500.0, 350.0), (700.0, 350.0), (900.0, 350.0)],
        // Row 3
        [(500.0, 485.0), (700.0, 485.0), (900.0, 485.0)],
    ];

    let mut pages_entities = [
        Vec::with_capacity(12),
        Vec::with_capacity(12),
        Vec::with_capacity(12),
    ];
    for (page_num, page_button_entities) in pages_entities.iter_mut().enumerate() {
        setup_buttons_for_page(
            &mut commands,
            page_num,
            page_button_entities,
            &button_positions,
        );

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

        // Prompt to go to the previous page, placed left of the top-left button.
        let previous_page = if page_num == 0 { 3 } else { page_num };
        let previous_page_node = commands
            .spawn((
                Text::new(format!("Page {} << ", previous_page)),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(360),
                    top: px(120),
                    width: px(140),
                    padding: UiRect::all(px(12)),
                    ..default()
                },
                TextLayout {
                    justify: Justify::Right,
                    ..default()
                },
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
            ))
            .id();

        // Prompt to go to the next page, placed right of the bottom-right button.
        let next_page_node = commands
            .spawn((
                Text::new(format!(">> Page {}", (page_num + 1) % 3 + 1)),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(1050),
                    top: px(525),
                    width: px(140),
                    padding: UiRect::all(px(12)),
                    ..default()
                },
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
            ))
            .id();

        commands
            .entity(page)
            .add_children(page_button_entities)
            .add_children(&[previous_page_node, next_page_node]);
    }
    let first_button = Some(pages_entities[0][0]);

    // Add manual edges within each page for navigation between rows
    let entity_pairs = [
        // the end of the first row should connect to the beginning of the second
        ((0, 2), (1, 0)),
        // the end of the second row should connect to the beginning of the third
        ((1, 2), (2, 0)),
        // the end of the third row should connect to the beginning of the fourth
        ((2, 2), (3, 0)),
    ];
    for page_entities in pages_entities.iter() {
        for ((entity_a_row, entity_a_col), (entity_b_row, entity_b_col)) in entity_pairs.iter() {
            manual_directional_nav_map.add_symmetrical_edge(
                page_entities[entity_a_row * 3 + entity_a_col],
                page_entities[entity_b_row * 3 + entity_b_col],
                CompassOctant::East,
            );
        }
    }

    // Add manual edges between pages. When navigating right (east) from the last button of a page,
    // go to the first button of the next page.
    manual_directional_nav_map.add_symmetrical_edge(
        pages_entities[0][11],
        pages_entities[1][0],
        CompassOctant::East,
    );
    manual_directional_nav_map.add_symmetrical_edge(
        pages_entities[1][11],
        pages_entities[2][0],
        CompassOctant::East,
    );
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

/// Creates the button entities for a page and places the entities into `page_entities`.
fn setup_buttons_for_page(
    commands: &mut Commands,
    page_num: usize,
    page_entities: &mut Vec<Entity>,
    button_positions: &[[(f64, f64); 3]; 4],
) {
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
                    Name::new(format!(
                        "Page {},\nRow {},\nButton {}",
                        page_num + 1,
                        i + 1,
                        j + 1
                    )),
                ))
                .with_child((
                    Text::new(format!(
                        "Page {},\nRow {},\nButton {}",
                        page_num + 1,
                        i + 1,
                        j + 1
                    )),
                    TextLayout {
                        justify: Justify::Center,
                        ..default()
                    },
                ))
                .id();
            page_entities.push(button_entity);
        }
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
