//! Demonstrates color grading with an interactive adjustment UI.

use std::{
    f32::consts::PI,
    fmt::{self, Formatter},
};

use bevy::{
    camera::Hdr,
    feathers::{
        containers::{pane, pane_body, pane_header},
        controls::{FeathersNumberInput, HardLimit, NumberInputPrecision, NumberInputValue},
        dark_theme::create_dark_theme,
        display::label,
        theme::{ThemeProps, UiTheme},
        tokens::{PANE_BODY_BG, PANE_HEADER_BG, PANE_HEADER_BORDER},
        FeathersPlugins,
    },
    light::CascadeShadowConfigBuilder,
    prelude::*,
    render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection},
    ui_widgets::ValueChange,
};
use std::fmt::Display;

/// The global color grading settings that the user can modify.
///
/// See the documentation of [`ColorGradingGlobal`] for more information about
/// each field here.
#[derive(Clone, Copy, PartialEq, Default)]
enum GlobalColorGradingSetting {
    #[default]
    Exposure,
    Temperature,
    Tint,
    Hue,
}

/// A color grading section that the user can modify the settings of:
/// highlights, midtones, or shadows.
#[derive(Clone, Copy, PartialEq, Default)]
enum SectionColorGradingName {
    #[default]
    Highlights,
    Midtones,
    Shadows,
}

/// The section-specific color grading setting that the user can modify.
///
/// See the documentation of [`ColorGradingSection`] for more information about
/// each field here.
#[derive(Clone, Copy, PartialEq, Default)]
enum SectionColorGradingSetting {
    #[default]
    Saturation,
    Contrast,
    Gamma,
    Gain,
    Lift,
}

/// A color grading settings that the user can modify.
#[derive(Component, Clone, Copy, PartialEq)]
enum ColorGradingSetting {
    /// The global color grading settings. They apply to
    /// the whole image as opposed to specifically to highlights, midtones, or
    /// shadows.
    Global(GlobalColorGradingSetting),

    /// A color grading setting that applies only to highlights, midtones, or shadows.
    Section(SectionColorGradingName, SectionColorGradingSetting),
}

impl Default for ColorGradingSetting {
    fn default() -> Self {
        Self::Global(default())
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(get_example_theme()))
        .add_systems(Startup, setup)
        .add_observer(handle_value_change_number_input)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Create the scene.
    add_basic_scene(&mut commands, &asset_server);

    // Create the root UI element.
    let color_grading = ColorGrading::default();
    add_buttons(&mut commands, &color_grading);

    add_help_text(&mut commands);

    // Spawn the camera.
    add_camera(&mut commands, &asset_server, color_grading);
}

/// Adds all the buttons on the bottom of the scene.
fn add_buttons(commands: &mut Commands, color_grading: &ColorGrading) {
    commands.spawn_scene(bsn! {
        // Spawn the parent node that contains all the buttons.
        Node {
            flex_direction: FlexDirection::Row,
            position_type: PositionType::Absolute,
            column_gap: px(6),
            left: px(12),
            bottom: px(12),
        }
        Children [
            // Create the first pane, which contains the global controls.
            pane_for_global_controls(color_grading),
            // Create the following panes for individual controls.
            pane_for_section(SectionColorGradingName::Highlights, color_grading),
            pane_for_section(SectionColorGradingName::Midtones, color_grading),
            pane_for_section(SectionColorGradingName::Shadows, color_grading),
        ]
    });
}

/// Adds the buttons for the global controls (those that control the scene as a
/// whole as opposed to shadows, midtones, or highlights).
fn pane_for_global_controls(color_grading: &ColorGrading) -> impl Scene {
    let make_button =
        |option| number_input_for_value(ColorGradingSetting::Global(option), color_grading);

    bsn! {
        pane()
        Children [
            // Spawn the label ("Highlights", etc.)
            pane_header()
            Children[
                Node {
                    width: px(120)
                    align_self: AlignSelf::Start,
                }
                Children [
                    label("Global Settings")
                ]
            ],

            // Spawn the buttons
            pane_body()
            Children [
                make_button(GlobalColorGradingSetting::Exposure),
                make_button(GlobalColorGradingSetting::Temperature),
                make_button(GlobalColorGradingSetting::Tint),
                make_button(GlobalColorGradingSetting::Hue),
            ]
        ]
    }
}

/// Adds the buttons that control color grading for individual sections
/// (highlights, midtones, shadows).
fn pane_for_section(section: SectionColorGradingName, color_grading: &ColorGrading) -> impl Scene {
    let make_button = |setting| {
        number_input_for_value(
            ColorGradingSetting::Section(section, setting),
            color_grading,
        )
    };

    bsn! {
        pane()
        Children [
            // Spawn the label ("Highlights", etc.)
            pane_header()
            Children [
                Node {
                    width: px(120),
                    align_self: AlignSelf::Start,
                }
                Children [
                    label(section.to_string())
                ],
            ],

            // Spawn the buttons.
            pane_body()
            Children[
                make_button(SectionColorGradingSetting::Saturation),
                make_button(SectionColorGradingSetting::Contrast),
                make_button(SectionColorGradingSetting::Gamma),
                make_button(SectionColorGradingSetting::Gain),
                make_button(SectionColorGradingSetting::Lift),
            ]
        ]
    }
}

/// Adds a feathers number input that controls one of the color grading values.
fn number_input_for_value(
    setting: ColorGradingSetting,
    color_grading: &ColorGrading,
) -> impl Scene {
    let setting_label = match setting {
        ColorGradingSetting::Global(setting) => setting.to_string(),
        ColorGradingSetting::Section(_, setting) => setting.to_string(),
    };

    bsn! {
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        Children [
            Node {
                width: px(120),
            }
            Children[
                label(setting_label)
            ],

            Node {
                align_items: AlignItems::Center,
                width: px(50),
            }
            @FeathersNumberInput
            template_value(NumberInputValue::F32(setting.get(color_grading)))
            template_value(setting)
            NumberInputPrecision(2)
            HardLimit::f32(0. ..10.)
        ]
    }
}

fn add_camera(commands: &mut Commands, asset_server: &AssetServer, color_grading: ColorGrading) {
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        color_grading,
        DistanceFog {
            color: Color::srgb_u8(43, 44, 47),
            falloff: FogFalloff::Linear {
                start: 1.0,
                end: 8.0,
            },
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2000.0,
            ..default()
        },
    ));
}

fn add_basic_scene(commands: &mut Commands, asset_server: &AssetServer) {
    // Spawn the main scene.
    commands.spawn(WorldAssetRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("models/TonemappingTest/TonemappingTest.gltf"),
    )));

    // Spawn the flight helmet.
    commands.spawn((
        WorldAssetRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf")),
        ),
        Transform::from_xyz(0.5, 0.0, -0.5).with_rotation(Quat::from_rotation_y(-0.15 * PI)),
    ));

    // Spawn the light.
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI * -0.15, PI * -0.15)),
        CascadeShadowConfigBuilder {
            maximum_distance: 3.0,
            first_cascade_far_bound: 0.9,
            ..default()
        }
        .build(),
    ));
}

/// Observer that handles changes to number inputs.
fn handle_value_change_number_input(
    value_change: On<ValueChange<f32>>,
    mut commands: Commands,
    setting_q: Query<&ColorGradingSetting, With<FeathersNumberInput>>,
    mut color_grading: Single<&mut ColorGrading>,
) {
    if let Ok(setting) = setting_q.get(value_change.source) {
        setting.set(&mut color_grading, value_change.value);

        commands
            .entity(value_change.source)
            .insert(NumberInputValue::F32(value_change.value));
    }
}

impl Display for GlobalColorGradingSetting {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match *self {
            GlobalColorGradingSetting::Exposure => "Exposure",
            GlobalColorGradingSetting::Temperature => "Temperature",
            GlobalColorGradingSetting::Tint => "Tint",
            GlobalColorGradingSetting::Hue => "Hue",
        };
        f.write_str(name)
    }
}

impl Display for SectionColorGradingName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match *self {
            SectionColorGradingName::Highlights => "Highlights",
            SectionColorGradingName::Midtones => "Midtones",
            SectionColorGradingName::Shadows => "Shadows",
        };
        f.write_str(name)
    }
}

impl Display for SectionColorGradingSetting {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match *self {
            SectionColorGradingSetting::Saturation => "Saturation",
            SectionColorGradingSetting::Contrast => "Contrast",
            SectionColorGradingSetting::Gamma => "Gamma",
            SectionColorGradingSetting::Gain => "Gain",
            SectionColorGradingSetting::Lift => "Lift",
        };
        f.write_str(name)
    }
}

impl Display for ColorGradingSetting {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ColorGradingSetting::Global(option) => write!(f, "\"{option}\""),
            ColorGradingSetting::Section(section, option) => {
                write!(f, "\"{option}\" for \"{section}\"")
            }
        }
    }
}

impl SectionColorGradingSetting {
    /// Returns the appropriate value in the given color grading section.
    fn get(&self, section: &ColorGradingSection) -> f32 {
        match *self {
            SectionColorGradingSetting::Saturation => section.saturation,
            SectionColorGradingSetting::Contrast => section.contrast,
            SectionColorGradingSetting::Gamma => section.gamma,
            SectionColorGradingSetting::Gain => section.gain,
            SectionColorGradingSetting::Lift => section.lift,
        }
    }

    /// Sets the appropriate value in the given set of color grading values.
    fn set(&self, section: &mut ColorGradingSection, value: f32) {
        match *self {
            SectionColorGradingSetting::Saturation => section.saturation = value,
            SectionColorGradingSetting::Contrast => section.contrast = value,
            SectionColorGradingSetting::Gamma => section.gamma = value,
            SectionColorGradingSetting::Gain => section.gain = value,
            SectionColorGradingSetting::Lift => section.lift = value,
        }
    }
}

impl GlobalColorGradingSetting {
    /// Returns the appropriate value in the given set of global color grading
    /// values.
    fn get(&self, global: &ColorGradingGlobal) -> f32 {
        match *self {
            GlobalColorGradingSetting::Exposure => global.exposure,
            GlobalColorGradingSetting::Temperature => global.temperature,
            GlobalColorGradingSetting::Tint => global.tint,
            GlobalColorGradingSetting::Hue => global.hue,
        }
    }

    /// Sets the appropriate value in the given set of global color grading
    /// values.
    fn set(&self, global: &mut ColorGradingGlobal, value: f32) {
        match *self {
            GlobalColorGradingSetting::Exposure => global.exposure = value,
            GlobalColorGradingSetting::Temperature => global.temperature = value,
            GlobalColorGradingSetting::Tint => global.tint = value,
            GlobalColorGradingSetting::Hue => global.hue = value,
        }
    }
}

impl ColorGradingSetting {
    /// Returns the appropriate value in the given set of color grading values.
    fn get(&self, color_grading: &ColorGrading) -> f32 {
        match self {
            ColorGradingSetting::Global(option) => option.get(&color_grading.global),
            ColorGradingSetting::Section(SectionColorGradingName::Highlights, option) => {
                option.get(&color_grading.highlights)
            }
            ColorGradingSetting::Section(SectionColorGradingName::Midtones, option) => {
                option.get(&color_grading.midtones)
            }
            ColorGradingSetting::Section(SectionColorGradingName::Shadows, option) => {
                option.get(&color_grading.shadows)
            }
        }
    }

    /// Sets the appropriate value in the given set of color grading values.
    fn set(&self, color_grading: &mut ColorGrading, value: f32) {
        match self {
            ColorGradingSetting::Global(option) => {
                option.set(&mut color_grading.global, value);
            }
            ColorGradingSetting::Section(SectionColorGradingName::Highlights, option) => {
                option.set(&mut color_grading.highlights, value);
            }
            ColorGradingSetting::Section(SectionColorGradingName::Midtones, option) => {
                option.set(&mut color_grading.midtones, value);
            }
            ColorGradingSetting::Section(SectionColorGradingName::Shadows, option) => {
                option.set(&mut color_grading.shadows, value);
            }
        }
    }
}

/// Get the Feathers Theme for the example. It is a slightly modified dark theme.
fn get_example_theme() -> ThemeProps {
    let mut props = create_dark_theme();

    // Pane background colors are made a little transparent to see the objects behind the setting controls.
    for token in [PANE_HEADER_BG, PANE_HEADER_BORDER, PANE_BODY_BG] {
        if let Some(color) = props.color.get_mut(&token) {
            color.set_alpha(0.9);
        }
    }

    props
}

/// Creates the help text at the top of the screen.
fn add_help_text(commands: &mut Commands) {
    commands.spawn_scene(bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
        }
        Children [
            Text::new("Drag a setting's input value to change the scene.\n\
                        Click into an input field to change values via keyboard.\n\
                        Values must be between 0 and 10.")
        ]
    });
}
