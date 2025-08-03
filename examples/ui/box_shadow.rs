//! This example shows how to create a node with a shadow and adjust its settings interactively.

use bevy::{
    color::palettes::css::*, prelude::*, time::Time, window::RequestRedraw, winit::WinitSettings,
};

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
    samples: 6,
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
    samples: u32,
}

#[derive(Component)]
struct ShadowNode;

#[derive(Component, PartialEq, Clone, Copy)]
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
    SamplesInc,
    SamplesDec,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum SettingType {
    XOffset,
    YOffset,
    Blur,
    Spread,
    Count,
    Shape,
    Samples,
}

impl SettingType {
    fn label(&self) -> &str {
        match self {
            SettingType::XOffset => "X Offset",
            SettingType::YOffset => "Y Offset",
            SettingType::Blur => "Blur",
            SettingType::Spread => "Spread",
            SettingType::Count => "Count",
            SettingType::Shape => "Shape",
            SettingType::Samples => "Samples",
        }
    }
}

#[derive(Resource, Default)]
struct HeldButton {
    button: Option<SettingsButton>,
    pressed_at: Option<f64>,
    last_repeat: Option<f64>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(SHADOW_DEFAULT_SETTINGS)
        .insert_resource(SHAPE_DEFAULT_SETTINGS)
        .insert_resource(HeldButton::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                button_system,
                button_color_system,
                update_shape.run_if(resource_changed::<ShapeSettings>),
                update_shadow.run_if(resource_changed::<ShadowSettings>),
                update_shadow_samples.run_if(resource_changed::<ShadowSettings>),
                button_repeat_system,
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
    commands.spawn((Camera2d, BoxShadowSamples(shadow.samples)));
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
                BorderColor::all(WHITE),
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
                bottom: Val::Px(24.0),
                width: Val::Px(270.0),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.12).with_alpha(0.85)),
            BorderColor::all(Color::WHITE.with_alpha(0.15)),
            BorderRadius::all(Val::Px(12.0)),
            ZIndex(10),
        ))
        .insert(children![
            build_setting_row(
                SettingType::Shape,
                SettingsButton::ShapePrev,
                SettingsButton::ShapeNext,
                shape.index as f32,
                &asset_server,
            ),
            build_setting_row(
                SettingType::XOffset,
                SettingsButton::XOffsetDec,
                SettingsButton::XOffsetInc,
                shadow.x_offset,
                &asset_server,
            ),
            build_setting_row(
                SettingType::YOffset,
                SettingsButton::YOffsetDec,
                SettingsButton::YOffsetInc,
                shadow.y_offset,
                &asset_server,
            ),
            build_setting_row(
                SettingType::Blur,
                SettingsButton::BlurDec,
                SettingsButton::BlurInc,
                shadow.blur,
                &asset_server,
            ),
            build_setting_row(
                SettingType::Spread,
                SettingsButton::SpreadDec,
                SettingsButton::SpreadInc,
                shadow.spread,
                &asset_server,
            ),
            build_setting_row(
                SettingType::Count,
                SettingsButton::CountDec,
                SettingsButton::CountInc,
                shadow.count as f32,
                &asset_server,
            ),
            // Add BoxShadowSamples as a setting row
            build_setting_row(
                SettingType::Samples,
                SettingsButton::SamplesDec,
                SettingsButton::SamplesInc,
                shadow.samples as f32,
                &asset_server,
            ),
            // Reset button
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    height: Val::Px(36.0),
                    margin: UiRect::top(Val::Px(12.0)),
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
                    BackgroundColor(NORMAL_BUTTON),
                    BorderRadius::all(Val::Px(8.)),
                    SettingsButton::Reset,
                    children![(
                        Text::new("Reset"),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 16.0,
                            ..default()
                        },
                    )],
                )],
            ),
        ]);
}

// --- UI Helper Functions ---

// Helper to return an input to the children! macro for a setting row
fn build_setting_row(
    setting_type: SettingType,
    dec: SettingsButton,
    inc: SettingsButton,
    value: f32,
    asset_server: &Res<AssetServer>,
) -> impl Bundle {
    let value_text = match setting_type {
        SettingType::Shape => SHAPES[value as usize % SHAPES.len()].0.to_string(),
        SettingType::Count => format!("{}", value as usize),
        _ => format!("{value:.1}"),
    };

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
                // Attach SettingType to the value label node, not the parent row
                children![(
                    Text::new(setting_type.label()),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 16.0,
                        ..default()
                    },
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
                    Text::new(if setting_type == SettingType::Shape {
                        "<"
                    } else {
                        "-"
                    }),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 18.0,
                        ..default()
                    },
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
                BorderRadius::all(Val::Px(6.)),
                children![{
                    (
                        Text::new(value_text),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 16.0,
                            ..default()
                        },
                        setting_type,
                    )
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
                    Text::new(if setting_type == SettingType::Shape {
                        ">"
                    } else {
                        "+"
                    }),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 18.0,
                        ..default()
                    },
                )],
            ),
        ],
    )
}

// --- SYSTEMS ---

// Update the shadow node's BoxShadow on resource changes
fn update_shadow(
    shadow: Res<ShadowSettings>,
    mut query: Query<&mut BoxShadow, With<ShadowNode>>,
    mut label_query: Query<(&mut Text, &SettingType)>,
) {
    for mut box_shadow in &mut query {
        *box_shadow = BoxShadow(generate_shadows(&shadow));
    }
    // Update value labels for shadow settings
    for (mut text, setting) in &mut label_query {
        let value = match setting {
            SettingType::XOffset => format!("{:.1}", shadow.x_offset),
            SettingType::YOffset => format!("{:.1}", shadow.y_offset),
            SettingType::Blur => format!("{:.1}", shadow.blur),
            SettingType::Spread => format!("{:.1}", shadow.spread),
            SettingType::Count => format!("{}", shadow.count),
            SettingType::Shape => continue,
            SettingType::Samples => format!("{}", shadow.samples),
        };
        *text = Text::new(value);
    }
}

fn update_shadow_samples(
    shadow: Res<ShadowSettings>,
    mut query: Query<&mut BoxShadowSamples, With<Camera2d>>,
) {
    for mut samples in &mut query {
        samples.0 = shadow.samples;
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
    mut label_query: Query<(&mut Text, &SettingType)>,
) {
    for (mut node, mut radius) in &mut query {
        SHAPES[shape.index % SHAPES.len()].1(&mut node, &mut radius);
    }
    for (mut text, kind) in &mut label_query {
        if *kind == SettingType::Shape {
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
    mut held: ResMut<HeldButton>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    for (interaction, btn) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                trigger_button_action(btn, &mut shadow, &mut shape);
                held.button = Some(*btn);
                held.pressed_at = Some(now);
                held.last_repeat = Some(now);
            }
            Interaction::None | Interaction::Hovered => {
                if held.button == Some(*btn) {
                    held.button = None;
                    held.pressed_at = None;
                    held.last_repeat = None;
                }
            }
        }
    }
}

fn trigger_button_action(
    btn: &SettingsButton,
    shadow: &mut ShadowSettings,
    shape: &mut ShapeSettings,
) {
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
        SettingsButton::SamplesInc => shadow.samples += 1,
        SettingsButton::SamplesDec => {
            if shadow.samples > 1 {
                shadow.samples -= 1;
            }
        }
    }
}

// System to repeat button action while held
fn button_repeat_system(
    time: Res<Time>,
    mut held: ResMut<HeldButton>,
    mut shadow: ResMut<ShadowSettings>,
    mut shape: ResMut<ShapeSettings>,
    mut redraw_events: EventWriter<RequestRedraw>,
) {
    if held.button.is_some() {
        redraw_events.write(RequestRedraw);
    }
    const INITIAL_DELAY: f64 = 0.15;
    const REPEAT_RATE: f64 = 0.08;
    if let (Some(btn), Some(pressed_at)) = (held.button, held.pressed_at) {
        let now = time.elapsed_secs_f64();
        let since_pressed = now - pressed_at;
        let last_repeat = held.last_repeat.unwrap_or(pressed_at);
        let since_last = now - last_repeat;
        if since_pressed > INITIAL_DELAY && since_last > REPEAT_RATE {
            trigger_button_action(&btn, &mut shadow, &mut shape);
            held.last_repeat = Some(now);
        }
    }
}

// Changes color of button on hover and on pressed
fn button_color_system(
    mut query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>, With<SettingsButton>),
    >,
) {
    for (interaction, mut color) in &mut query {
        match *interaction {
            Interaction::Pressed => *color = PRESSED_BUTTON.into(),
            Interaction::Hovered => *color = HOVERED_BUTTON.into(),
            Interaction::None => *color = NORMAL_BUTTON.into(),
        }
    }
}
