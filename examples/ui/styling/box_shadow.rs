//! This example shows how to create a node with a shadow and adjust its settings interactively.

use crate::number_input::{number_input_f32, number_input_i32};
use crate::radio::{feathers_option_buttons, main_ui_node_scene, RadioButtonOptionValue};
use bevy::{
    color::palettes::css::*,
    feathers::{
        containers::{pane, pane_body},
        controls::{FeathersButton, FeathersNumberInput, NumberInputPrecision, NumberInputValue},
        dark_theme::create_dark_theme,
        display::caption,
        theme::UiTheme,
        FeathersPlugins,
    },
    prelude::*,
    ui_widgets::{radio_self_update, Activate, ValueChange},
};

#[path = "../../helpers/number_input.rs"]
mod number_input;

#[path = "../../helpers/radio.rs"]
mod radio;

/// Shapes that the node displayed on the screen can be.
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
    const fn label(&self) -> &'static str {
        match *self {
            Shape::Square => "Square",
            Shape::RoundedSquare => "Rounded Square",
            Shape::Circle => "Circle",
            Shape::LongRectangle => "Long Rectangle",
            Shape::TallRectangle => "Tall Rectangle",
        }
    }
}

const SHAPE_OPTIONS: [(Shape, &str); 5] = [
    (Shape::Square, Shape::Square.label()),
    (Shape::RoundedSquare, Shape::RoundedSquare.label()),
    (Shape::Circle, Shape::Circle.label()),
    (Shape::LongRectangle, Shape::LongRectangle.label()),
    (Shape::TallRectangle, Shape::TallRectangle.label()),
];

/// Settings that the user can modify within the example.
#[derive(Resource)]
struct AppSettings {
    shape: Shape,
    // Refer to `ShadowStyle` for information on x_offset, y_offset,
    // blur (radius), and spread (radius)
    x_offset: f32,
    y_offset: f32,
    blur: f32,
    spread: f32,
    /// The number of `BoxShadow`s created
    count: usize,
    /// The number of samples used in `BoxShadowSamples`
    samples: u32,
}

/// The example initializes with these default settings.
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            shape: Shape::default(),
            x_offset: 20.0,
            y_offset: 20.0,
            blur: 10.0,
            spread: 15.0,
            count: 1,
            samples: 6,
        }
    }
}

#[derive(Component, Clone, Default)]
struct ShadowNode;

#[derive(Component, Clone, Default)]
struct SettingsPanel;

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
enum AppNumberInputF32 {
    #[default]
    XOffset,
    YOffset,
    Blur,
    Spread,
}

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
enum AppNumberInputI32 {
    #[default]
    Count,
    Samples,
}

impl AppNumberInputF32 {
    fn label(&self) -> &str {
        match self {
            AppNumberInputF32::XOffset => "X Offset",
            AppNumberInputF32::YOffset => "Y Offset",
            AppNumberInputF32::Blur => "Blur",
            AppNumberInputF32::Spread => "Spread",
        }
    }
}

impl AppNumberInputI32 {
    fn label(&self) -> &str {
        match self {
            AppNumberInputI32::Count => "Count (1 - 3)",
            AppNumberInputI32::Samples => "Samples (0 - 15)",
        }
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .init_resource::<AppSettings>()
        .add_systems(Startup, setup)
        .add_observer(on_value_change_i32_update_shadow)
        .add_observer(on_value_change_f32_update_shadow)
        .add_observer(on_value_change_update_shape)
        .add_observer(radio_self_update)
        .run();
}

// --- UI Setup ---
fn setup(mut commands: Commands, app_settings: Res<AppSettings>) {
    // create shape node
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

    commands.spawn_scene_list(bsn_list! {
        // Camera
        Camera2d
        BoxShadowSamples({app_settings.samples}),

        // Centered shape with shadow
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        BackgroundColor(GRAY)
        Children [
            template_value(node)
            BorderColor::all(WHITE)
            BackgroundColor(Color::srgb(0.21, 0.21, 0.21))
            BoxShadow(vec![ShadowStyle {
                color: Color::BLACK.with_alpha(0.8),
                x_offset: px(app_settings.x_offset),
                y_offset: px(app_settings.y_offset),
                spread_radius: px(app_settings.spread),
                blur_radius: px(app_settings.blur),
            }])
            ShadowNode
        ],

        settings_panel_scene(&app_settings),
    });
}

fn settings_panel_scene(app_settings: &AppSettings) -> impl Scene {
    bsn! {
        SettingsPanel
        ZIndex(10)
        pane()
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            bottom: px(0),
        }
        Children [
            pane_body()
            Children [
                feathers_option_buttons(
                    "Shape",
                    &SHAPE_OPTIONS,
                ),
                number_input_f32(
                    AppNumberInputF32::XOffset.label(),
                    Some(AppNumberInputF32::XOffset),
                    app_settings.x_offset,
                    NumberInputPrecision(0),
                    -200. ..200.
                ),
                number_input_f32(
                    AppNumberInputF32::YOffset.label(),
                    Some(AppNumberInputF32::YOffset),
                    app_settings.y_offset,
                    NumberInputPrecision(0),
                    -200. ..200.
                ),
                number_input_f32(
                    AppNumberInputF32::Blur.label(),
                    Some(AppNumberInputF32::Blur),
                    app_settings.blur,
                    NumberInputPrecision(0),
                    0. ..100.
                ),
                number_input_f32(
                    AppNumberInputF32::Spread.label(),
                    Some(AppNumberInputF32::Spread),
                    app_settings.spread,
                    NumberInputPrecision(0),
                    -200. ..200.
                ),
                number_input_i32(
                    AppNumberInputI32::Count.label(),
                    Some(AppNumberInputI32::Count),
                    app_settings.count as i32,
                    NumberInputPrecision(0),
                    1..3
                ),
                number_input_i32(
                    AppNumberInputI32::Samples.label(),
                    Some(AppNumberInputI32::Samples),
                    app_settings.samples as i32,
                    NumberInputPrecision(0),
                    1..15
                ),
                // Reset button
                @FeathersButton {
                    @caption: bsn! { caption("Reset") }
                }
                on(on_activate_reset)
            ]
        ]
    }
}

// --- SYSTEMS ---

/// Update the shadow node's `BoxShadow` or the camera's `BoxShadowSamples` on any change to the i32 number inputs.
fn on_value_change_i32_update_shadow(
    value_change: On<ValueChange<i32>>,
    number_input_q: Query<&AppNumberInputI32, With<FeathersNumberInput>>,
    mut commands: Commands,
    mut app_settings: ResMut<AppSettings>,
    mut box_shadow_q: Query<&mut BoxShadow, With<ShadowNode>>,
    mut samples_q: Query<&mut BoxShadowSamples, With<Camera2d>>,
) {
    if let Ok(app_number_input_i32) = number_input_q.get(value_change.source) {
        match app_number_input_i32 {
            AppNumberInputI32::Count => {
                app_settings.count = value_change.value as usize;
                for mut box_shadow in &mut box_shadow_q {
                    *box_shadow = BoxShadow(generate_shadows(&app_settings));
                }
            }
            AppNumberInputI32::Samples => {
                app_settings.samples = value_change.value as u32;
                for mut samples in &mut samples_q {
                    samples.0 = app_settings.samples;
                }
            }
        }
    }

    commands
        .entity(value_change.source)
        .insert(NumberInputValue::I32(value_change.value));
}

/// Update the shadow node's `BoxShadow` on any change to the f32 number inputs.
fn on_value_change_f32_update_shadow(
    value_change: On<ValueChange<f32>>,
    number_input_q: Query<&AppNumberInputF32, With<FeathersNumberInput>>,
    mut commands: Commands,
    mut app_settings: ResMut<AppSettings>,
    mut box_shadow_q: Query<&mut BoxShadow, With<ShadowNode>>,
) {
    if let Ok(app_number_input_i32) = number_input_q.get(value_change.source) {
        match app_number_input_i32 {
            AppNumberInputF32::XOffset => {
                app_settings.x_offset = value_change.value;
            }
            AppNumberInputF32::YOffset => {
                app_settings.y_offset = value_change.value;
            }
            AppNumberInputF32::Blur => {
                app_settings.blur = value_change.value;
            }
            AppNumberInputF32::Spread => {
                app_settings.spread = value_change.value;
            }
        }
        for mut box_shadow in &mut box_shadow_q {
            *box_shadow = BoxShadow(generate_shadows(&app_settings));
        }
    }

    commands
        .entity(value_change.source)
        .insert(NumberInputValue::F32(value_change.value));
}

/// Update shape of `ShadowNode` if shape selection has changed
fn on_value_change_update_shape(
    event: On<ValueChange<Entity>>,
    new_value_query: Query<&RadioButtonOptionValue<Shape>>,
    mut app_settings: ResMut<AppSettings>,
    mut node_q: Query<&mut Node, With<ShadowNode>>,
) {
    let Ok(RadioButtonOptionValue(shape)) = new_value_query.get(event.value) else {
        return;
    };
    app_settings.shape = *shape;

    for mut node in &mut node_q {
        app_settings.shape.change_node(&mut node);
    }
}

/// If the reset button was activated, reset app settings to default values.
/// This observer is placed directly on the reset button.
fn on_activate_reset(
    _event: On<Activate>,
    mut commands: Commands,
    mut app_settings: ResMut<AppSettings>,
    settings_panel_q: Single<Entity, With<SettingsPanel>>,
    mut node_q: Query<&mut Node, With<ShadowNode>>,
    mut box_shadow_q: Query<&mut BoxShadow, With<ShadowNode>>,
    mut samples_q: Query<&mut BoxShadowSamples, With<Camera2d>>,
) {
    *app_settings = AppSettings::default();

    // Reset the settings panel. It is quicker to do this
    // than to go through every field and setting the fields correctly.
    commands.entity(settings_panel_q.entity()).despawn();
    commands.spawn_scene(settings_panel_scene(&app_settings));

    // Do a full refresh of the box and its shadows.
    for mut node in &mut node_q {
        app_settings.shape.change_node(&mut node);
    }

    for mut box_shadow in &mut box_shadow_q {
        *box_shadow = BoxShadow(generate_shadows(&app_settings));
    }

    for mut samples in &mut samples_q {
        samples.0 = app_settings.samples;
    }
}

fn generate_shadows(app_settings: &AppSettings) -> Vec<ShadowStyle> {
    match app_settings.count {
        1 => vec![make_shadow(
            BLACK.into(),
            app_settings.x_offset,
            app_settings.y_offset,
            app_settings.spread,
            app_settings.blur,
        )],
        2 => vec![
            make_shadow(
                BLUE.into(),
                app_settings.x_offset,
                app_settings.y_offset,
                app_settings.spread,
                app_settings.blur,
            ),
            make_shadow(
                YELLOW.into(),
                -app_settings.x_offset,
                -app_settings.y_offset,
                app_settings.spread,
                app_settings.blur,
            ),
        ],
        3 => vec![
            make_shadow(
                BLUE.into(),
                app_settings.x_offset,
                app_settings.y_offset,
                app_settings.spread,
                app_settings.blur,
            ),
            make_shadow(
                YELLOW.into(),
                -app_settings.x_offset,
                -app_settings.y_offset,
                app_settings.spread,
                app_settings.blur,
            ),
            make_shadow(
                RED.into(),
                app_settings.y_offset,
                -app_settings.x_offset,
                app_settings.spread,
                app_settings.blur,
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
