//! Demonstrates percentage-closer soft shadows (PCSS).

use std::{f32::consts::PI, marker::PhantomData};

use bevy::{
    core_pipeline::{
        experimental::taa::{TemporalAntiAliasPlugin, TemporalAntiAliasSettings},
        prepass::{DepthPrepass, MotionVectorPrepass},
        Skybox,
    },
    ecs::system::EntityCommands,
    math::vec3,
    pbr::{CubemapVisibleEntities, ShadowFilteringMethod},
    prelude::*,
    render::{
        camera::TemporalJitter,
        primitives::{CubemapFrusta, Frustum},
        view::VisibleEntities,
    },
};

/// The path to the UI font.
static FONT_PATH: &str = "fonts/FiraMono-Medium.ttf";

/// The size of the soft shadow penumbras when PCSS is enabled.
const SOFT_SHADOW_SIZE: f32 = 10.0;

/// The intensity of the point and spot lights.
const POINT_LIGHT_INTENSITY: f32 = 1_000_000_000.0;

/// The range in meters of the point and spot lights.
const POINT_LIGHT_RANGE: f32 = 110.0;

/// The depth bias for directional and spot lights. This value is set higher
/// than the default to avoid shadow acne.
const DIRECTIONAL_SHADOW_DEPTH_BIAS: f32 = 0.20;

/// The depth bias for point lights. This value is set higher than the default to
/// avoid shadow acne.
///
/// Unfortunately, there is a bit of Peter Panning with this value, because of
/// the distance and angle of the light. This can't be helped in this scene
/// without increasing the shadow map size beyond reasonable limits.
const POINT_SHADOW_DEPTH_BIAS: f32 = 0.35;

/// The near Z value for the shadow map, in meters. This is set higher than the
/// default in order to achieve greater resolution in the shadow map for point
/// and spot lights.
const SHADOW_MAP_NEAR_Z: f32 = 50.0;

/// The current application settings (light type, shadow filter, and the status
/// of PCSS).
#[derive(Resource, Default)]
struct AppStatus {
    /// The type of light presently in the scene: either directional or point.
    light_type: LightType,
    /// The type of shadow filter: Gaussian or temporal.
    shadow_filter: ShadowFilter,
    /// Whether soft shadows are enabled.
    soft_shadows: SoftShadows,
}

/// The type of light presently in the scene: either directional or point.
#[derive(Clone, Copy, Default, PartialEq)]
enum LightType {
    /// A directional light, with a cascaded shadow map.
    #[default]
    Directional,
    /// A point light, with a cube shadow map.
    Point,
    /// A spot light, with a cube shadow map.
    Spot,
}

/// The type of shadow filter.
///
/// Generally, `Gaussian` is preferred when temporal antialiasing isn't in use,
/// while `Temporal` is preferred when TAA is in use. In this example, this
/// setting also turns TAA on and off.
#[derive(Clone, Copy, Default, PartialEq)]
enum ShadowFilter {
    /// The non-temporal Gaussian filter (Castano '13 for directional lights, an
    /// analogous alternative for point and spot lights).
    #[default]
    NonTemporal,
    /// The temporal Gaussian filter (Jimenez '14 for directional lights, an
    /// analogous alternative for point and spot lights).
    Temporal,
}

/// Whether PCSS is enabled or disabled.
#[derive(Clone, Copy, Default, PartialEq)]
enum SoftShadows {
    /// Soft shadows (PCSS) are enabled.
    #[default]
    Enabled,
    /// Soft shadows (PCSS) are disabled.
    Disabled,
}

/// A marker component that we place on all radio `Button`s.
///
/// The type parameter specifies the setting that this button controls: one of
/// `LightType`, `ShadowFilter`, or `SoftShadows`.
#[derive(Component, Deref, DerefMut)]
struct RadioButton<T>(T);

/// A marker component that we place on all `Text` inside radio buttons.
///
/// The type parameter specifies the setting that this button controls: one of
/// `LightType`, `ShadowFilter`, or `SoftShadows`.
#[derive(Component, Deref, DerefMut)]
struct RadioButtonText<T>(T);

/// An event that's sent whenever the user changes one of the settings by
/// clicking a radio button.
///
/// The type parameter specifies the setting that was changed: one of
/// `LightType`, `ShadowFilter`, or `SoftShadows`.
#[derive(Event)]
struct RadioButtonChangeEvent<T>(PhantomData<T>);

/// The example application entry point.
fn main() {
    App::new()
        .init_resource::<AppStatus>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Percentage Closer Soft Shadows Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TemporalAntiAliasPlugin)
        .add_event::<RadioButtonChangeEvent<LightType>>()
        .add_event::<RadioButtonChangeEvent<ShadowFilter>>()
        .add_event::<RadioButtonChangeEvent<SoftShadows>>()
        .add_systems(Startup, setup)
        .add_systems(Update, handle_ui_interactions)
        .add_systems(
            Update,
            (
                update_light_type_radio_buttons,
                update_shadow_filter_radio_buttons,
                update_soft_shadow_radio_buttons,
            )
                .after(handle_ui_interactions),
        )
        .add_systems(
            Update,
            (
                handle_light_type_change,
                handle_shadow_filter_change,
                handle_pcss_toggle,
            )
                .after(handle_ui_interactions),
        )
        .run();
}

/// Creates all the objects in the scene.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, app_status: Res<AppStatus>) {
    let font = asset_server.load(FONT_PATH);

    spawn_camera(&mut commands, &asset_server);
    spawn_light(&mut commands, &app_status);
    spawn_gltf_scene(&mut commands, &asset_server);
    spawn_buttons(&mut commands, &font);
}

/// Spawns the camera, with the initial shadow filtering method.
fn spawn_camera(commands: &mut Commands, asset_server: &AssetServer) {
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(-12.912 * 0.7, 4.466 * 0.7, -10.624 * 0.7)
                .with_rotation(Quat::from_euler(
                    EulerRot::YXZ,
                    -134.76 / 180.0 * PI,
                    -0.175,
                    0.0,
                )),
            ..default()
        })
        .insert(ShadowFilteringMethod::Gaussian)
        // `TemporalJitter` is needed for TAA. Note that it does nothing without
        // `TemporalAntiAliasSettings`.
        .insert(TemporalJitter::default())
        // The depth prepass is needed for TAA.
        .insert(DepthPrepass)
        // The motion vector prepass is needed for TAA.
        .insert(MotionVectorPrepass)
        // Add a nice skybox.
        .insert(Skybox {
            image: asset_server.load("environment_maps/sky_skybox.ktx2"),
            brightness: 500.0,
        });
}

/// Spawns the initial light.
fn spawn_light(commands: &mut Commands, app_status: &AppStatus) {
    // Because this light can become a directional light, point light, or spot
    // light depending on the settings, we add the union of the components
    // necessary for this light to behave as all three of those.
    commands
        .spawn(DirectionalLightBundle {
            directional_light: create_directional_light(app_status),
            transform: Transform::from_rotation(Quat::from_array([
                0.6539259,
                -0.34646285,
                0.36505926,
                -0.5648683,
            ]))
            .with_translation(vec3(57.693, 34.334, -6.422)),
            ..default()
        })
        // These two are needed for point lights.
        .insert(CubemapVisibleEntities::default())
        .insert(CubemapFrusta::default())
        // These two are needed for spot lights.
        .insert(VisibleEntities::default())
        .insert(Frustum::default());
}

/// Loads and spawns the glTF palm tree scene.
fn spawn_gltf_scene(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/PalmTree/PalmTree.gltf#Scene0"),
        ..default()
    });
}

/// Spawns all the buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands, font: &Handle<Font>) {
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
            spawn_option_buttons(
                parent,
                "Light Type",
                &[
                    (LightType::Directional, "Directional"),
                    (LightType::Point, "Point"),
                    (LightType::Spot, "Spot"),
                ],
                font,
            );
            spawn_option_buttons(
                parent,
                "Shadow Filter",
                &[
                    (ShadowFilter::Temporal, "Temporal"),
                    (ShadowFilter::NonTemporal, "Non-Temporal"),
                ],
                font,
            );
            spawn_option_buttons(
                parent,
                "Soft Shadows",
                &[(SoftShadows::Enabled, "On"), (SoftShadows::Disabled, "Off")],
                font,
            );
        });
}

/// Spawns the buttons that allow configuration of a setting.
///
/// The user may change the setting to any one of the labeled `options`.
///
/// The type parameter specifies the particular setting: one of `LightType`,
/// `ShadowFilter`, or `SoftShadows`.
fn spawn_option_buttons<T>(
    parent: &mut ChildBuilder,
    title: &str,
    options: &[(T, &str)],
    font: &Handle<Font>,
) where
    T: Clone + Send + Sync + 'static,
{
    // Add the parent node for the row.
    parent
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            spawn_ui_text(parent, title, font, Color::BLACK).insert(Style {
                width: Val::Px(125.0),
                ..default()
            });

            for (option_index, (option_value, option_name)) in options.iter().enumerate() {
                spawn_option_button(
                    parent,
                    option_value,
                    option_name,
                    option_index == 0,
                    option_index == 0,
                    option_index == options.len() - 1,
                    font,
                );
            }
        });
}

/// Spawns a single radio button that allows configuration of a setting.
///
/// The type parameter specifies the particular setting: one of `LightType`,
/// `ShadowFilter`, or `SoftShadows`.
fn spawn_option_button<T>(
    parent: &mut ChildBuilder,
    option_value: &T,
    option_name: &str,
    is_selected: bool,
    is_first: bool,
    is_last: bool,
    font: &Handle<Font>,
) where
    T: Clone + Send + Sync + 'static,
{
    let (bg_color, fg_color) = if is_selected {
        (Color::WHITE, Color::BLACK)
    } else {
        (Color::BLACK, Color::WHITE)
    };

    // Add the button node.
    parent
        .spawn(ButtonBundle {
            style: Style {
                border: UiRect::all(Val::Px(1.0)).with_left(if is_first {
                    Val::Px(1.0)
                } else {
                    Val::Px(0.0)
                }),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                ..default()
            },
            border_color: BorderColor(Color::WHITE),
            border_radius: BorderRadius::ZERO
                .with_left(if is_first { Val::Px(6.0) } else { Val::Px(0.0) })
                .with_right(if is_last { Val::Px(6.0) } else { Val::Px(0.0) }),
            image: UiImage::default().with_color(bg_color),
            ..default()
        })
        .insert(RadioButton(option_value.clone()))
        .with_children(|parent| {
            spawn_ui_text(parent, option_name, font, fg_color)
                .insert(RadioButtonText(option_value.clone()));
        });
}

/// Spawns text for the UI.
///
/// Returns the `EntityCommands`, which allow further customization of the text
/// style.
fn spawn_ui_text<'a>(
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

/// Checks for clicks on the radio buttons and sends `RadioButtonChangeEvent`s
/// as necessary.
fn handle_ui_interactions(
    mut interactions: Query<
        (
            &Interaction,
            AnyOf<(
                &RadioButton<LightType>,
                &RadioButton<ShadowFilter>,
                &RadioButton<SoftShadows>,
            )>,
        ),
        With<Button>,
    >,
    mut light_type_events: EventWriter<RadioButtonChangeEvent<LightType>>,
    mut shadow_filter_events: EventWriter<RadioButtonChangeEvent<ShadowFilter>>,
    mut soft_shadow_events: EventWriter<RadioButtonChangeEvent<SoftShadows>>,
    mut app_status: ResMut<AppStatus>,
) {
    for (interaction, (maybe_light_type, maybe_shadow_filter, maybe_soft_shadows)) in
        interactions.iter_mut()
    {
        // Only handle clicks.
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Check each setting. If the clicked button matched one, then send the
        // appropriate event.
        if let Some(light_type) = maybe_light_type {
            app_status.light_type = **light_type;
            light_type_events.send(RadioButtonChangeEvent(PhantomData));
        }
        if let Some(shadow_filter) = maybe_shadow_filter {
            app_status.shadow_filter = **shadow_filter;
            shadow_filter_events.send(RadioButtonChangeEvent(PhantomData));
        }
        if let Some(soft_shadows) = maybe_soft_shadows {
            app_status.soft_shadows = **soft_shadows;
            soft_shadow_events.send(RadioButtonChangeEvent(PhantomData));
        }
    }
}

/// Updates the style of the radio buttons that select the light type to reflect
/// the light type in use.
fn update_light_type_radio_buttons(
    mut light_type_buttons: Query<(&mut UiImage, &RadioButton<LightType>)>,
    mut light_type_button_texts: Query<(&mut Text, &RadioButtonText<LightType>), Without<UiImage>>,
    app_status: Res<AppStatus>,
) {
    for (mut button_style, button) in light_type_buttons.iter_mut() {
        update_ui_radio_button(&mut button_style, button, app_status.light_type);
    }
    for (mut button_text_style, button_text) in light_type_button_texts.iter_mut() {
        update_ui_radio_button_text(&mut button_text_style, button_text, app_status.light_type);
    }
}

/// Updates the style of the radio buttons that select the shadow filter to
/// reflect which filter is selected.
fn update_shadow_filter_radio_buttons(
    mut shadow_filter_buttons: Query<(&mut UiImage, &RadioButton<ShadowFilter>)>,
    mut shadow_filter_button_texts: Query<
        (&mut Text, &RadioButtonText<ShadowFilter>),
        Without<UiImage>,
    >,
    app_status: Res<AppStatus>,
) {
    for (mut button_style, button) in shadow_filter_buttons.iter_mut() {
        update_ui_radio_button(&mut button_style, button, app_status.shadow_filter);
    }
    for (mut button_text_style, button_text) in shadow_filter_button_texts.iter_mut() {
        update_ui_radio_button_text(
            &mut button_text_style,
            button_text,
            app_status.shadow_filter,
        );
    }
}

/// Updates the style of the radio buttons that enable and disable soft shadows
/// to reflect whether PCSS is enabled.
fn update_soft_shadow_radio_buttons(
    mut soft_shadow_buttons: Query<(&mut UiImage, &RadioButton<SoftShadows>)>,
    mut soft_shadow_button_texts: Query<
        (&mut Text, &RadioButtonText<SoftShadows>),
        Without<UiImage>,
    >,
    app_status: Res<AppStatus>,
) {
    for (mut button_style, button) in soft_shadow_buttons.iter_mut() {
        update_ui_radio_button(&mut button_style, button, app_status.soft_shadows);
    }
    for (mut button_text_style, button_text) in soft_shadow_button_texts.iter_mut() {
        update_ui_radio_button_text(&mut button_text_style, button_text, app_status.soft_shadows);
    }
}

/// Updates the style of the button part of a radio button to reflect its
/// selected status.
fn update_ui_radio_button<T>(image: &mut UiImage, radio_button: &RadioButton<T>, value: T)
where
    T: PartialEq,
{
    *image = UiImage::default().with_color(if value == **radio_button {
        Color::WHITE
    } else {
        Color::BLACK
    });
}

/// Updates the style of the label of a radio button to reflect its selected
/// status.
fn update_ui_radio_button_text<T>(text: &mut Text, radio_button_text: &RadioButtonText<T>, value: T)
where
    T: PartialEq,
{
    let text_color = if value == **radio_button_text {
        Color::BLACK
    } else {
        Color::WHITE
    };

    for section in &mut text.sections {
        section.style.color = text_color;
    }
}

/// Handles requests from the user to change the type of light.
fn handle_light_type_change(
    mut commands: Commands,
    mut lights: Query<Entity, Or<(With<DirectionalLight>, With<PointLight>, With<SpotLight>)>>,
    mut events: EventReader<RadioButtonChangeEvent<LightType>>,
    app_status: Res<AppStatus>,
) {
    for _ in events.read() {
        for light in lights.iter_mut() {
            let mut light_commands = commands.entity(light);
            light_commands
                .remove::<DirectionalLight>()
                .remove::<PointLight>()
                .remove::<SpotLight>();
            match app_status.light_type {
                LightType::Point => {
                    light_commands.insert(create_point_light(&app_status));
                }
                LightType::Spot => {
                    light_commands.insert(create_spot_light(&app_status));
                }
                LightType::Directional => {
                    light_commands.insert(create_directional_light(&app_status));
                }
            }
        }
    }
}

/// Handles requests from the user to change the shadow filter method.
///
/// This system is also responsible for enabling and disabling TAA as
/// appropriate.
fn handle_shadow_filter_change(
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut ShadowFilteringMethod)>,
    mut events: EventReader<RadioButtonChangeEvent<ShadowFilter>>,
    app_status: Res<AppStatus>,
) {
    for _ in events.read() {
        for (camera, mut shadow_filtering_method) in cameras.iter_mut() {
            match app_status.shadow_filter {
                ShadowFilter::NonTemporal => {
                    *shadow_filtering_method = ShadowFilteringMethod::Gaussian;
                    commands
                        .entity(camera)
                        .remove::<TemporalAntiAliasSettings>();
                }
                ShadowFilter::Temporal => {
                    *shadow_filtering_method = ShadowFilteringMethod::Temporal;
                    commands
                        .entity(camera)
                        .insert(TemporalAntiAliasSettings::default());
                }
            }
        }
    }
}

/// Handles requests from the user to toggle soft shadows on and off.
fn handle_pcss_toggle(
    mut lights: Query<AnyOf<(&mut DirectionalLight, &mut PointLight, &mut SpotLight)>>,
    mut events: EventReader<RadioButtonChangeEvent<SoftShadows>>,
    app_status: Res<AppStatus>,
) {
    for _ in events.read() {
        // Recreating the lights is the simplest way to toggle soft shadows.
        for (directional_light, point_light, spot_light) in lights.iter_mut() {
            if let Some(mut directional_light) = directional_light {
                *directional_light = create_directional_light(&app_status);
            }
            if let Some(mut point_light) = point_light {
                *point_light = create_point_light(&app_status);
            }
            if let Some(mut spot_light) = spot_light {
                *spot_light = create_spot_light(&app_status);
            }
        }
    }
}

/// Creates the [`DirectionalLight`] component with the appropriate settings.
fn create_directional_light(app_status: &AppStatus) -> DirectionalLight {
    DirectionalLight {
        shadows_enabled: true,
        soft_shadow_size: match app_status.soft_shadows {
            SoftShadows::Enabled => Some(SOFT_SHADOW_SIZE),
            SoftShadows::Disabled => None,
        },
        shadow_depth_bias: DIRECTIONAL_SHADOW_DEPTH_BIAS,
        ..default()
    }
}

/// Creates the [`PointLight`] component with the appropriate settings.
fn create_point_light(app_status: &AppStatus) -> PointLight {
    PointLight {
        intensity: POINT_LIGHT_INTENSITY,
        range: POINT_LIGHT_RANGE,
        shadows_enabled: true,
        soft_shadow_size: match app_status.soft_shadows {
            SoftShadows::Enabled => Some(SOFT_SHADOW_SIZE),
            SoftShadows::Disabled => None,
        },
        shadow_depth_bias: POINT_SHADOW_DEPTH_BIAS,
        shadow_map_near_z: SHADOW_MAP_NEAR_Z,
        ..default()
    }
}

/// Creates the [`SpotLight`] component with the appropriate settings.
fn create_spot_light(app_status: &AppStatus) -> SpotLight {
    SpotLight {
        intensity: POINT_LIGHT_INTENSITY,
        range: POINT_LIGHT_RANGE,
        radius: 0.0,
        shadows_enabled: true,
        soft_shadow_size: match app_status.soft_shadows {
            SoftShadows::Enabled => Some(SOFT_SHADOW_SIZE),
            SoftShadows::Disabled => None,
        },
        shadow_depth_bias: DIRECTIONAL_SHADOW_DEPTH_BIAS,
        shadow_map_near_z: SHADOW_MAP_NEAR_Z,
        ..default()
    }
}
