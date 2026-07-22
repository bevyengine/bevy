//! This example shows how to create a node with a shadow and adjust its settings interactively.

use crate::radio::{feathers_option_buttons, main_ui_node_scene, RadioButtonOptionValue};
use bevy::{
    color::palettes::css::*,
    feathers::{dark_theme::create_dark_theme, theme::UiTheme, FeathersPlugins},
    prelude::*,
    time::Time,
    ui_widgets::{radio_self_update, ValueChange},
    window::RequestRedraw,
};

#[path = "../../helpers/radio.rs"]
mod radio;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

const SHADOW_DEFAULT_SETTINGS: ShadowSettings = ShadowSettings {
    x_offset: 20.0,
    y_offset: 20.0,
    blur: 10.0,
    spread: 15.0,
    count: 1,
    samples: 6,
};

/// The current shape displayed on the screen
#[derive(Component, Clone, Copy, Default, PartialEq)]
enum Shape {
    #[default]
    Square,
    RoundedSquare,
    Circle,
    LongRectangle,
    TallRectangle,
}

impl Shape {
    /// Mutates the `node` to become the given shape.
    fn change_node(&self, node: &mut Node) {
        match *self {
            Self::Square => {
                node.width = px(164);
                node.height = px(164);
                node.border_radius = BorderRadius::ZERO;
            }
            Self::RoundedSquare => {
                node.width = px(164);
                node.height = px(164);
                node.border_radius = BorderRadius::all(px(41));
            }
            Self::Circle => {
                node.width = px(164);
                node.height = px(164);
                node.border_radius = BorderRadius::MAX;
            }
            Self::LongRectangle => {
                node.width = px(240);
                node.height = px(80);
                node.border_radius = BorderRadius::all(px(32));
            }
            Self::TallRectangle => {
                node.width = px(80);
                node.height = px(240);
                node.border_radius = BorderRadius::all(px(32));
            }
        }
    }

    /// Returns the name of the shape as a string.
    fn name(&self) -> &'static str {
        match *self {
            Shape::Square => "Square",
            Shape::RoundedSquare => "Rounded Square",
            Shape::Circle => "Circle",
            Shape::LongRectangle => "Long Rectangle",
            Shape::TallRectangle => "Tall Rectangle",
        }
    }
}

#[derive(Resource, Default)]
struct AppSettings {
    shape: Shape,
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

#[derive(Component, PartialEq, Clone, Copy, Default)]
enum SettingsButton {
    #[default]
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
    Reset,
    SamplesInc,
    SamplesDec,
}

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
enum SettingType {
    #[default]
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
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .init_resource::<AppSettings>()
        .insert_resource(SHADOW_DEFAULT_SETTINGS)
        .insert_resource(HeldButton::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                button_system,
                button_color_system,
                update_shadow.run_if(resource_changed::<ShadowSettings>),
                update_shadow_samples.run_if(resource_changed::<ShadowSettings>),
                button_repeat_system,
            ),
        )
        .add_observer(on_value_change_update_shape)
        .add_observer(radio_self_update)
        .run();
}

// --- UI Setup ---
fn setup(mut commands: Commands, shadow: Res<ShadowSettings>, app_settings: Res<AppSettings>) {
    commands.spawn((Camera2d, BoxShadowSamples(shadow.samples)));
    // Spawn shape node
    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(GRAY.into()),
        ))
        .insert(children![{
            let mut node = Node {
                width: px(164),
                height: px(164),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::ZERO,
                ..default()
            };
            app_settings.shape.change_node(&mut node);

            (
                node,
                BorderColor::all(WHITE),
                BackgroundColor(Color::srgb(0.21, 0.21, 0.21)),
                BoxShadow(vec![ShadowStyle {
                    color: Color::BLACK.with_alpha(0.8),
                    x_offset: px(shadow.x_offset),
                    y_offset: px(shadow.y_offset),
                    spread_radius: px(shadow.spread),
                    blur_radius: px(shadow.blur),
                }]),
                ShadowNode,
            )
        }]);

    // Settings Panel
    commands.spawn_scene(bsn! {
        main_ui_node_scene()
        ZIndex(10)
        Children [
            feathers_option_buttons(
                "Shape",
                &[
                    (Shape::Square, Shape::Square.name()),
                    (Shape::RoundedSquare, Shape::RoundedSquare.name()),
                    (Shape::Circle, Shape::Circle.name()),
                    (Shape::LongRectangle, Shape::LongRectangle.name()),
                    (Shape::TallRectangle, Shape::TallRectangle.name()),
                ],
            ),
            build_setting_row(
                SettingType::XOffset,
                SettingsButton::XOffsetDec,
                SettingsButton::XOffsetInc,
                shadow.x_offset,
            ),
            build_setting_row(
                SettingType::YOffset,
                SettingsButton::YOffsetDec,
                SettingsButton::YOffsetInc,
                shadow.y_offset,
            ),
            build_setting_row(
                SettingType::Blur,
                SettingsButton::BlurDec,
                SettingsButton::BlurInc,
                shadow.blur,
            ),
            build_setting_row(
                SettingType::Spread,
                SettingsButton::SpreadDec,
                SettingsButton::SpreadInc,
                shadow.spread,
            ),
            build_setting_row(
                SettingType::Count,
                SettingsButton::CountDec,
                SettingsButton::CountInc,
                shadow.count as f32,
            ),
            // Add BoxShadowSamples as a setting row
            build_setting_row(
                SettingType::Samples,
                SettingsButton::SamplesDec,
                SettingsButton::SamplesInc,
                shadow.samples as f32,
            ),
            // Reset button
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                height: px(36),
                margin: UiRect::top(px(12)),
            }
            Children [
                Button
                Node {
                    width: px(90),
                    height: px(32),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(px(8)),
                }
                BackgroundColor(NORMAL_BUTTON)
                template_value(SettingsButton::Reset)
                Children [
                    Text::new("Reset")
                ],
            ],
        ]
    });
}

// --- UI Helper Functions ---

// Helper to return an input to the children! macro for a setting row
fn build_setting_row(
    setting_type: SettingType,
    dec: SettingsButton,
    inc: SettingsButton,
    value: f32,
) -> impl Scene {
    let value_text = match setting_type {
        SettingType::Shape => "TODO To Remove".to_string(),
        SettingType::Count => format!("{}", value as usize),
        _ => format!("{value:.1}"),
    };

    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            height: px(32),
        }
        Children [
            Node {
                width: px(80),
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
            }
            // Attach SettingType to the value label node, not the parent row
            Children [
                Text::new(setting_type.label())
            ],


            Button
            Node {
                width: px(28),
                height: px(28),
                margin: UiRect::left(px(8)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(6)),
            }
            BackgroundColor(Color::WHITE)
            template_value(dec)
            Children [
                Text::new("-")
            ],

            Node {
                width: px(48),
                height: px(28),
                margin: UiRect::horizontal(px(8)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(6)),
            }
            Children [
                Text::new(value_text)
                template_value(setting_type)
            ],
            Button
            Node {
                width: px(28),
                height: px(28),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(6)),
            }
            BackgroundColor(Color::WHITE)
            template_value(inc)
            Children [
                Text::new("+")
            ],
        ]
    }
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
        x_offset: px(x_offset),
        y_offset: px(y_offset),
        spread_radius: px(spread),
        blur_radius: px(blur),
    }
}

// Update shape of ShadowNode if shape selection changed
fn on_value_change_update_shape(
    event: On<ValueChange<Entity>>,
    new_value_query: Query<&RadioButtonOptionValue<Shape>>,
    mut app_settings: ResMut<AppSettings>,
    query: Query<&mut Node, With<ShadowNode>>,
    label_query: Query<(&mut Text, &SettingType)>,
) {
    let Ok(RadioButtonOptionValue(shape)) = new_value_query.get(event.value) else {
        return;
    };
    app_settings.shape = *shape;

    update_shape(&app_settings, query, label_query);
}

fn update_shape(
    app_settings: &AppSettings,
    mut query: Query<&mut Node, With<ShadowNode>>,
    mut label_query: Query<(&mut Text, &SettingType)>,
) {
    for mut node in &mut query {
        app_settings.shape.change_node(&mut node);
    }
    for (mut text, kind) in &mut label_query {
        if *kind == SettingType::Shape {
            *text = Text::new(app_settings.shape.name());
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
    mut app_settings: ResMut<AppSettings>,
    mut held: ResMut<HeldButton>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    for (interaction, btn) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                trigger_button_action(btn, &mut shadow, &mut app_settings);
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
    app_settings: &mut AppSettings,
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
        SettingsButton::Reset => {
            *app_settings = AppSettings::default();
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
    mut app_settings: ResMut<AppSettings>,
    mut request_redraw_writer: MessageWriter<RequestRedraw>,
) {
    if held.button.is_some() {
        request_redraw_writer.write(RequestRedraw);
    }
    const INITIAL_DELAY: f64 = 0.15;
    const REPEAT_RATE: f64 = 0.08;
    if let (Some(btn), Some(pressed_at)) = (held.button, held.pressed_at) {
        let now = time.elapsed_secs_f64();
        let since_pressed = now - pressed_at;
        let last_repeat = held.last_repeat.unwrap_or(pressed_at);
        let since_last = now - last_repeat;
        if since_pressed > INITIAL_DELAY && since_last > REPEAT_RATE {
            trigger_button_action(&btn, &mut shadow, &mut app_settings);
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
