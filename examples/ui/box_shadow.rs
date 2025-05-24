//! This example shows how to create a node with a shadow and adjust its settings interactively.

use bevy::{color::palettes::css::*, prelude::*, winit::WinitSettings};

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

const SHAPE_DEFAULT_SETTINGS: ShapeSettings = ShapeSettings { index: 0 };

const SHADOW_DEFAULT_SETTINGS: ShadowSettings = ShadowSettings {
    x_offset: 20.0,
    y_offset: 20.0,
    blur: 10.0,
    spread: 15.0,
    count: 1,
};

const SHAPES: &[(&str, fn(&mut Node, &mut BorderRadius))] = &[
    ("1", |node, radius| {
        node.width = Val::Px(164.);
        node.height = Val::Px(164.);
        *radius = BorderRadius::ZERO;
    }),
    ("2", |node, radius| {
        node.width = Val::Px(164.);
        node.height = Val::Px(164.);
        *radius = BorderRadius::all(Val::Px(41.));
    }),
    ("3", |node, radius| {
        node.width = Val::Px(164.);
        node.height = Val::Px(164.);
        *radius = BorderRadius::MAX;
    }),
    ("4", |node, radius| {
        node.width = Val::Px(240.);
        node.height = Val::Px(80.);
        *radius = BorderRadius::all(Val::Px(32.));
    }),
    ("5", |node, radius| {
        node.width = Val::Px(80.);
        node.height = Val::Px(240.);
        *radius = BorderRadius::all(Val::Px(32.));
    }),
];

#[derive(Resource, Default)]
struct ShapeSettings {
    index: usize,
}

#[derive(Resource, Default)]
struct ShadowSettings {
    x_offset: f32,
    y_offset: f32,
    blur: f32,
    spread: f32,
    count: usize,
}

#[derive(Component)]
struct ShadowNode;

#[derive(Component)]
enum SettingsButton {
    XOffsetInc,
    XOffsetDec,
    YOffsetInc,
    YOffsetDec,
    BlurInc,
    BlurDec,
    SpreadInc,
    SpreadDec,
    CountInc,
    CountDec,
    ShapePrev,
    ShapeNext,
    Reset,
}

#[derive(Component)]
struct ValueLabel(String);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(SHADOW_DEFAULT_SETTINGS)
        .insert_resource(SHAPE_DEFAULT_SETTINGS)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                button_system,
                button_color_system,
                update_shape,
                update_shadow,
            ),
        )
        .run();
}

// --- UI Setup ---
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    shadow: Res<ShadowSettings>,
    shape: Res<ShapeSettings>,
) {
    commands.spawn((Camera2d, BoxShadowSamples(6)));
    // Spawn shape node
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(GRAY.into()),
        ))
        .insert(children![{
            let mut node = Node {
                width: Val::Px(164.),
                height: Val::Px(164.),
                border: UiRect::all(Val::Px(1.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            };
            let mut radius = BorderRadius::ZERO;
            SHAPES[shape.index % SHAPES.len()].1(&mut node, &mut radius);

            (
                node,
                BorderColor(WHITE.into()),
                radius,
                BackgroundColor(Color::srgb(0.21, 0.21, 0.21)),
                BoxShadow(vec![ShadowStyle {
                    color: Color::BLACK.with_alpha(0.8),
                    x_offset: Val::Px(shadow.x_offset),
                    y_offset: Val::Px(shadow.y_offset),
                    spread_radius: Val::Px(shadow.spread),
                    blur_radius: Val::Px(shadow.blur),
                }]),
                ShadowNode,
            )
        }]);

    // Settings Panel
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                top: Val::Percent(50.0),
                width: Val::Px(270.0),
                min_height: Val::Px(260.0),
                align_items: AlignItems::FlexStart,
                justify_content: JustifyContent::FlexStart,
                row_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.12).with_alpha(0.85)),
            BorderColor(Color::WHITE.with_alpha(0.15)),
            BorderRadius::all(Val::Px(12.0)),
            ZIndex(10),
        ))
        .insert(children![
            // Shape settings (transparent background)
            spawn_setting_children(
                "Shape:",
                SettingsButton::ShapePrev,
                SettingsButton::ShapeNext,
                shape.index as f32,
                &asset_server,
                BackgroundColor(Color::NONE),
            ),
            // Shadow settings (gray background)
            spawn_setting_children(
                "X Offset:",
                SettingsButton::XOffsetDec,
                SettingsButton::XOffsetInc,
                shadow.x_offset,
                &asset_server,
                BackgroundColor(Color::WHITE.with_alpha(0.08)),
            ),
            spawn_setting_children(
                "Y Offset:",
                SettingsButton::YOffsetDec,
                SettingsButton::YOffsetInc,
                shadow.y_offset,
                &asset_server,
                BackgroundColor(Color::WHITE.with_alpha(0.08)),
            ),
            spawn_setting_children(
                "Blur:",
                SettingsButton::BlurDec,
                SettingsButton::BlurInc,
                shadow.blur,
                &asset_server,
                BackgroundColor(Color::WHITE.with_alpha(0.08)),
            ),
            spawn_setting_children(
                "Spread:",
                SettingsButton::SpreadDec,
                SettingsButton::SpreadInc,
                shadow.spread,
                &asset_server,
                BackgroundColor(Color::WHITE.with_alpha(0.08)),
            ),
            spawn_setting_children(
                "Count:",
                SettingsButton::CountDec,
                SettingsButton::CountInc,
                shadow.count as f32,
                &asset_server,
                BackgroundColor(Color::WHITE.with_alpha(0.08)),
            ),
            // Reset button
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    height: Val::Px(36.0),
                    margin: UiRect::top(Val::Px(12.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                children![(
                    Button,
                    Node {
                        width: Val::Px(90.),
                        height: Val::Px(32.),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::WHITE),
                    BorderRadius::all(Val::Px(8.)),
                    SettingsButton::Reset,
                    children![(
                        Text::new("Reset"),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    )],
                )],
            ),
        ]);
}

// --- UI Helper Functions ---

// Helper to return children! macro output for a setting row
fn spawn_setting_children(
    label: &str,
    dec: SettingsButton,
    inc: SettingsButton,
    value: f32,
    asset_server: &Res<AssetServer>,
    label_bg: BackgroundColor,
) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            height: Val::Px(32.0),
            ..default()
        },
        children![
            (
                Node {
                    width: Val::Px(80.0),
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![(
                    Text::new(label),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                )],
            ),
            (
                Button,
                Node {
                    width: Val::Px(28.),
                    height: Val::Px(28.),
                    margin: UiRect::left(Val::Px(8.)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                BorderRadius::all(Val::Px(6.)),
                dec,
                children![(
                    Text::new(if label == "Shape:" { "<" } else { "-" }),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                )],
            ),
            (
                Node {
                    width: Val::Px(48.),
                    height: Val::Px(28.),
                    margin: UiRect::horizontal(Val::Px(8.)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                label_bg,
                BorderRadius::all(Val::Px(6.)),
                children![{
                    if label == "Shape:" {
                        (
                            Text::new(SHAPES[value as usize % SHAPES.len()].0),
                            TextFont {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            ValueLabel(label.to_string()),
                        )
                    } else {
                        (
                            Text::new(if label == "Count:" {
                                format!("{}", value as usize)
                            } else {
                                format!("{:.1}", value)
                            }),
                            TextFont {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            ValueLabel(label.to_string()),
                        )
                    }
                }],
            ),
            (
                Button,
                Node {
                    width: Val::Px(28.),
                    height: Val::Px(28.),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                BorderRadius::all(Val::Px(6.)),
                inc,
                children![(
                    Text::new(if label == "Shape:" { ">" } else { "+" }),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                )],
            ),
        ],
    )
}

// --- SYSTEMS ---

// Update the shadow node's BoxShadow and background color if settings changed
fn update_shadow(
    shadow: Res<ShadowSettings>,
    mut query: Query<(&mut BoxShadow, &mut BackgroundColor), With<ShadowNode>>,
    mut label_query: Query<(&mut Text, &ValueLabel)>,
) {
    if !shadow.is_changed() {
        return;
    }
    for (mut box_shadow, _background_color) in &mut query {
        *box_shadow = BoxShadow(generate_shadows(&shadow));
    }
    // Update value labels for shadow settings
    for (mut text, label) in &mut label_query {
        let value = match label.0.as_str() {
            "X Offset:" => format!("{:.1}", shadow.x_offset),
            "Y Offset:" => format!("{:.1}", shadow.y_offset),
            "Blur:" => format!("{:.1}", shadow.blur),
            "Spread:" => format!("{:.1}", shadow.spread),
            "Count:" => format!("{}", shadow.count),
            _ => continue,
        };
        if label.0 != "Shape:" {
            *text = Text::new(value);
        }
    }
}

fn generate_shadows(shadow: &ShadowSettings) -> Vec<ShadowStyle> {
    match shadow.count {
        1 => vec![make_shadow(
            BLACK.into(),
            shadow.x_offset,
            shadow.y_offset,
            shadow.spread,
            shadow.blur,
        )],
        2 => vec![
            make_shadow(
                BLUE.into(),
                shadow.x_offset,
                shadow.y_offset,
                shadow.spread,
                shadow.blur,
            ),
            make_shadow(
                YELLOW.into(),
                -shadow.x_offset,
                -shadow.y_offset,
                shadow.spread,
                shadow.blur,
            ),
        ],
        3 => vec![
            make_shadow(
                BLUE.into(),
                shadow.x_offset,
                shadow.y_offset,
                shadow.spread,
                shadow.blur,
            ),
            make_shadow(
                YELLOW.into(),
                -shadow.x_offset,
                -shadow.y_offset,
                shadow.spread,
                shadow.blur,
            ),
            make_shadow(
                RED.into(),
                shadow.y_offset,
                -shadow.x_offset,
                shadow.spread,
                shadow.blur,
            ),
        ],
        _ => vec![],
    }
}

fn make_shadow(color: Color, x_offset: f32, y_offset: f32, spread: f32, blur: f32) -> ShadowStyle {
    ShadowStyle {
        color: color.with_alpha(0.8),
        x_offset: Val::Px(x_offset),
        y_offset: Val::Px(y_offset),
        spread_radius: Val::Px(spread),
        blur_radius: Val::Px(blur),
    }
}

// Update shape of ShadowNode if shape selection changed
fn update_shape(
    shape: Res<ShapeSettings>,
    mut query: Query<(&mut Node, &mut BorderRadius), With<ShadowNode>>,
    mut label_query: Query<(&mut Text, &ValueLabel)>,
) {
    if !shape.is_changed() {
        return;
    }
    for (mut node, mut radius) in &mut query {
        SHAPES[shape.index % SHAPES.len()].1(&mut node, &mut radius);
    }
    for (mut text, label) in &mut label_query {
        if label.0 == "Shape:" {
            *text = Text::new(SHAPES[shape.index % SHAPES.len()].0);
        }
    }
}

// Handles button interactions for all settings
fn button_system(
    mut interaction_query: Query<
        (&Interaction, &SettingsButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut shadow: ResMut<ShadowSettings>,
    mut shape: ResMut<ShapeSettings>,
) {
    for (interaction, btn) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            match btn {
                SettingsButton::XOffsetInc => shadow.x_offset += 1.0,
                SettingsButton::XOffsetDec => shadow.x_offset -= 1.0,
                SettingsButton::YOffsetInc => shadow.y_offset += 1.0,
                SettingsButton::YOffsetDec => shadow.y_offset -= 1.0,
                SettingsButton::BlurInc => shadow.blur = (shadow.blur + 1.0).max(0.0),
                SettingsButton::BlurDec => shadow.blur = (shadow.blur - 1.0).max(0.0),
                SettingsButton::SpreadInc => shadow.spread += 1.0,
                SettingsButton::SpreadDec => shadow.spread -= 1.0,
                SettingsButton::CountInc => {
                    if shadow.count < 3 {
                        shadow.count += 1;
                    }
                }
                SettingsButton::CountDec => {
                    if shadow.count > 1 {
                        shadow.count -= 1;
                    }
                }
                SettingsButton::ShapePrev => {
                    if shape.index == 0 {
                        shape.index = SHAPES.len() - 1;
                    } else {
                        shape.index -= 1;
                    }
                }
                SettingsButton::ShapeNext => {
                    shape.index = (shape.index + 1) % SHAPES.len();
                }
                SettingsButton::Reset => {
                    *shape = SHAPE_DEFAULT_SETTINGS;
                    *shadow = SHADOW_DEFAULT_SETTINGS;
                }
            }
        }
    }
}

// Changes color of button on hover and on pressed
fn button_color_system(
    mut query: Query<
        (&Interaction, &mut BackgroundColor, Option<&SettingsButton>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, shadow_btn) in &mut query {
        if shadow_btn.is_some() {
            match *interaction {
                Interaction::Pressed => *color = PRESSED_BUTTON.into(),
                Interaction::Hovered => *color = HOVERED_BUTTON.into(),
                Interaction::None => *color = NORMAL_BUTTON.into(),
            }
        }
    }
}
