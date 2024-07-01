//! Demonstrates color grading with an interactive adjustment UI.

use std::{
    f32::consts::PI,
    fmt::{self, Formatter},
};

use bevy::{
    ecs::system::EntityCommands,
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
    render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection},
};
use std::fmt::Display;

static FONT_PATH: &str = "fonts/FiraMono-Medium.ttf";

/// How quickly the value changes per frame.
const OPTION_ADJUSTMENT_SPEED: f32 = 0.003;

/// The color grading section that the user has selected: highlights, midtones,
/// or shadows.
#[derive(Clone, Copy, PartialEq)]
enum SelectedColorGradingSection {
    Highlights,
    Midtones,
    Shadows,
}

/// The global option that the user has selected.
///
/// See the documentation of [`ColorGradingGlobal`] for more information about
/// each field here.
#[derive(Clone, Copy, PartialEq, Default)]
enum SelectedGlobalColorGradingOption {
    #[default]
    Exposure,
    Temperature,
    Tint,
    Hue,
}

/// The section-specific option that the user has selected.
///
/// See the documentation of [`ColorGradingSection`] for more information about
/// each field here.
#[derive(Clone, Copy, PartialEq)]
enum SelectedSectionColorGradingOption {
    Saturation,
    Contrast,
    Gamma,
    Gain,
    Lift,
}

/// The color grading option that the user has selected.
#[derive(Clone, Copy, PartialEq, Resource)]
enum SelectedColorGradingOption {
    /// The user has selected a global color grading option: one that applies to
    /// the whole image as opposed to specifically to highlights, midtones, or
    /// shadows.
    Global(SelectedGlobalColorGradingOption),

    /// The user has selected a color grading option that applies only to
    /// highlights, midtones, or shadows.
    Section(
        SelectedColorGradingSection,
        SelectedSectionColorGradingOption,
    ),
}

impl Default for SelectedColorGradingOption {
    fn default() -> Self {
        Self::Global(default())
    }
}

/// Buttons consist of three parts: the button itself, a label child, and a
/// value child. This specifies one of the three entities.
#[derive(Clone, Copy, PartialEq, Component)]
enum ColorGradingOptionWidgetType {
    /// The parent button.
    Button,
    /// The label of the button.
    Label,
    /// The numerical value that the button displays.
    Value,
}

#[derive(Clone, Copy, Component)]
struct ColorGradingOptionWidget {
    widget_type: ColorGradingOptionWidgetType,
    option: SelectedColorGradingOption,
}

/// A marker component for the help text at the top left of the screen.
#[derive(Clone, Copy, Component)]
struct HelpText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SelectedColorGradingOption>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_button_presses,
                adjust_color_grading_option,
                update_ui_state,
            )
                .chain(),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    currently_selected_option: Res<SelectedColorGradingOption>,
    asset_server: Res<AssetServer>,
) {
    // Create the scene.
    add_basic_scene(&mut commands, &asset_server);

    // Create the root UI element.
    let font = asset_server.load(FONT_PATH);
    let color_grading = ColorGrading::default();
    add_buttons(&mut commands, &font, &color_grading);

    // Spawn help text.
    add_help_text(&mut commands, &font, &currently_selected_option);

    // Spawn the camera.
    add_camera(&mut commands, &asset_server, color_grading);
}

/// Adds all the buttons on the bottom of the scene.
fn add_buttons(commands: &mut Commands, font: &Handle<Font>, color_grading: &ColorGrading) {
    // Spawn the parent node that contains all the buttons.
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                row_gap: Val::Px(6.0),
                left: Val::Px(10.0),
                bottom: Val::Px(10.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // Create the first row, which contains the global controls.
            add_buttons_for_global_controls(parent, color_grading, font);

            // Create the rows for individual controls.
            for section in [
                SelectedColorGradingSection::Highlights,
                SelectedColorGradingSection::Midtones,
                SelectedColorGradingSection::Shadows,
            ] {
                add_buttons_for_section(parent, section, color_grading, font);
            }
        });
}

/// Adds the buttons for the global controls (those that control the scene as a
/// whole as opposed to shadows, midtones, or highlights).
fn add_buttons_for_global_controls(
    parent: &mut ChildBuilder,
    color_grading: &ColorGrading,
    font: &Handle<Font>,
) {
    // Add the parent node for the row.
    parent
        .spawn(NodeBundle {
            style: Style::default(),
            ..default()
        })
        .with_children(|parent| {
            // Add some placeholder text to fill this column.
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(125.0),
                    ..default()
                },
                ..default()
            });

            // Add each global color grading option button.
            for option in [
                SelectedGlobalColorGradingOption::Exposure,
                SelectedGlobalColorGradingOption::Temperature,
                SelectedGlobalColorGradingOption::Tint,
                SelectedGlobalColorGradingOption::Hue,
            ] {
                add_button_for_value(
                    parent,
                    SelectedColorGradingOption::Global(option),
                    color_grading,
                    font,
                );
            }
        });
}

/// Adds the buttons that control color grading for individual sections
/// (highlights, midtones, shadows).
fn add_buttons_for_section(
    parent: &mut ChildBuilder,
    section: SelectedColorGradingSection,
    color_grading: &ColorGrading,
    font: &Handle<Font>,
) {
    // Spawn the row container.
    parent
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // Spawn the label ("Highlights", etc.)
            add_text(parent, &section.to_string(), font, Color::WHITE).insert(Style {
                width: Val::Px(125.0),
                ..default()
            });

            // Spawn the buttons.
            for option in [
                SelectedSectionColorGradingOption::Saturation,
                SelectedSectionColorGradingOption::Contrast,
                SelectedSectionColorGradingOption::Gamma,
                SelectedSectionColorGradingOption::Gain,
                SelectedSectionColorGradingOption::Lift,
            ] {
                add_button_for_value(
                    parent,
                    SelectedColorGradingOption::Section(section, option),
                    color_grading,
                    font,
                );
            }
        });
}

/// Adds a button that controls one of the color grading values.
fn add_button_for_value(
    parent: &mut ChildBuilder,
    option: SelectedColorGradingOption,
    color_grading: &ColorGrading,
    font: &Handle<Font>,
) {
    // Add the button node.
    parent
        .spawn(ButtonBundle {
            style: Style {
                border: UiRect::all(Val::Px(1.0)),
                width: Val::Px(200.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                margin: UiRect::right(Val::Px(12.0)),
                ..default()
            },
            border_color: BorderColor(Color::WHITE),
            border_radius: BorderRadius::MAX,
            background_color: Color::BLACK.into(),
            ..default()
        })
        .insert(ColorGradingOptionWidget {
            widget_type: ColorGradingOptionWidgetType::Button,
            option,
        })
        .with_children(|parent| {
            // Add the button label.
            let label = match option {
                SelectedColorGradingOption::Global(option) => option.to_string(),
                SelectedColorGradingOption::Section(_, option) => option.to_string(),
            };
            add_text(parent, &label, font, Color::WHITE).insert(ColorGradingOptionWidget {
                widget_type: ColorGradingOptionWidgetType::Label,
                option,
            });

            // Add a spacer.
            parent.spawn(NodeBundle {
                style: Style {
                    flex_grow: 1.0,
                    ..default()
                },
                ..default()
            });

            // Add the value text.
            add_text(
                parent,
                &format!("{:.3}", option.get(color_grading)),
                font,
                Color::WHITE,
            )
            .insert(ColorGradingOptionWidget {
                widget_type: ColorGradingOptionWidgetType::Value,
                option,
            });
        });
}

/// Creates the help text at the top of the screen.
fn add_help_text(
    commands: &mut Commands,
    font: &Handle<Font>,
    currently_selected_option: &SelectedColorGradingOption,
) {
    commands
        .spawn(TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                ..default()
            },
            ..TextBundle::from_section(
                create_help_text(currently_selected_option),
                TextStyle {
                    font: font.clone(),
                    ..default()
                },
            )
        })
        .insert(HelpText);
}

/// Adds some text to the scene.
fn add_text<'a>(
    parent: &'a mut ChildBuilder,
    label: &str,
    font: &Handle<Font>,
    color: Color,
) -> EntityCommands<'a> {
    parent.spawn(TextBundle::from_section(
        label,
        TextStyle {
            font: font.clone(),
            font_size: 18.0,
            color,
        },
    ))
}

fn add_camera(commands: &mut Commands, asset_server: &AssetServer, color_grading: ColorGrading) {
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(0.7, 0.7, 1.0)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            color_grading,
            ..default()
        },
        FogSettings {
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
        },
    ));
}

fn add_basic_scene(commands: &mut Commands, asset_server: &AssetServer) {
    // Spawn the main scene.
    commands.spawn(SceneBundle {
        scene: asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("models/TonemappingTest/TonemappingTest.gltf"),
        ),
        ..default()
    });

    // Spawn the flight helmet.
    commands.spawn(SceneBundle {
        scene: asset_server
            .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf")),
        transform: Transform::from_xyz(0.5, 0.0, -0.5)
            .with_rotation(Quat::from_rotation_y(-0.15 * PI)),
        ..default()
    });

    // Spawn the light.
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            PI * -0.15,
            PI * -0.15,
        )),
        cascade_shadow_config: CascadeShadowConfigBuilder {
            maximum_distance: 3.0,
            first_cascade_far_bound: 0.9,
            ..default()
        }
        .into(),
        ..default()
    });
}

impl Display for SelectedGlobalColorGradingOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match *self {
            SelectedGlobalColorGradingOption::Exposure => "Exposure",
            SelectedGlobalColorGradingOption::Temperature => "Temperature",
            SelectedGlobalColorGradingOption::Tint => "Tint",
            SelectedGlobalColorGradingOption::Hue => "Hue",
        };
        f.write_str(name)
    }
}

impl Display for SelectedColorGradingSection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match *self {
            SelectedColorGradingSection::Highlights => "Highlights",
            SelectedColorGradingSection::Midtones => "Midtones",
            SelectedColorGradingSection::Shadows => "Shadows",
        };
        f.write_str(name)
    }
}

impl Display for SelectedSectionColorGradingOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match *self {
            SelectedSectionColorGradingOption::Saturation => "Saturation",
            SelectedSectionColorGradingOption::Contrast => "Contrast",
            SelectedSectionColorGradingOption::Gamma => "Gamma",
            SelectedSectionColorGradingOption::Gain => "Gain",
            SelectedSectionColorGradingOption::Lift => "Lift",
        };
        f.write_str(name)
    }
}

impl Display for SelectedColorGradingOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SelectedColorGradingOption::Global(option) => write!(f, "\"{}\"", option),
            SelectedColorGradingOption::Section(section, option) => {
                write!(f, "\"{}\" for \"{}\"", option, section)
            }
        }
    }
}

impl SelectedSectionColorGradingOption {
    /// Returns the appropriate value in the given color grading section.
    fn get(&self, section: &ColorGradingSection) -> f32 {
        match *self {
            SelectedSectionColorGradingOption::Saturation => section.saturation,
            SelectedSectionColorGradingOption::Contrast => section.contrast,
            SelectedSectionColorGradingOption::Gamma => section.gamma,
            SelectedSectionColorGradingOption::Gain => section.gain,
            SelectedSectionColorGradingOption::Lift => section.lift,
        }
    }

    fn set(&self, section: &mut ColorGradingSection, value: f32) {
        match *self {
            SelectedSectionColorGradingOption::Saturation => section.saturation = value,
            SelectedSectionColorGradingOption::Contrast => section.contrast = value,
            SelectedSectionColorGradingOption::Gamma => section.gamma = value,
            SelectedSectionColorGradingOption::Gain => section.gain = value,
            SelectedSectionColorGradingOption::Lift => section.lift = value,
        }
    }
}

impl SelectedGlobalColorGradingOption {
    /// Returns the appropriate value in the given set of global color grading
    /// values.
    fn get(&self, global: &ColorGradingGlobal) -> f32 {
        match *self {
            SelectedGlobalColorGradingOption::Exposure => global.exposure,
            SelectedGlobalColorGradingOption::Temperature => global.temperature,
            SelectedGlobalColorGradingOption::Tint => global.tint,
            SelectedGlobalColorGradingOption::Hue => global.hue,
        }
    }

    /// Sets the appropriate value in the given set of global color grading
    /// values.
    fn set(&self, global: &mut ColorGradingGlobal, value: f32) {
        match *self {
            SelectedGlobalColorGradingOption::Exposure => global.exposure = value,
            SelectedGlobalColorGradingOption::Temperature => global.temperature = value,
            SelectedGlobalColorGradingOption::Tint => global.tint = value,
            SelectedGlobalColorGradingOption::Hue => global.hue = value,
        }
    }
}

impl SelectedColorGradingOption {
    /// Returns the appropriate value in the given set of color grading values.
    fn get(&self, color_grading: &ColorGrading) -> f32 {
        match self {
            SelectedColorGradingOption::Global(option) => option.get(&color_grading.global),
            SelectedColorGradingOption::Section(
                SelectedColorGradingSection::Highlights,
                option,
            ) => option.get(&color_grading.highlights),
            SelectedColorGradingOption::Section(SelectedColorGradingSection::Midtones, option) => {
                option.get(&color_grading.midtones)
            }
            SelectedColorGradingOption::Section(SelectedColorGradingSection::Shadows, option) => {
                option.get(&color_grading.shadows)
            }
        }
    }

    /// Sets the appropriate value in the given set of color grading values.
    fn set(&self, color_grading: &mut ColorGrading, value: f32) {
        match self {
            SelectedColorGradingOption::Global(option) => {
                option.set(&mut color_grading.global, value);
            }
            SelectedColorGradingOption::Section(
                SelectedColorGradingSection::Highlights,
                option,
            ) => option.set(&mut color_grading.highlights, value),
            SelectedColorGradingOption::Section(SelectedColorGradingSection::Midtones, option) => {
                option.set(&mut color_grading.midtones, value);
            }
            SelectedColorGradingOption::Section(SelectedColorGradingSection::Shadows, option) => {
                option.set(&mut color_grading.shadows, value);
            }
        }
    }
}

/// Handles mouse clicks on the buttons when the user clicks on a new one.
fn handle_button_presses(
    mut interactions: Query<(&Interaction, &ColorGradingOptionWidget), Changed<Interaction>>,
    mut currently_selected_option: ResMut<SelectedColorGradingOption>,
) {
    for (interaction, widget) in interactions.iter_mut() {
        if widget.widget_type == ColorGradingOptionWidgetType::Button
            && *interaction == Interaction::Pressed
        {
            *currently_selected_option = widget.option;
        }
    }
}

/// Updates the state of the UI based on the current state.
fn update_ui_state(
    mut buttons: Query<(
        &mut BackgroundColor,
        &mut BorderColor,
        &ColorGradingOptionWidget,
    )>,
    mut button_text: Query<(&mut Text, &ColorGradingOptionWidget), Without<HelpText>>,
    mut help_text: Query<&mut Text, With<HelpText>>,
    cameras: Query<Ref<ColorGrading>>,
    currently_selected_option: Res<SelectedColorGradingOption>,
) {
    // Exit early if the UI didn't change
    if !currently_selected_option.is_changed() && !cameras.single().is_changed() {
        return;
    }

    // The currently-selected option is drawn with inverted colors.
    for (mut background, mut border_color, widget) in buttons.iter_mut() {
        if *currently_selected_option == widget.option {
            *background = Color::WHITE.into();
            *border_color = Color::BLACK.into();
        } else {
            *background = Color::BLACK.into();
            *border_color = Color::WHITE.into();
        }
    }

    let value_label = cameras.iter().next().map(|color_grading| {
        format!(
            "{:.3}",
            currently_selected_option.get(color_grading.as_ref())
        )
    });

    // Update the buttons.
    for (mut text, widget) in button_text.iter_mut() {
        // Set the text color.

        let color = if *currently_selected_option == widget.option {
            Color::BLACK
        } else {
            Color::WHITE
        };

        for section in &mut text.sections {
            section.style.color = color;
        }

        // Update the displayed value, if this is the currently-selected option.
        if widget.widget_type == ColorGradingOptionWidgetType::Value
            && *currently_selected_option == widget.option
        {
            if let Some(ref value_label) = value_label {
                for section in &mut text.sections {
                    section.value.clone_from(value_label);
                }
            }
        }
    }

    // Update the help text.
    help_text.single_mut().sections[0].value = create_help_text(&currently_selected_option);
}

/// Creates the help text at the top left of the window.
fn create_help_text(currently_selected_option: &SelectedColorGradingOption) -> String {
    format!("Press Left/Right to adjust {}", currently_selected_option)
}

/// Processes keyboard input to change the value of the currently-selected color
/// grading option.
fn adjust_color_grading_option(
    mut cameras: Query<&mut ColorGrading>,
    input: Res<ButtonInput<KeyCode>>,
    currently_selected_option: Res<SelectedColorGradingOption>,
) {
    let mut delta = 0.0;
    if input.pressed(KeyCode::ArrowLeft) {
        delta -= OPTION_ADJUSTMENT_SPEED;
    }
    if input.pressed(KeyCode::ArrowRight) {
        delta += OPTION_ADJUSTMENT_SPEED;
    }

    if delta != 0.0 {
        let mut color_grading = cameras.single_mut();
        let new_value = currently_selected_option.get(&color_grading) + delta;
        currently_selected_option.set(&mut color_grading, new_value);
    }
}
